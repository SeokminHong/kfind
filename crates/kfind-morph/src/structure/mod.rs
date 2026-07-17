use std::ops::Range;
use std::sync::Arc;

use kfind_data::{ComponentPart, ComponentResource, DataFinePos};

use crate::PredicatePos;
use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern, StructuralSignature};

#[derive(Clone, Copy, Debug)]
pub struct BoundedTokenContext<'a> {
    pub previous: Option<&'a str>,
    pub current: &'a str,
    pub next: Option<&'a str>,
}

impl<'a> BoundedTokenContext<'a> {
    #[must_use]
    pub const fn current(current: &'a str) -> Self {
        Self {
            previous: None,
            current,
            next: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructuralEvidence {
    Whole,
    SourceComponent,
    RuntimeComponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintUnavailable {
    InvalidSpans,
    NodeLimit { actual: usize, limit: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintOutcome {
    Supported,
    Contradicted,
    Ambiguous,
    Unavailable(ConstraintUnavailable),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstraintSupport {
    pub pattern_index: usize,
    pub evidence: StructuralEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintDecision {
    pub outcome: ConstraintOutcome,
    pub supported: Vec<ConstraintSupport>,
}

impl ConstraintDecision {
    fn unavailable(reason: ConstraintUnavailable) -> Self {
        Self {
            outcome: ConstraintOutcome::Unavailable(reason),
            supported: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductPolicy {
    RecallFirst,
    Unambiguous,
}

impl ProductPolicy {
    #[must_use]
    pub fn accepts(self, decision: &ConstraintDecision) -> bool {
        !decision.supported.is_empty()
            && match self {
                Self::RecallFirst => matches!(
                    decision.outcome,
                    ConstraintOutcome::Supported | ConstraintOutcome::Ambiguous
                ),
                Self::Unambiguous => decision.outcome == ConstraintOutcome::Supported,
            }
    }
}

#[derive(Debug)]
pub struct ConstraintResolver {
    resource: Arc<ComponentResource>,
    attached_auxiliary: bool,
}

impl ConstraintResolver {
    #[must_use]
    pub fn new(resource: Arc<ComponentResource>) -> Self {
        Self {
            resource,
            attached_auxiliary: false,
        }
    }

    #[must_use]
    pub const fn with_attached_auxiliary(mut self, enabled: bool) -> Self {
        self.attached_auxiliary = enabled;
        self
    }

    #[must_use]
    pub fn resource(&self) -> &ComponentResource {
        &self.resource
    }

    #[must_use]
    pub fn has_whole_modifier(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length == text.len() {
                    matched |= analyses.iter().any(|analysis| {
                        analysis
                            .pos
                            .split('+')
                            .next()
                            .is_some_and(|pos| matches!(pos, "MM" | "MAG" | "MAJ"))
                    });
                }
            });
        matched
    }

    #[must_use]
    pub fn supports_predicate_ending_path(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        self.supports_predicate_ending_path_with_terminal(
            text, anchor_len, pos, node_limit, None, false,
        )
    }

    fn supports_predicate_ending_path_with_terminal(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
        node_limit: usize,
        required_terminal: Option<&str>,
        allow_complete_anchor: bool,
    ) -> bool {
        if anchor_len == 0
            || if allow_complete_anchor {
                anchor_len > text.len()
            } else {
                anchor_len >= text.len()
            }
            || !text.is_char_boundary(anchor_len)
        {
            return false;
        }
        let mut visited = vec![[[false; 2]; 2]; text.len() + 1];
        let mut pending = Vec::new();
        let mut nodes = 0;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length != anchor_len {
                    return;
                }
                for analysis in analyses {
                    nodes += 1;
                    let mut positions = analysis.pos.split('+');
                    let Some(first) = positions.next() else {
                        continue;
                    };
                    let endings = positions.collect::<Vec<_>>();
                    if predicate_pos_matches(first, pos)
                        && endings.iter().all(|ending| ending.starts_with('E'))
                    {
                        let has_ending = !endings.is_empty();
                        let terminal_matches = endings.last().is_some_and(|ending| {
                            required_terminal.is_none_or(|tag| *ending == tag)
                        });
                        if !visited[length][usize::from(has_ending)][usize::from(terminal_matches)]
                        {
                            visited[length][usize::from(has_ending)]
                                [usize::from(terminal_matches)] = true;
                            pending.push((length, has_ending, terminal_matches));
                        }
                    }
                }
            });
        while let Some((start, has_ending, terminal_matches)) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if start == text.len()
                && has_ending
                && required_terminal.is_none_or(|_| terminal_matches)
            {
                return true;
            }
            self.resource
                .common_prefixes(&text.as_bytes()[start..], |length, analyses| {
                    if length == 0 || start + length > text.len() {
                        return;
                    }
                    for analysis in analyses {
                        nodes += 1;
                        let endings = analysis.pos.split('+').collect::<Vec<_>>();
                        if !endings.is_empty()
                            && endings.iter().all(|ending| ending.starts_with('E'))
                        {
                            let end = start + length;
                            let terminal_matches = endings.last().is_some_and(|ending| {
                                required_terminal.is_none_or(|tag| *ending == tag)
                            });
                            if !visited[end][1][usize::from(terminal_matches)] {
                                visited[end][1][usize::from(terminal_matches)] = true;
                                pending.push((end, true, terminal_matches));
                            }
                        }
                    }
                });
        }
        false
    }

    #[must_use]
    pub fn supports_adnominal_dependent_noun_particle_path(
        &self,
        text: &str,
        anchor_len: usize,
        adnominal_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        if anchor_len == 0
            || anchor_len > adnominal_len
            || adnominal_len >= text.len()
            || !text.is_char_boundary(anchor_len)
            || !text.is_char_boundary(adnominal_len)
        {
            return false;
        }
        self.supports_predicate_ending_path_with_terminal(
            &text[..adnominal_len],
            anchor_len,
            pos,
            node_limit,
            Some("ETM"),
            true,
        ) && complete_dependent_noun_particle_suffix(
            &self.resource,
            &text[adnominal_len..],
            node_limit,
        )
    }

    #[must_use]
    pub fn supports_predicate_ending_particle_path(
        &self,
        text: &str,
        anchor_len: usize,
        ending_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        if anchor_len == 0
            || anchor_len >= ending_len
            || ending_len >= text.len()
            || !text.is_char_boundary(anchor_len)
            || !text.is_char_boundary(ending_len)
        {
            return false;
        }
        self.supports_predicate_ending_path(&text[..ending_len], anchor_len, pos, node_limit)
            && complete_suffix(&self.resource, &text[ending_len..], |position| {
                position.starts_with('J')
            })
    }

    #[must_use]
    pub fn supports_ending_suffix_path(&self, text: &str, start: usize, node_limit: usize) -> bool {
        if start >= text.len() || !text.is_char_boundary(start) {
            return false;
        }
        let mut visited = vec![false; text.len() + 1];
        let mut pending = vec![start];
        let mut nodes = 0;
        while let Some(position) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if position == text.len() {
                return true;
            }
            self.resource
                .common_prefixes(&text.as_bytes()[position..], |length, analyses| {
                    if length == 0 || position + length > text.len() {
                        return;
                    }
                    for analysis in analyses {
                        nodes += 1;
                        if analysis.pos.split('+').all(|pos| pos.starts_with('E')) {
                            let end = position + length;
                            if !visited[end] {
                                visited[end] = true;
                                pending.push(end);
                            }
                        }
                    }
                });
        }
        false
    }

    #[must_use]
    pub fn auxiliary_splits(&self, text: &str) -> Vec<usize> {
        let mut splits = Vec::new();
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                for analysis in analyses {
                    let positions = analysis.pos.split('+').collect::<Vec<_>>();
                    let Some(first) = positions.first() else {
                        continue;
                    };
                    if *first != "VX" || !positions[1..].iter().all(|pos| pos.starts_with('E')) {
                        continue;
                    }
                    if length == text.len() || positions.len() == 1 {
                        splits.push(length);
                    }
                }
            });
        splits.sort_unstable();
        splits.dedup();
        splits
    }

    #[must_use]
    pub fn supports_auxiliary_sequence(&self, text: &str, node_limit: usize) -> bool {
        if text.is_empty() {
            return false;
        }
        let mut visited = vec![false; text.len() + 1];
        let mut pending = vec![0];
        let mut nodes = 0;
        while let Some(start) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if start == text.len() && start > 0 {
                return true;
            }
            self.resource
                .common_prefixes(&text.as_bytes()[start..], |length, analyses| {
                    if length == 0 || start + length > text.len() {
                        return;
                    }
                    for analysis in analyses {
                        nodes += 1;
                        let positions = analysis.pos.split('+').collect::<Vec<_>>();
                        let allowed = if start == 0 {
                            positions.first() == Some(&"VX")
                                && positions[1..].iter().all(|pos| pos.starts_with('E'))
                        } else {
                            positions.iter().all(|pos| pos.starts_with('E'))
                        };
                        if allowed {
                            let end = start + length;
                            if !visited[end] {
                                visited[end] = true;
                                pending.push(end);
                            }
                        }
                    }
                });
        }
        false
    }

    #[must_use]
    pub fn whole_predicate_conflicts(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
    ) -> bool {
        self.whole_predicate_conflicts_at(text, 0..anchor_len, pos)
    }

    #[must_use]
    pub fn whole_predicate_conflicts_at(
        &self,
        text: &str,
        anchor: Range<usize>,
        pos: PredicatePos,
    ) -> bool {
        if anchor.is_empty()
            || anchor.end > text.len()
            || !text.is_char_boundary(anchor.start)
            || !text.is_char_boundary(anchor.end)
        {
            return false;
        }
        let mut whole_predicate = false;
        let mut aligned_query_stem = false;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length != text.len() {
                    return;
                }
                for analysis in analyses {
                    let Some(first) = analysis.pos.split('+').next() else {
                        continue;
                    };
                    if !DataFinePos::parse(first).is_some_and(DataFinePos::is_predicate) {
                        continue;
                    }
                    whole_predicate = true;
                    aligned_query_stem |= analysis.components.iter().any(|component| {
                        component.span == anchor && predicate_pos_matches(component.pos, pos)
                    });
                }
            });
        whole_predicate && !aligned_query_stem
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> ConstraintDecision {
        let include_attached_auxiliary = self.attached_auxiliary
            || patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Vx);
        let include_nominal_copula = patterns.iter().any(|pattern| pattern.fine_pos.is_nominal())
            && copula_surface_begins_at(context.current, spans.core.end);
        let prepared = match self.prepare_context_inner(
            context,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
        ) {
            Ok(prepared) => prepared,
            Err(reason) => return ConstraintDecision::unavailable(reason),
        };
        prepared.resolve_candidate(spans, patterns)
    }

    pub fn prepare_context(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        self.prepare_context_inner(context, node_limit, self.attached_auxiliary, false)
    }

    pub fn prepare_context_for_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
        include_nominal_copula: bool,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        self.prepare_context_inner(
            context,
            node_limit,
            self.attached_auxiliary,
            include_nominal_copula,
        )
    }

    fn prepare_context_inner(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        let evidence = TokenEvidence::collect(
            &self.resource,
            context.current,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
        )?;
        let selection = select_structure(&self.resource, context, &evidence);
        Ok(PreparedStructuralContext {
            text: context.current.into(),
            evidence,
            selection,
        })
    }
}

#[derive(Debug)]
pub struct PreparedStructuralContext {
    text: Box<str>,
    evidence: TokenEvidence,
    selection: StructureSelection,
}

impl PreparedStructuralContext {
    #[must_use]
    pub fn has_nominal_copula_host(&self, span: &Range<usize>) -> bool {
        self.evidence.has_nominal_copula_host(span)
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
    ) -> ConstraintDecision {
        if !spans.is_valid_for(&self.text)
            || spans.token != (0..self.text.len())
            || patterns.iter().any(|pattern| !pattern.is_well_formed())
        {
            return ConstraintDecision::unavailable(ConstraintUnavailable::InvalidSpans);
        }
        let raw = collect_pattern_supports(
            &self.evidence,
            &spans,
            patterns,
            self.selection.graph_nominal_host(),
        );
        if raw.is_empty() {
            return ConstraintDecision {
                outcome: ConstraintOutcome::Contradicted,
                supported: Vec::new(),
            };
        }
        let mut supported = raw
            .into_iter()
            .filter(|support| {
                self.selection
                    .accepts(support, &spans, patterns, &self.evidence)
            })
            .collect::<Vec<_>>();
        supported.sort_unstable_by_key(|support| (support.pattern_index, support.evidence as u8));
        supported.dedup();
        if supported.is_empty() {
            return ConstraintDecision {
                outcome: ConstraintOutcome::Contradicted,
                supported,
            };
        }
        let signature_count = distinct_signature_count(&supported, patterns);
        ConstraintDecision {
            outcome: if signature_count > 1 {
                ConstraintOutcome::Ambiguous
            } else {
                ConstraintOutcome::Supported
            },
            supported,
        }
    }
}

fn predicate_pos_matches(actual: &str, expected: PredicatePos) -> bool {
    match expected {
        PredicatePos::Verb => actual == "VV",
        PredicatePos::Adjective => matches!(actual, "VA" | "VCN"),
        PredicatePos::AuxiliaryVerb | PredicatePos::AuxiliaryAdjective => actual == "VX",
        PredicatePos::Copula => actual == "VCP",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Unit {
    span: Range<usize>,
    pos: DataFinePos,
    evidence: StructuralEvidence,
    from_whole_nominal: bool,
}

#[derive(Debug, Default)]
struct TokenEvidence {
    units: Vec<Unit>,
    has_whole_nominal_source_components: bool,
    runtime_spans: Vec<Range<usize>>,
    attached_auxiliary_spans: Box<[Range<usize>]>,
    nominal_copula_hosts: Box<[Range<usize>]>,
    adnominal_ends: Vec<usize>,
    has_complete_path: bool,
    numeric_spans: Box<[Range<usize>]>,
    has_numeral_sequence: bool,
}

impl TokenEvidence {
    fn collect(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
    ) -> Result<Self, ConstraintUnavailable> {
        if text.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            Self::collect_mode::<true>(
                resource,
                text,
                node_limit,
                include_attached_auxiliary,
                include_nominal_copula,
            )
        } else {
            Self::collect_mode::<false>(
                resource,
                text,
                node_limit,
                include_attached_auxiliary,
                include_nominal_copula,
            )
        }
    }

    fn collect_mode<const NUMERIC: bool>(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
    ) -> Result<Self, ConstraintUnavailable> {
        let numeric_end = if NUMERIC {
            text.bytes().take_while(u8::is_ascii_digit).count()
        } else {
            0
        };
        let numeric_unit = if NUMERIC {
            numeric_unit_span(resource, text)
        } else {
            None
        };
        let mut edges = Vec::new();
        for start in text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(text.len()))
        {
            if start == text.len() {
                continue;
            }
            resource.common_prefix_groups(&text.as_bytes()[start..], |length, analyses| {
                if length == 0 || start + length > text.len() {
                    return;
                }
                for analysis in analyses {
                    edges.push(Edge {
                        span: start..start + length,
                        pos: analysis.pos,
                        components: analysis.components,
                    });
                }
            });
            if edges.len() > node_limit {
                return Err(ConstraintUnavailable::NodeLimit {
                    actual: edges.len(),
                    limit: node_limit,
                });
            }
        }
        let mixed_numeral_spans = if NUMERIC
            && numeric_unit.is_none()
            && edges
                .iter()
                .any(|edge| edge.span.start == numeric_end && edge.pos == "NR")
        {
            numeral_sequence_spans(text.len(), numeric_end, &edges, true)
        } else {
            Vec::new()
        };
        let numeric_prefix = numeric_unit
            .as_ref()
            .map(|unit| unit.start)
            .or_else(|| (!mixed_numeral_spans.is_empty()).then_some(numeric_end));
        let forward = numeric_prefix.map_or_else(
            || forward_positions(text.len(), &edges),
            |prefix_end| forward_positions_with_prefix(text.len(), &edges, prefix_end),
        );
        let complete = complete_edges(text.len(), &edges, &forward);
        let has_complete_path = forward[text.len()];
        let attached_auxiliary_spans = if include_attached_auxiliary {
            attached_auxiliary_spans(text.len(), &edges)
        } else {
            Box::default()
        };
        let nominal_copula_hosts = if include_nominal_copula {
            nominal_copula_hosts(text, &edges)
        } else {
            Box::default()
        };
        let mut units = Vec::new();
        let mut has_whole_nominal_source_components = false;
        let mut runtime_spans = Vec::new();
        let mut adnominal_ends = Vec::new();
        for (index, edge) in edges.iter().enumerate() {
            let eligible = if has_complete_path {
                complete[index]
            } else {
                forward[edge.span.start]
            };
            if !eligible {
                continue;
            }
            runtime_spans.push(edge.span.clone());
            if edge.pos.split('+').next_back() == Some("ETM") {
                adnominal_ends.push(edge.span.end);
            }
            let whole_edge = edge.span == (0..text.len());
            let has_one_position = edge.pos.split('+').filter_map(DataFinePos::parse).count() == 1;
            let whole_nominal_analysis = whole_edge
                && has_one_position
                && edge
                    .pos
                    .split('+')
                    .filter_map(DataFinePos::parse)
                    .all(DataFinePos::is_nominal);
            for pos in edge.pos.split('+').filter_map(DataFinePos::parse) {
                units.push(Unit {
                    span: edge.span.clone(),
                    pos,
                    evidence: if whole_edge && has_one_position {
                        StructuralEvidence::Whole
                    } else {
                        StructuralEvidence::RuntimeComponent
                    },
                    from_whole_nominal: false,
                });
            }
            for component in &edge.components {
                if component.pos == "ETM" {
                    adnominal_ends.push(edge.span.start + component.span.end);
                }
                let Some(pos) = DataFinePos::parse(component.pos) else {
                    continue;
                };
                let span =
                    edge.span.start + component.span.start..edge.span.start + component.span.end;
                let from_whole_nominal = whole_nominal_analysis && pos.is_nominal();
                has_whole_nominal_source_components |= from_whole_nominal;
                units.push(Unit {
                    span,
                    pos,
                    evidence: StructuralEvidence::SourceComponent,
                    from_whole_nominal,
                });
            }
        }
        if let Some(unit) = numeric_unit.as_ref() {
            for (index, edge) in edges.iter().enumerate() {
                let eligible = if has_complete_path {
                    complete[index]
                } else {
                    forward[edge.span.start]
                };
                if !eligible {
                    continue;
                }
                if edge.span == *unit && edge.pos.split('+').any(|pos| pos == "NNBC") {
                    units.push(Unit {
                        span: unit.clone(),
                        pos: DataFinePos::Nnb,
                        evidence: if edge.span == (0..text.len()) {
                            StructuralEvidence::Whole
                        } else {
                            StructuralEvidence::RuntimeComponent
                        },
                        from_whole_nominal: false,
                    });
                }
                for component in edge.components.iter().filter(|part| {
                    part.pos == "NNBC"
                        && edge.span.start + part.span.start == unit.start
                        && edge.span.start + part.span.end == unit.end
                }) {
                    units.push(Unit {
                        span: edge.span.start + component.span.start
                            ..edge.span.start + component.span.end,
                        pos: DataFinePos::Nnb,
                        evidence: StructuralEvidence::SourceComponent,
                        from_whole_nominal: false,
                    });
                }
            }
        }
        let (numeric_spans, has_numeral_sequence) = if let Some(unit) = numeric_unit {
            (vec![unit].into_boxed_slice(), false)
        } else if !mixed_numeral_spans.is_empty() {
            (mixed_numeral_spans.into_boxed_slice(), true)
        } else if !NUMERIC
            && edges
                .iter()
                .take_while(|edge| edge.span.start == 0)
                .any(|edge| edge.pos == "NR")
        {
            let spans = hangul_numeral_spans(text.len(), &edges);
            let has_numeral_sequence = !spans.is_empty();
            (spans.into_boxed_slice(), has_numeral_sequence)
        } else {
            (Vec::new().into_boxed_slice(), false)
        };
        units.sort_unstable_by_key(|unit| {
            (
                unit.span.start,
                unit.span.end,
                unit.pos,
                unit.evidence as u8,
                !unit.from_whole_nominal,
            )
        });
        units.dedup_by(|current, previous| {
            let same_unit = current.span == previous.span
                && current.pos == previous.pos
                && current.evidence == previous.evidence;
            if same_unit {
                let from_whole_nominal = current.from_whole_nominal || previous.from_whole_nominal;
                current.from_whole_nominal = from_whole_nominal;
                previous.from_whole_nominal = from_whole_nominal;
            }
            same_unit
        });
        runtime_spans.sort_unstable_by_key(|span| (span.start, span.end));
        runtime_spans.dedup();
        adnominal_ends.sort_unstable();
        adnominal_ends.dedup();
        Ok(Self {
            units,
            has_whole_nominal_source_components,
            runtime_spans,
            attached_auxiliary_spans,
            nominal_copula_hosts,
            adnominal_ends,
            has_complete_path,
            numeric_spans,
            has_numeral_sequence,
        })
    }

    fn has_whole(&self, pos: DataFinePos) -> bool {
        self.units
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos == pos)
    }

    fn has_whole_nominal_source_component(&self, span: &Range<usize>, pos: DataFinePos) -> bool {
        self.units.iter().any(|unit| {
            unit.from_whole_nominal
                && unit.span == *span
                && unit.pos == pos
                && unit.evidence == StructuralEvidence::SourceComponent
        })
    }

    fn has_predicate_ending_at(&self, end: usize) -> bool {
        self.units
            .iter()
            .any(|unit| unit.span.end == end && unit.pos.is_predicate())
    }

    fn has_adnominal_ending_at(&self, end: usize) -> bool {
        self.adnominal_ends.binary_search(&end).is_ok()
    }

    fn has_nominal_copula_host(&self, span: &Range<usize>) -> bool {
        self.nominal_copula_hosts
            .binary_search_by_key(&(span.start, span.end), |host| (host.start, host.end))
            .is_ok()
    }
}

#[derive(Debug)]
struct Edge<'a> {
    span: Range<usize>,
    pos: &'a str,
    components: Vec<ComponentPart<'a>>,
}

fn attached_auxiliary_spans(text_len: usize, edges: &[Edge<'_>]) -> Box<[Range<usize>]> {
    let mut predicate_path = vec![false; text_len + 1];
    let mut connective_boundary = vec![false; text_len + 1];
    for edge in edges {
        let ends_in_connective = if edge.span.start == 0 {
            predicate_path_ends_in_connective(edge.pos)
        } else if predicate_path[edge.span.start] {
            ending_path_ends_in_connective(edge.pos)
        } else {
            None
        };
        if let Some(ends_in_connective) = ends_in_connective {
            predicate_path[edge.span.end] = true;
            if ends_in_connective {
                connective_boundary[edge.span.end] = true;
            }
        }
    }

    let mut ending_suffix = vec![false; text_len + 1];
    ending_suffix[text_len] = true;
    for edge in edges.iter().rev() {
        if ending_suffix[edge.span.end] && edge.pos.split('+').all(|pos| pos.starts_with('E')) {
            ending_suffix[edge.span.start] = true;
        }
    }

    let mut spans = edges
        .iter()
        .filter(|edge| {
            let mut positions = edge.pos.split('+');
            connective_boundary[edge.span.start]
                && positions.next() == Some("VX")
                && positions.all(|pos| pos.starts_with('E'))
                && ending_suffix[edge.span.end]
        })
        .map(|edge| edge.span.clone())
        .collect::<Vec<_>>();
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum CopulaSuffixState {
    Start,
    Copula,
    Ending,
    Particle,
}

const COPULA_SUFFIX_STATE_COUNT: usize = 4;

fn nominal_copula_hosts(text: &str, edges: &[Edge<'_>]) -> Box<[Range<usize>]> {
    if !text
        .char_indices()
        .skip(1)
        .any(|(_, character)| matches!(character, '이' | '입'))
    {
        return Box::default();
    }
    let text_len = text.len();
    let mut nominal_prefix = vec![false; text_len + 1];
    nominal_prefix[0] = true;
    for start in 0..text_len {
        if !nominal_prefix[start] {
            continue;
        }
        for edge in edges.iter().filter(|edge| edge.span.start == start) {
            if edge.pos.split('+').all(nominal_host_pos) {
                nominal_prefix[edge.span.end] = true;
            }
        }
    }

    let mut suffix = vec![[false; COPULA_SUFFIX_STATE_COUNT]; text_len + 1];
    suffix[text_len][CopulaSuffixState::Ending as usize] = true;
    suffix[text_len][CopulaSuffixState::Particle as usize] = true;
    for start in (0..text_len).rev() {
        for edge in edges.iter().filter(|edge| edge.span.start == start) {
            for state in copula_suffix_states() {
                let Some(next) = advance_copula_suffix(state, edge.pos) else {
                    continue;
                };
                suffix[start][state as usize] |= suffix[edge.span.end][next as usize];
            }
        }
    }

    nominal_prefix
        .into_iter()
        .enumerate()
        .skip(1)
        .take(text_len.saturating_sub(1))
        .filter_map(|(end, nominal)| {
            (nominal
                && copula_surface_begins_at(text, end)
                && suffix[end][CopulaSuffixState::Start as usize])
                .then_some(0..end)
        })
        .collect()
}

fn copula_surface_begins_at(text: &str, start: usize) -> bool {
    text.get(start..)
        .is_some_and(|suffix| suffix.starts_with('이') || suffix.starts_with('입'))
}

fn nominal_host_pos(pos: &str) -> bool {
    DataFinePos::parse(pos).is_some_and(DataFinePos::is_nominal)
        || matches!(pos, "XPN" | "XSN" | "XR")
}

const fn copula_suffix_states() -> [CopulaSuffixState; COPULA_SUFFIX_STATE_COUNT] {
    [
        CopulaSuffixState::Start,
        CopulaSuffixState::Copula,
        CopulaSuffixState::Ending,
        CopulaSuffixState::Particle,
    ]
}

fn advance_copula_suffix(
    mut state: CopulaSuffixState,
    positions: &str,
) -> Option<CopulaSuffixState> {
    for pos in positions.split('+') {
        state = match (state, pos) {
            (CopulaSuffixState::Start, "VCP") => CopulaSuffixState::Copula,
            (CopulaSuffixState::Copula | CopulaSuffixState::Ending, pos)
                if pos.starts_with('E') =>
            {
                CopulaSuffixState::Ending
            }
            (CopulaSuffixState::Ending | CopulaSuffixState::Particle, pos)
                if pos.starts_with('J') =>
            {
                CopulaSuffixState::Particle
            }
            _ => return None,
        };
    }
    Some(state)
}

fn predicate_path_ends_in_connective(pos: &str) -> Option<bool> {
    let mut positions = pos.split('+');
    if !matches!(positions.next(), Some("VV" | "VA")) {
        return None;
    }
    let mut last = None;
    for position in positions {
        if !position.starts_with('E') {
            return None;
        }
        last = Some(position);
    }
    Some(last == Some("EC"))
}

fn ending_path_ends_in_connective(pos: &str) -> Option<bool> {
    let mut last = None;
    for position in pos.split('+') {
        if !position.starts_with('E') {
            return None;
        }
        last = Some(position);
    }
    Some(last == Some("EC"))
}

fn forward_positions(text_len: usize, edges: &[Edge<'_>]) -> Vec<bool> {
    let mut forward = vec![false; text_len + 1];
    forward[0] = true;
    for start in 0..text_len {
        if !forward[start] {
            continue;
        }
        for edge in edges.iter().filter(|edge| edge.span.start == start) {
            forward[edge.span.end] = true;
        }
    }
    forward
}

fn forward_positions_with_prefix(
    text_len: usize,
    edges: &[Edge<'_>],
    prefix_end: usize,
) -> Vec<bool> {
    let mut forward = vec![false; text_len + 1];
    forward[0] = true;
    forward[prefix_end] = true;
    for start in 0..text_len {
        if !forward[start] {
            continue;
        }
        for edge in edges.iter().filter(|edge| edge.span.start == start) {
            forward[edge.span.end] = true;
        }
    }
    forward
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum NumeralPathState {
    Start,
    OneNumeral,
    ManyNumerals,
    Unit,
    ManyNumeralParticles,
    UnitParticles,
}

const NUMERAL_PATH_STATE_COUNT: usize = 6;

fn numeral_path_transition(state: NumeralPathState, pos: &str) -> Option<NumeralPathState> {
    let particle = pos.split('+').all(|part| part.starts_with('J'));
    match (state, pos) {
        (NumeralPathState::Start, "NR") => Some(NumeralPathState::OneNumeral),
        (NumeralPathState::OneNumeral, "NR") | (NumeralPathState::ManyNumerals, "NR") => {
            Some(NumeralPathState::ManyNumerals)
        }
        (NumeralPathState::OneNumeral, "NNB" | "NNBC")
        | (NumeralPathState::ManyNumerals, "NNB" | "NNBC") => Some(NumeralPathState::Unit),
        (NumeralPathState::ManyNumerals, _) if particle => {
            Some(NumeralPathState::ManyNumeralParticles)
        }
        (NumeralPathState::ManyNumeralParticles, _) if particle => {
            Some(NumeralPathState::ManyNumeralParticles)
        }
        (NumeralPathState::Unit, _) | (NumeralPathState::UnitParticles, _) if particle => {
            Some(NumeralPathState::UnitParticles)
        }
        _ => None,
    }
}

fn numeral_path_state(index: usize) -> NumeralPathState {
    const STATES: [NumeralPathState; NUMERAL_PATH_STATE_COUNT] = [
        NumeralPathState::Start,
        NumeralPathState::OneNumeral,
        NumeralPathState::ManyNumerals,
        NumeralPathState::Unit,
        NumeralPathState::ManyNumeralParticles,
        NumeralPathState::UnitParticles,
    ];
    STATES[index]
}

fn complete_numeral_path_state(state: NumeralPathState, require_unit: bool) -> bool {
    if require_unit {
        matches!(
            state,
            NumeralPathState::Unit | NumeralPathState::UnitParticles
        )
    } else {
        matches!(
            state,
            NumeralPathState::ManyNumerals
                | NumeralPathState::Unit
                | NumeralPathState::ManyNumeralParticles
                | NumeralPathState::UnitParticles
        )
    }
}

fn hangul_numeral_spans(text_len: usize, edges: &[Edge<'_>]) -> Vec<Range<usize>> {
    numeral_sequence_spans(text_len, 0, edges, false)
}

fn numeral_sequence_spans(
    text_len: usize,
    sequence_start: usize,
    edges: &[Edge<'_>],
    require_unit: bool,
) -> Vec<Range<usize>> {
    let mut forward = vec![[false; NUMERAL_PATH_STATE_COUNT]; text_len + 1];
    forward[sequence_start][NumeralPathState::Start as usize] = true;
    for start in sequence_start..text_len {
        for edge in edges.iter().filter(|edge| edge.span.start == start) {
            for state_index in 0..NUMERAL_PATH_STATE_COUNT {
                if !forward[start][state_index] {
                    continue;
                }
                let state = numeral_path_state(state_index);
                if let Some(next) = numeral_path_transition(state, edge.pos) {
                    forward[edge.span.end][next as usize] = true;
                }
            }
        }
    }

    let mut backward = vec![[false; NUMERAL_PATH_STATE_COUNT]; text_len + 1];
    for (state_index, complete) in backward[text_len].iter_mut().enumerate() {
        *complete = complete_numeral_path_state(numeral_path_state(state_index), require_unit);
    }
    for start in (0..text_len).rev() {
        for edge in edges.iter().filter(|edge| edge.span.start == start) {
            for state_index in 0..NUMERAL_PATH_STATE_COUNT {
                let state = numeral_path_state(state_index);
                let Some(next) = numeral_path_transition(state, edge.pos) else {
                    continue;
                };
                backward[start][state_index] |= backward[edge.span.end][next as usize];
            }
        }
    }

    let mut spans = Vec::new();
    for edge in edges.iter().filter(|edge| edge.pos == "NR") {
        let belongs_to_complete_path = (0..NUMERAL_PATH_STATE_COUNT).any(|state_index| {
            if !forward[edge.span.start][state_index] {
                return false;
            }
            let state = numeral_path_state(state_index);
            numeral_path_transition(state, edge.pos)
                .is_some_and(|next| backward[edge.span.end][next as usize])
        });
        if belongs_to_complete_path {
            spans.push(edge.span.clone());
        }
    }
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans
}

fn complete_edges(text_len: usize, edges: &[Edge<'_>], forward: &[bool]) -> Vec<bool> {
    let mut backward = vec![false; text_len + 1];
    backward[text_len] = true;
    for start in (0..text_len).rev() {
        backward[start] = edges
            .iter()
            .filter(|edge| edge.span.start == start)
            .any(|edge| backward[edge.span.end]);
    }
    edges
        .iter()
        .map(|edge| forward[edge.span.start] && backward[edge.span.end])
        .collect()
}

fn collect_pattern_supports(
    evidence: &TokenEvidence,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    graph_nominal_host: Option<&Range<usize>>,
) -> Vec<ConstraintSupport> {
    let mut supports = Vec::new();
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        let support_start = supports.len();
        for unit in &evidence.units {
            if unit.span != spans.core || unit.pos != pattern.fine_pos {
                continue;
            }
            let allowed = match unit.evidence {
                StructuralEvidence::Whole => true,
                StructuralEvidence::SourceComponent => pattern.component_capability.allows_source(),
                StructuralEvidence::RuntimeComponent => {
                    pattern.component_capability.allows_runtime()
                }
            };
            if allowed {
                supports.push(ConstraintSupport {
                    pattern_index,
                    evidence: unit.evidence,
                });
            }
        }
        if supports.len() == support_start && predicate_nominalization(pattern, spans) {
            for unit in evidence
                .units
                .iter()
                .filter(|unit| unit.span == spans.anchor && unit.pos.is_nominal())
            {
                supports.push(ConstraintSupport {
                    pattern_index,
                    evidence: unit.evidence,
                });
            }
        }
        if supports.len() == support_start
            && pattern.component_capability.allows_runtime()
            && (evidence.runtime_spans.contains(&spans.core)
                || (pattern.fine_pos.is_nominal() && evidence.has_nominal_copula_host(&spans.core))
                || (pattern.fine_pos.is_nominal()
                    && graph_nominal_host == Some(&spans.core)
                    && spans.consumed == spans.token
                    && matches!(pattern.continuation, MorphContinuation::NominalParticles))
                || (spans.core == spans.token
                    && matches!(pattern.continuation, MorphContinuation::NominalParticles))
                || (spans.core.start == spans.token.start
                    && spans.consumed == spans.token
                    && matches!(pattern.continuation, MorphContinuation::Predicate { .. }))
                || (spans.consumed == spans.token
                    && matches!(pattern.continuation, MorphContinuation::Predicate { .. })
                    && evidence.has_whole(pattern.fine_pos))
                || (!evidence.has_complete_path
                    && (spans.consumed == spans.token
                        || matches!(pattern.continuation, MorphContinuation::Predicate { .. }))))
        {
            supports.push(ConstraintSupport {
                pattern_index,
                evidence: StructuralEvidence::RuntimeComponent,
            });
        }
    }
    supports
}

#[derive(Clone, Debug)]
enum StructureSelection {
    Whole,
    RepeatedAdverb,
    AdjacentDeterminer,
    NominalSpan {
        selected: Range<usize>,
        allow_components: bool,
        allow_whole_nominal_source_components: bool,
    },
    CopularFrame {
        nominal: Range<usize>,
        copula: Range<usize>,
    },
    DependentNoun,
    NumericUnit {
        unit: Range<usize>,
    },
    NumeralSequence {
        fallback: Box<StructureSelection>,
    },
    RuntimeCompatible {
        graph_nominal_host: Option<Range<usize>>,
    },
}

impl StructureSelection {
    fn graph_nominal_host(&self) -> Option<&Range<usize>> {
        match self {
            Self::RuntimeCompatible { graph_nominal_host } => graph_nominal_host.as_ref(),
            Self::NumeralSequence { fallback } => fallback.graph_nominal_host(),
            _ => None,
        }
    }

    fn accepts(
        &self,
        support: &ConstraintSupport,
        spans: &CandidateSpans,
        patterns: &[QueryMorphPattern],
        evidence: &TokenEvidence,
    ) -> bool {
        let Some(pattern) = patterns.get(support.pattern_index) else {
            return false;
        };
        match self {
            Self::Whole => support.evidence == StructuralEvidence::Whole,
            Self::RepeatedAdverb => {
                support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Mag
            }
            Self::AdjacentDeterminer => {
                (support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Mm)
                    || !matches!(
                        pattern.fine_pos,
                        DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
                    )
            }
            Self::NominalSpan {
                selected,
                allow_components,
                allow_whole_nominal_source_components,
            } => {
                (support.evidence == StructuralEvidence::Whole
                    && spans.core == spans.token
                    && spans.consumed == spans.token)
                    || (pattern.fine_pos.is_nominal()
                        && ((*allow_whole_nominal_source_components
                            && support.evidence == StructuralEvidence::SourceComponent
                            && evidence.has_whole_nominal_source_component(
                                &spans.core,
                                pattern.fine_pos,
                            ))
                            || spans.core == *selected
                            || (spans.core.start == selected.start
                                && spans.consumed.end == selected.end
                                && evidence.units.iter().any(|unit| {
                                    unit.span == (spans.core.end..selected.end)
                                        && unit.pos.is_particle()
                                }))
                            || ((nominal_component_is_supported(
                                *allow_components,
                                support.evidence,
                                &spans.core,
                                selected,
                                evidence,
                                &pattern.lexical_form,
                            ) || proper_noun_dependent_noun_frame(
                                pattern, spans, selected, evidence,
                            )) && spans.core.start >= selected.start
                                && spans.core.end <= selected.end
                                && spans.core != *selected)))
                    || (predicate_nominalization(pattern, spans)
                        && spans.anchor.start >= selected.start
                        && spans.anchor.end <= selected.end
                        && (spans.consumed.end == spans.token.end
                            || spans.anchor.start > selected.start
                            || spans.anchor.end < selected.end
                            || (spans.anchor != spans.token
                                && evidence.units.iter().any(|unit| {
                                    unit.span == spans.token
                                        && unit.evidence == StructuralEvidence::Whole
                                        && unit.pos.is_nominal()
                                }))))
                    || (matches!(pattern.continuation, MorphContinuation::Predicate { .. })
                        && (spans.core.start == selected.start
                            || (pattern.fine_pos == DataFinePos::Vcp
                                && spans.core.start == selected.end
                                && (spans.consumed.end > spans.core.end
                                    || matches!(
                                        pattern.continuation,
                                        MorphContinuation::Predicate {
                                            state: crate::ContinuationState::Terminal,
                                            ..
                                        }
                                    ))))
                        && spans.consumed.end == spans.token.end
                        && runtime_position_is_supported(pattern, spans, evidence))
                    || (matches!(
                        pattern.fine_pos,
                        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm
                    ) && spans.core.start == selected.start
                        && (spans.consumed == spans.core || spans.consumed.end == spans.token.end))
            }
            Self::CopularFrame { nominal, copula } => {
                (spans.core == *nominal && pattern.fine_pos.is_nominal())
                    || (spans.core == *copula && pattern.fine_pos == DataFinePos::Vcp)
            }
            Self::DependentNoun => {
                support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Nnb
            }
            Self::NumericUnit { unit } => {
                matches!(pattern.fine_pos, DataFinePos::Nnb | DataFinePos::Nr)
                    && spans.core == *unit
                    && spans.consumed.end == spans.token.end
            }
            Self::NumeralSequence { fallback } => {
                (pattern.fine_pos == DataFinePos::Nr
                    && evidence.numeric_spans.contains(&spans.core))
                    || fallback.accepts(support, spans, patterns, evidence)
            }
            Self::RuntimeCompatible { graph_nominal_host } => match support.evidence {
                StructuralEvidence::Whole | StructuralEvidence::SourceComponent => true,
                StructuralEvidence::RuntimeComponent => {
                    runtime_position_is_supported(pattern, spans, evidence)
                        && runtime_nominal_component_is_supported(
                            pattern,
                            spans,
                            evidence,
                            graph_nominal_host.as_ref(),
                        )
                }
            },
        }
    }
}

fn nominal_component_is_supported(
    allow_components: bool,
    support: StructuralEvidence,
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
    lexical_form: &str,
) -> bool {
    if allow_components || support == StructuralEvidence::SourceComponent {
        return true;
    }
    support == StructuralEvidence::RuntimeComponent
        && lexical_form.chars().count() > 1
        && nominal_component_is_on_preferred_path(core, selected, evidence)
}

fn nominal_component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    let span_len = selected.len();
    let mut edges = evidence
        .units
        .iter()
        .filter(|unit| {
            unit.pos.is_nominal()
                && unit.span.start >= selected.start
                && unit.span.end <= selected.end
                && unit.span != *selected
                && matches!(
                    unit.evidence,
                    StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
                )
        })
        .map(|unit| {
            (
                unit.span.start - selected.start,
                unit.span.end - selected.start,
                (
                    1_usize,
                    usize::from(unit.evidence != StructuralEvidence::SourceComponent),
                ),
            )
        })
        .collect::<Vec<_>>();
    edges.sort_unstable_by_key(|(start, end, cost)| (*start, *end, *cost));
    edges.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);

    let mut forward = vec![None; span_len + 1];
    forward[0] = Some((0_usize, 0_usize));
    for position in 0..span_len {
        let Some(prefix) = forward[position] else {
            continue;
        };
        for (_, end, cost) in edges.iter().filter(|(start, _, _)| *start == position) {
            update_min_cost(&mut forward[*end], add_cost(prefix, *cost));
        }
    }
    let Some(best) = forward[span_len] else {
        return false;
    };

    let mut backward = vec![None; span_len + 1];
    backward[span_len] = Some((0_usize, 0_usize));
    for position in (1..=span_len).rev() {
        let Some(suffix) = backward[position] else {
            continue;
        };
        for (start, _, cost) in edges.iter().filter(|(_, end, _)| *end == position) {
            update_min_cost(&mut backward[*start], add_cost(*cost, suffix));
        }
    }

    let core_start = core.start - selected.start;
    let core_end = core.end - selected.start;
    edges.iter().any(|(start, end, cost)| {
        if *start != core_start || *end != core_end {
            return false;
        }
        let (Some(prefix), Some(suffix)) = (forward[*start], backward[*end]) else {
            return false;
        };
        add_cost(add_cost(prefix, *cost), suffix) == best
    })
}

fn add_cost(left: (usize, usize), right: (usize, usize)) -> (usize, usize) {
    (left.0 + right.0, left.1 + right.1)
}

fn update_min_cost(current: &mut Option<(usize, usize)>, candidate: (usize, usize)) {
    if current.is_none_or(|value| candidate < value) {
        *current = Some(candidate);
    }
}

fn runtime_nominal_component_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
    graph_nominal_host: Option<&Range<usize>>,
) -> bool {
    let Some(host) = graph_nominal_host else {
        return true;
    };
    if !pattern.fine_pos.is_nominal()
        || spans.core == *host
        || spans.core.start < host.start
        || spans.core.end > host.end
    {
        return true;
    }
    if proper_noun_dependent_noun_frame(pattern, spans, host, evidence) {
        return true;
    }
    if spans.core.start == host.start && pattern.lexical_form.chars().count() > 1 {
        return true;
    }
    pattern.lexical_form.chars().count() > 1
        && nominal_component_is_on_preferred_path(&spans.core, host, evidence)
}

fn proper_noun_dependent_noun_frame(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    host: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos == DataFinePos::Nnb
        && evidence.has_complete_path
        && pattern.lexical_form.chars().count() == 1
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && spans.core.end == host.end
        && spans.consumed.end > spans.core.end
        && evidence.units.iter().any(|unit| {
            unit.pos == DataFinePos::Nnp
                && unit.span.start == host.start
                && unit.span.end == spans.core.start
                && unit.span.len() > spans.core.len()
        })
        && evidence.units.iter().any(|unit| {
            unit.pos.is_particle()
                && unit.span.start == spans.core.end
                && unit.span.end <= spans.consumed.end
        })
}

fn runtime_position_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    let starts_token = spans.core.start == spans.token.start;
    let leading_only = matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm | DataFinePos::Mag
    );
    let predicate = matches!(pattern.continuation, MorphContinuation::Predicate { .. });
    let terminal_predicate_component = matches!(
        pattern.continuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            ..
        }
    ) && (spans.anchor.end > spans.core.end
        || spans.core.len() > pattern.lexical_form.len());
    let whole_predicate_continuation = whole_predicate_continuation(pattern, spans, evidence);
    let copula_nominal_host = copula_has_complete_nominal_host(pattern, spans, evidence);
    let attached_auxiliary = pattern.fine_pos == DataFinePos::Vx
        && evidence.attached_auxiliary_spans.contains(&spans.core);
    let trailing_predicate_subspan = predicate
        && spans.consumed.end != spans.token.end
        && !terminal_predicate_component
        && !whole_predicate_continuation;
    let internal_runtime_predicate = predicate
        && (pattern.fine_pos != DataFinePos::Vcp || evidence.has_whole(DataFinePos::Mag))
        && spans.core.start != spans.token.start
        && spans.consumed == spans.core
        && !attached_auxiliary
        && !predicate_nominalization(pattern, spans);
    let modifier_before_predicate = predicate
        && !copula_nominal_host
        && !attached_auxiliary
        && spans.core.start != spans.token.start
        && evidence.units.iter().any(|unit| {
            unit.span.end == spans.core.start
                && matches!(unit.pos, DataFinePos::Mag | DataFinePos::Maj)
        });
    let exact_component_prefix =
        (matches!(
            pattern.fine_pos,
            DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm
        ) || (matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
            && pattern.lexical_form.chars().count() > 1))
            && starts_token
            && !evidence
                .units
                .iter()
                .any(|unit| unit.evidence == StructuralEvidence::Whole);
    let trailing_exact_subspan = matches!(pattern.continuation, MorphContinuation::Exact)
        && spans.consumed.end != spans.token.end
        && !exact_component_prefix;
    let multi_syllable_nominal_component = matches!(
        pattern.fine_pos,
        DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
    ) && pattern.lexical_form.chars().count() > 1;
    let trailing_nominal_chain =
        matches!(pattern.continuation, MorphContinuation::NominalParticles)
            && spans.consumed.end != spans.token.end
            && !evidence.has_nominal_copula_host(&spans.core)
            && !exact_component_prefix
            && !multi_syllable_nominal_component;
    let nominal_after_predicate = pattern.fine_pos.is_nominal()
        && pattern.lexical_form.chars().count() == 1
        && spans.consumed.end > spans.core.end
        && evidence.has_predicate_ending_at(spans.core.start);
    let glued_dependent_noun =
        pattern.fine_pos == DataFinePos::Nnb && evidence.has_adnominal_ending_at(spans.core.start);
    let terminal_nominal_in_predicate_frame = pattern.fine_pos.is_nominal()
        && pattern.lexical_form.chars().count() == 1
        && spans.core.start > spans.token.start
        && spans.core.end == spans.token.end
        && evidence.units.iter().any(|unit| {
            unit.pos.is_predicate()
                && ((unit.span.start == spans.token.start && unit.span.end <= spans.core.start)
                    || (unit.span.start < spans.core.start && unit.span.end >= spans.core.end))
        })
        && !glued_dependent_noun;

    (!leading_only || starts_token)
        && !trailing_predicate_subspan
        && !internal_runtime_predicate
        && !modifier_before_predicate
        && !trailing_exact_subspan
        && !trailing_nominal_chain
        && !nominal_after_predicate
        && !terminal_nominal_in_predicate_frame
}

fn whole_predicate_continuation(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    evidence.units.iter().any(|unit| {
        unit.span == (spans.core.start..spans.token.end)
            && unit.pos == pattern.fine_pos
            && unit.pos.is_predicate()
    })
}

fn copula_has_complete_nominal_host(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos == DataFinePos::Vcp
        && evidence.has_complete_path
        && !evidence.has_whole(DataFinePos::Mag)
        && spans.core.start > spans.token.start
        && evidence
            .units
            .iter()
            .any(|unit| unit.span == (spans.token.start..spans.core.start) && unit.pos.is_nominal())
}

fn predicate_nominalization(pattern: &QueryMorphPattern, spans: &CandidateSpans) -> bool {
    matches!(
        pattern.continuation,
        MorphContinuation::Predicate {
            nominal_particles: true,
            ..
        }
    ) && spans.anchor != spans.core
}

fn select_structure(
    resource: &ComponentResource,
    context: BoundedTokenContext<'_>,
    evidence: &TokenEvidence,
) -> StructureSelection {
    if (context.previous == Some(context.current) || context.next == Some(context.current))
        && evidence.has_whole(DataFinePos::Mag)
    {
        return StructureSelection::RepeatedAdverb;
    }
    let next_starts_nominal = context.next.is_some_and(|next| {
        let exact_nominal =
            exact_analysis_starts_with_pos(resource, next, |pos| pos.starts_with('N'));
        let exact_competitor =
            exact_analysis_starts_with_pos(resource, next, |pos| !pos.starts_with('N'));
        nominal_particle_host(resource, next).is_some() || (exact_nominal && !exact_competitor)
    });
    let particle_host = nominal_particle_host(resource, context.current);
    if next_starts_nominal
        && context.current.chars().count() == 1
        && evidence.has_whole(DataFinePos::Mm)
    {
        return StructureSelection::AdjacentDeterminer;
    }
    if let Some((nominal, copula)) = copular_frame(resource, context) {
        return StructureSelection::CopularFrame { nominal, copula };
    }
    if evidence.has_whole(DataFinePos::Mag)
        && particle_host.is_none()
        && context.next.is_some_and(|next| {
            exact_analysis_starts_with_pos(resource, next, |pos| pos.starts_with('V'))
        })
        && has_copular_adnominal_split(resource, context.current)
    {
        return StructureSelection::Whole;
    }
    if context.previous.is_some_and(|previous| {
        exact_analysis_ends_with_pos(resource, previous, |pos| pos == "ETM")
            || adnominal_suffix_is_supported(resource, previous)
    }) && has_exact_fine_pos(resource, context.current, |pos| pos == DataFinePos::Nnb)
    {
        return StructureSelection::DependentNoun;
    }
    if let Some(first) = evidence
        .numeric_spans
        .first()
        .filter(|_| !evidence.has_numeral_sequence)
    {
        return StructureSelection::NumericUnit {
            unit: first.clone(),
        };
    }
    let fallback = if let Some(host) = particle_host {
        let allow_components = false;
        let allow_whole_nominal_source_components =
            host != (0..context.current.len()) && evidence.has_whole_nominal_source_components;
        StructureSelection::NominalSpan {
            selected: host,
            allow_components,
            allow_whole_nominal_source_components,
        }
    } else {
        StructureSelection::RuntimeCompatible {
            graph_nominal_host: complete_nominal_particle_host(resource, context.current),
        }
    };
    if evidence.has_numeral_sequence {
        StructureSelection::NumeralSequence {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    }
}

fn numeric_unit_span(resource: &ComponentResource, text: &str) -> Option<Range<usize>> {
    let numeric_end = text.bytes().take_while(u8::is_ascii_digit).count();
    if numeric_end == 0 || numeric_end == text.len() {
        return None;
    }
    text[numeric_end..]
        .char_indices()
        .map(|(offset, character)| numeric_end + offset + character.len_utf8())
        .filter(|&unit_end| {
            has_exact_source_numeric_unit(resource, &text[numeric_end..unit_end])
                && complete_suffix(resource, &text[unit_end..], |pos| pos.starts_with('J'))
        })
        .max()
        .map(|unit_end| numeric_end..unit_end)
}

fn has_exact_source_numeric_unit(resource: &ComponentResource, text: &str) -> bool {
    let mut matched = false;
    resource.common_prefixes(text.as_bytes(), |length, analyses| {
        if length == text.len() {
            matched |= analyses
                .iter()
                .any(|analysis| matches!(analysis.pos, "NNB" | "NNBC" | "NR"));
        }
    });
    matched
}

fn adnominal_suffix_is_supported(resource: &ComponentResource, text: &str) -> bool {
    let surface_shape = text.ends_with("는") || text.ends_with("던");
    surface_shape
        && text.char_indices().map(|(offset, _)| offset).any(|start| {
            has_exact_sequence(resource, &text[start..], &["ETM"])
                || has_exact_sequence(resource, &text[start..], &["EP", "ETM"])
        })
}

fn exact_analysis_starts_with_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Fn(&str) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefixes(text.as_bytes(), |length, analyses| {
        if length == text.len() {
            matched |= analyses
                .iter()
                .any(|analysis| analysis.pos.split('+').next().is_some_and(&accepts));
        }
    });
    matched
}

fn has_copular_adnominal_split(resource: &ComponentResource, current: &str) -> bool {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .any(|split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && has_exact_sequence(resource, &current[split..], &["VCP", "ETM"])
        })
}

fn copular_frame(
    resource: &ComponentResource,
    context: BoundedTokenContext<'_>,
) -> Option<(Range<usize>, Range<usize>)> {
    let previous = context.previous?;
    let next = context.next?;
    if !complete_pos_sequence(resource, previous, &["VCN", "EC"])
        || !starts_with_pos(resource, next, |pos| matches!(pos, "NNB" | "NNBC"))
    {
        return None;
    }
    let split = unique_copular_split(resource, context.current)?;
    Some((0..split, split..context.current.len()))
}

fn unique_copular_split(resource: &ComponentResource, current: &str) -> Option<usize> {
    let mut matches = current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && (has_exact_sequence(resource, &current[split..], &["VCP"])
                    || has_exact_sequence(resource, &current[split..], &["VCP", "ETM"]))
        });
    let split = matches.next()?;
    matches.next().is_none().then_some(split)
}

fn nominal_particle_host(resource: &ComponentResource, current: &str) -> Option<Range<usize>> {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && complete_suffix(resource, &current[split..], |pos| pos.starts_with('J'))
        })
        .max()
        .map(|end| 0..end)
}

fn complete_nominal_particle_host(
    resource: &ComponentResource,
    current: &str,
) -> Option<Range<usize>> {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            complete_nominal_host(resource, &current[..split])
                && complete_suffix(resource, &current[split..], |pos| pos.starts_with('J'))
        })
        .max()
        .map(|end| 0..end)
}

fn complete_nominal_host(resource: &ComponentResource, text: &str) -> bool {
    let mut visited = vec![[false; 2]; text.len() + 1];
    let mut pending = vec![(0, false)];
    while let Some((start, has_nominal)) = pending.pop() {
        if start == text.len() {
            if has_nominal {
                return true;
            }
            continue;
        }
        resource.common_prefixes(&text.as_bytes()[start..], |length, analyses| {
            if length == 0 || start + length > text.len() {
                return;
            }
            for analysis in analyses {
                let mut next_has_nominal = has_nominal;
                let valid = analysis.pos.split('+').all(|pos| {
                    if DataFinePos::parse(pos).is_some_and(DataFinePos::is_nominal) {
                        next_has_nominal = true;
                        true
                    } else {
                        matches!(pos, "XPN" | "XSN" | "XR")
                    }
                });
                let end = start + length;
                let state = usize::from(next_has_nominal);
                if valid && !visited[end][state] {
                    visited[end][state] = true;
                    pending.push((end, next_has_nominal));
                }
            }
        });
    }
    false
}

fn complete_suffix(
    resource: &ComponentResource,
    suffix: &str,
    accepts: impl Copy + Fn(&str) -> bool,
) -> bool {
    if suffix.is_empty() {
        return true;
    }
    let mut next = Vec::new();
    resource.common_prefixes(suffix.as_bytes(), |length, analyses| {
        if length > 0
            && analyses
                .iter()
                .any(|analysis| analysis.pos.split('+').all(accepts))
        {
            next.push(length);
        }
    });
    next.into_iter()
        .any(|length| complete_suffix(resource, &suffix[length..], accepts))
}

fn complete_dependent_noun_particle_suffix(
    resource: &ComponentResource,
    suffix: &str,
    node_limit: usize,
) -> bool {
    let mut visited = vec![[false; 3]; suffix.len() + 1];
    let mut pending = vec![(0, 0_usize)];
    let mut nodes = 0;
    while let Some((start, state)) = pending.pop() {
        if nodes > node_limit {
            return false;
        }
        if start == suffix.len() {
            if state == 2 {
                return true;
            }
            continue;
        }
        resource.common_prefixes(&suffix.as_bytes()[start..], |length, analyses| {
            if length == 0 || start + length > suffix.len() {
                return;
            }
            for analysis in analyses {
                nodes += 1;
                let mut next_state = state;
                let valid = analysis.pos.split('+').all(|position| match next_state {
                    0 if matches!(position, "NNB" | "NNBC") => {
                        next_state = 1;
                        true
                    }
                    1 | 2 if position.starts_with('J') => {
                        next_state = 2;
                        true
                    }
                    _ => false,
                });
                let end = start + length;
                if valid && !visited[end][next_state] {
                    visited[end][next_state] = true;
                    pending.push((end, next_state));
                }
            }
        });
    }
    false
}

fn has_exact_fine_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Fn(DataFinePos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefixes(text.as_bytes(), |length, analyses| {
        if length == text.len() {
            matched |= analyses
                .iter()
                .filter_map(|analysis| DataFinePos::parse(analysis.pos))
                .any(&accepts);
        }
    });
    matched
}

fn has_exact_sequence(resource: &ComponentResource, text: &str, expected: &[&str]) -> bool {
    let mut matched = false;
    resource.common_prefixes(text.as_bytes(), |length, analyses| {
        if length == text.len() {
            matched |= analyses
                .iter()
                .any(|analysis| analysis.pos.split('+').eq(expected.iter().copied()));
        }
    });
    matched
}

fn complete_pos_sequence(resource: &ComponentResource, text: &str, expected: &[&str]) -> bool {
    if text.is_empty() || expected.is_empty() {
        return text.is_empty() && expected.is_empty();
    }
    let mut next = Vec::new();
    resource.common_prefixes(text.as_bytes(), |length, analyses| {
        for analysis in analyses {
            let actual = analysis.pos.split('+').collect::<Vec<_>>();
            if length > 0 && expected.starts_with(&actual) {
                next.push((length, actual.len()));
            }
        }
    });
    next.into_iter().any(|(length, consumed)| {
        complete_pos_sequence(resource, &text[length..], &expected[consumed..])
    })
}

fn exact_analysis_ends_with_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Copy + Fn(&str) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefixes(text.as_bytes(), |length, analyses| {
        if length == text.len() {
            matched |= analyses
                .iter()
                .any(|analysis| analysis.pos.split('+').next_back().is_some_and(accepts));
        }
    });
    matched
}

fn starts_with_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Fn(&str) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefixes(text.as_bytes(), |_, analyses| {
        matched |= analyses
            .iter()
            .any(|analysis| analysis.pos.split('+').next().is_some_and(&accepts));
    });
    matched
}

fn distinct_signature_count(
    supports: &[ConstraintSupport],
    patterns: &[QueryMorphPattern],
) -> usize {
    let mut signatures = Vec::<StructuralSignature<'_>>::new();
    for support in supports {
        let signature = patterns[support.pattern_index].structural_signature();
        if !signatures.contains(&signature) {
            signatures.push(signature);
        }
    }
    signatures.len()
}

#[cfg(test)]
mod tests;
