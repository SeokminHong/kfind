use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use kfind_data::{ComponentPart, ComponentResource, DataFinePos};

use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern, StructuralSignature};
use crate::{PredicatePos, PredicatePosSet};

const NIKL_ATTACHED_NOMINAL_SUFFIXES: &str =
    include_str!("../../../../data/rules/nikl-attached-nominal-suffixes.tsv");

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

    #[must_use]
    pub fn has_exact_predicate_ending_path(&self, text: &str, pos: PredicatePos) -> bool {
        let mut matched = false;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length != text.len() {
                    return;
                }
                matched |= analyses.iter().any(|analysis| {
                    let mut positions = analysis.pos.split('+');
                    let Some(first) = positions.next() else {
                        return false;
                    };
                    let mut has_ending = false;
                    predicate_pos_matches(first, pos)
                        && positions.all(|position| {
                            has_ending = true;
                            position.starts_with('E')
                        })
                        && has_ending
                });
            });
        matched
    }

    #[must_use]
    pub fn has_exact_pronoun_copula_ending_path(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length != text.len() {
                    return;
                }
                matched |= analyses.iter().any(|analysis| {
                    let positions = analysis.pos.split('+').collect::<Vec<_>>();
                    positions.len() >= 3
                        && positions[0] == "NP"
                        && positions[1] == "VCP"
                        && positions[2..].iter().all(|pos| pos.starts_with('E'))
                        && positions
                            .last()
                            .is_some_and(|pos| matches!(*pos, "EC" | "EF"))
                });
            });
        matched
    }

    #[must_use]
    pub fn has_exact_lost_span_copula_ending_path(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length != text.len() {
                    return;
                }
                matched |= analyses.iter().any(|analysis| {
                    let positions = analysis.pos.split('+').collect::<Vec<_>>();
                    let Some(copula_index) = positions.iter().position(|pos| *pos == "VCP") else {
                        return false;
                    };
                    copula_index > 0
                        && positions[..copula_index]
                            .iter()
                            .all(|pos| DataFinePos::parse(pos).is_some_and(DataFinePos::is_nominal))
                        && positions[copula_index + 1..]
                            .iter()
                            .all(|pos| pos.starts_with('E'))
                        && positions
                            .last()
                            .is_some_and(|pos| matches!(*pos, "EC" | "EF"))
                        && !analysis
                            .components
                            .iter()
                            .any(|component| component.pos == "VCP")
                });
            });
        matched
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
    pub fn has_source_aligned_compound_predicate_component(
        &self,
        text: &str,
        anchor: Range<usize>,
        source_positions: PredicatePosSet,
        node_limit: usize,
    ) -> bool {
        if anchor.is_empty()
            || anchor.start == 0
            || anchor.end > text.len()
            || !text.is_char_boundary(anchor.start)
            || !text.is_char_boundary(anchor.end)
        {
            return false;
        }
        TokenEvidence::collect(&self.resource, text, node_limit, false, false, false).is_ok_and(
            |evidence| {
                !evidence.has_whole_modifier()
                    && evidence
                        .compound_predicate_components
                        .iter()
                        .filter(|component| {
                            component.start == anchor.start && component.end == anchor.end
                        })
                        .any(|component| {
                            source_positions
                                .iter()
                                .any(|pos| predicate_pos_matches(component.pos.as_str(), pos))
                        })
            },
        )
    }

    #[must_use]
    pub fn has_attached_auxiliary_whole_path(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefixes(text.as_bytes(), |length, analyses| {
                if length == text.len() {
                    supported |= analyses
                        .iter()
                        .any(|analysis| is_attached_auxiliary_whole_path_raw(analysis.pos));
                }
            });
        supported
    }

    #[must_use]
    pub fn has_complete_nominal_surface(&self, text: &str) -> bool {
        !text.is_empty() && complete_nominal_host(&self.resource, text)
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
        let include_nominal_derivation_predicate = patterns.iter().any(|pattern| {
            pattern.fine_pos.is_nominal()
                && matches!(pattern.continuation, MorphContinuation::NominalParticles)
                && pattern.component_capability.allows_runtime()
        });
        let prepared = match self.prepare_context_inner(
            context,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
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
        self.prepare_context_inner(context, node_limit, self.attached_auxiliary, false, true)
    }

    pub fn prepare_context_for_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        self.prepare_context_inner(
            context,
            node_limit,
            self.attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )
    }

    fn prepare_context_inner(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        let token = self.prepare_token_graph_inner(
            context.current,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )?;
        let selection = select_structure(&self.resource, context, &token.evidence);
        Ok(PreparedStructuralContext {
            token: PreparedTokenGraphStorage::Owned(token),
            selection,
        })
    }

    pub fn prepare_token_graph_for_candidate(
        &self,
        current: &str,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedTokenGraph, ConstraintUnavailable> {
        self.prepare_token_graph_inner(
            current,
            node_limit,
            self.attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )
    }

    pub fn prepare_context_with_token_graph(
        &self,
        context: BoundedTokenContext<'_>,
        token: Arc<PreparedTokenGraph>,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        if token.text.as_ref() != context.current {
            return Err(ConstraintUnavailable::InvalidSpans);
        }
        let selection = select_structure(&self.resource, context, &token.evidence);
        Ok(PreparedStructuralContext {
            token: PreparedTokenGraphStorage::Shared(token),
            selection,
        })
    }

    fn prepare_token_graph_inner(
        &self,
        current: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedTokenGraph, ConstraintUnavailable> {
        let evidence = TokenEvidence::collect(
            &self.resource,
            current,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )?;
        Ok(PreparedTokenGraph {
            text: current.into(),
            evidence,
        })
    }
}

#[derive(Debug)]
pub struct PreparedTokenGraph {
    text: Box<str>,
    evidence: TokenEvidence,
}

impl PreparedTokenGraph {
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.text.len() + self.evidence.memory_usage()
    }
}

#[derive(Debug)]
pub struct PreparedStructuralContext {
    token: PreparedTokenGraphStorage,
    selection: StructureSelection,
}

#[derive(Debug)]
// The inline owned variant preserves the allocation-free one-shot preparation path.
#[allow(clippy::large_enum_variant)]
enum PreparedTokenGraphStorage {
    Owned(PreparedTokenGraph),
    Shared(Arc<PreparedTokenGraph>),
}

impl PreparedTokenGraphStorage {
    fn as_ref(&self) -> &PreparedTokenGraph {
        match self {
            Self::Owned(token) => token,
            Self::Shared(token) => token,
        }
    }
}

impl PreparedStructuralContext {
    #[must_use]
    pub fn has_nominal_copula_host(&self, span: &Range<usize>) -> bool {
        self.token.as_ref().evidence.has_nominal_copula_host(span)
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
    ) -> ConstraintDecision {
        let token = self.token.as_ref();
        if !spans.is_valid_for(&token.text)
            || spans.token != (0..token.text.len())
            || patterns.iter().any(|pattern| !pattern.is_well_formed())
        {
            return ConstraintDecision::unavailable(ConstraintUnavailable::InvalidSpans);
        }
        let raw = collect_pattern_supports(
            &token.evidence,
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
                    .accepts(support, &spans, patterns, &token.text, &token.evidence)
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

#[derive(Debug)]
struct StartGraph<T> {
    items: Vec<T>,
    start_offsets: Box<[usize]>,
}

impl<T> Default for StartGraph<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            start_offsets: Box::default(),
        }
    }
}

impl<T> StartGraph<T> {
    fn from_sorted_by(
        text_len: usize,
        items: Vec<T>,
        start_of: impl Copy + Fn(&T) -> usize,
    ) -> Self {
        debug_assert!(
            items
                .windows(2)
                .all(|pair| start_of(&pair[0]) <= start_of(&pair[1]))
        );
        debug_assert!(items.iter().all(|item| start_of(item) <= text_len));

        let mut start_offsets = Vec::with_capacity(text_len + 2);
        let mut item_index = 0;
        for start in 0..text_len + 2 {
            while items
                .get(item_index)
                .is_some_and(|item| start_of(item) < start)
            {
                item_index += 1;
            }
            start_offsets.push(item_index);
        }
        Self {
            items,
            start_offsets: start_offsets.into_boxed_slice(),
        }
    }

    fn all(&self) -> &[T] {
        &self.items
    }

    fn starting_range(&self, start: usize) -> Range<usize> {
        let Some(next) = start.checked_add(1) else {
            return 0..0;
        };
        let (Some(&range_start), Some(&range_end)) =
            (self.start_offsets.get(start), self.start_offsets.get(next))
        else {
            return 0..0;
        };
        range_start..range_end
    }

    fn starting_at(&self, start: usize) -> &[T] {
        &self.items[self.starting_range(start)]
    }

    fn memory_usage(&self) -> usize {
        self.items.capacity() * std::mem::size_of::<T>()
            + self.start_offsets.len() * std::mem::size_of::<usize>()
    }
}

type UnitGraph = StartGraph<Unit>;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct UnitPathCost {
    units: usize,
    runtime_components: usize,
}

impl UnitPathCost {
    fn append(self, unit: &Unit) -> Self {
        Self {
            units: self.units + 1,
            runtime_components: self.runtime_components
                + usize::from(unit.evidence != StructuralEvidence::SourceComponent),
        }
    }

    fn combine(self, suffix: Self) -> Self {
        Self {
            units: self.units + suffix.units,
            runtime_components: self.runtime_components + suffix.runtime_components,
        }
    }
}

impl StartGraph<Unit> {
    fn minimum_path_len(
        &self,
        span: &Range<usize>,
        accepts: impl Copy + Fn(DataFinePos) -> bool,
    ) -> Option<usize> {
        if span.is_empty() {
            return Some(0);
        }
        let mut costs = vec![None; span.len() + 1];
        costs[0] = Some(0_usize);
        for offset in 0..span.len() {
            let Some(cost) = costs[offset] else {
                continue;
            };
            let start = span.start + offset;
            for unit in self
                .starting_at(start)
                .iter()
                .filter(|unit| unit.span.end <= span.end && accepts(unit.pos))
            {
                let end = unit.span.end - span.start;
                let next = cost + 1;
                if costs[end].is_none_or(|current| next < current) {
                    costs[end] = Some(next);
                }
            }
        }
        costs[span.len()]
    }

    fn contains_on_preferred_path(
        &self,
        core: &Range<usize>,
        selected: &Range<usize>,
        accepts: impl Copy + Fn(&Unit) -> bool,
    ) -> bool {
        if core.start < selected.start
            || core.end > selected.end
            || core.is_empty()
            || selected.is_empty()
        {
            return false;
        }
        let eligible = |unit: &Unit| {
            unit.span.end <= selected.end
                && accepts(unit)
                && matches!(
                    unit.evidence,
                    StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
                )
        };

        let span_len = selected.len();
        let mut prefix_costs = vec![None; span_len + 1];
        prefix_costs[0] = Some(UnitPathCost::default());
        for offset in 0..span_len {
            let Some(prefix) = prefix_costs[offset] else {
                continue;
            };
            for unit in self
                .starting_at(selected.start + offset)
                .iter()
                .filter(|unit| eligible(unit))
            {
                let end = unit.span.end - selected.start;
                let candidate = prefix.append(unit);
                if prefix_costs[end].is_none_or(|current| candidate < current) {
                    prefix_costs[end] = Some(candidate);
                }
            }
        }
        let Some(best) = prefix_costs[span_len] else {
            return false;
        };

        let mut suffix_costs = vec![None; span_len + 1];
        suffix_costs[span_len] = Some(UnitPathCost::default());
        for offset in (0..span_len).rev() {
            for unit in self
                .starting_at(selected.start + offset)
                .iter()
                .filter(|unit| eligible(unit))
            {
                let end = unit.span.end - selected.start;
                let Some(suffix) = suffix_costs[end] else {
                    continue;
                };
                let candidate = UnitPathCost::default().append(unit).combine(suffix);
                if suffix_costs[offset].is_none_or(|current| candidate < current) {
                    suffix_costs[offset] = Some(candidate);
                }
            }
        }

        let core_start = core.start - selected.start;
        let core_end = core.end - selected.start;
        self.starting_at(core.start)
            .iter()
            .filter(|unit| unit.span == *core && eligible(unit))
            .any(|unit| {
                let (Some(prefix), Some(suffix)) =
                    (prefix_costs[core_start], suffix_costs[core_end])
                else {
                    return false;
                };
                prefix.append(unit).combine(suffix) == best
            })
    }
}

#[derive(Clone, Debug)]
struct NumericUnitPath {
    unit: Range<usize>,
    dependent_tail: Option<Range<usize>>,
}

#[derive(Debug, Default)]
struct TokenEvidence {
    units: UnitGraph,
    nominal_particle_hosts: Box<[Range<usize>]>,
    complete_nominal_particle_host: Option<Range<usize>>,
    has_whole_nominal_source_components: bool,
    runtime_spans: Vec<Range<usize>>,
    compound_predicate_components: Box<[PredicateComponent]>,
    attached_auxiliary_spans: Box<[Range<usize>]>,
    nominal_copula_hosts: Box<[Range<usize>]>,
    adnominal_ends: Vec<usize>,
    has_complete_path: bool,
    leading_predicate_spans: Box<[Range<usize>]>,
    runtime_nominal_derivation_spans: Box<[Range<usize>]>,
    nominal_derivation_predicate_prefixes: Box<[Range<usize>]>,
    derivational_suffix_starts: Box<[usize]>,
    adnominal_derivation_suffix_starts: Box<[usize]>,
    numeric_spans: Box<[Range<usize>]>,
    numeric_dependent_tail: Option<Range<usize>>,
    has_numeral_sequence: bool,
}

impl TokenEvidence {
    fn memory_usage(&self) -> usize {
        self.units.memory_usage()
            + self.nominal_particle_hosts.len() * std::mem::size_of::<Range<usize>>()
            + self.runtime_spans.capacity() * std::mem::size_of::<Range<usize>>()
            + self.compound_predicate_components.len() * std::mem::size_of::<PredicateComponent>()
            + self.attached_auxiliary_spans.len() * std::mem::size_of::<Range<usize>>()
            + self.nominal_copula_hosts.len() * std::mem::size_of::<Range<usize>>()
            + self.adnominal_ends.capacity() * std::mem::size_of::<usize>()
            + self.leading_predicate_spans.len() * std::mem::size_of::<Range<usize>>()
            + self.runtime_nominal_derivation_spans.len() * std::mem::size_of::<Range<usize>>()
            + self.nominal_derivation_predicate_prefixes.len() * std::mem::size_of::<Range<usize>>()
            + self.derivational_suffix_starts.len() * std::mem::size_of::<usize>()
            + self.adnominal_derivation_suffix_starts.len() * std::mem::size_of::<usize>()
            + self.numeric_spans.len() * std::mem::size_of::<Range<usize>>()
    }

    fn collect(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<Self, ConstraintUnavailable> {
        if text.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            Self::collect_mode::<true>(
                resource,
                text,
                node_limit,
                include_attached_auxiliary,
                include_nominal_copula,
                include_nominal_derivation_predicate,
            )
        } else {
            Self::collect_mode::<false>(
                resource,
                text,
                node_limit,
                include_attached_auxiliary,
                include_nominal_copula,
                include_nominal_derivation_predicate,
            )
        }
    }

    fn collect_mode<const NUMERIC: bool>(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<Self, ConstraintUnavailable> {
        let numeric_end = if NUMERIC {
            text.bytes().take_while(u8::is_ascii_digit).count()
        } else {
            0
        };
        let numeric_path = if NUMERIC {
            numeric_unit_path(resource, text)
        } else {
            None
        };
        let numeric_unit = numeric_path.as_ref().map(|path| path.unit.clone());
        let graph = EdgeGraph::collect(resource, text, node_limit)?;
        let path_facts = CommonPathFacts::collect(text, &graph);
        let nominal_paths = path_facts.nominal_paths(text);
        let edges = graph.edges();
        let mixed_numeral_spans = if NUMERIC
            && numeric_unit.is_none()
            && graph
                .starting_at(numeric_end)
                .iter()
                .any(|edge| graph.positions(edge) == [StructuralPos::Fine(DataFinePos::Nr)])
        {
            numeral_sequence_spans(text.len(), numeric_end, &graph, true)
        } else {
            Vec::new()
        };
        let numeric_prefix = numeric_unit
            .as_ref()
            .map(|unit| unit.start)
            .or_else(|| (!mixed_numeral_spans.is_empty()).then_some(numeric_end));
        let forward = numeric_prefix.map_or_else(
            || forward_positions(text.len(), &graph),
            |prefix_end| forward_positions_with_prefix(text.len(), &graph, prefix_end),
        );
        let complete = complete_edges(text.len(), &graph, &forward);
        let has_complete_path = forward[text.len()];
        let leading_predicate_spans = leading_predicate_spans(&graph, &path_facts.ending_suffix);
        let compound_predicate_components = compound_predicate_components(
            text.len(),
            &graph,
            &path_facts.predicate_connective_boundaries,
        );
        let runtime_nominal_derivation_spans = runtime_nominal_derivation_spans(&graph, &complete);
        let nominal_derivation_predicate_prefixes = if include_nominal_derivation_predicate {
            nominal_derivation_predicate_prefixes(text.len(), &graph, &path_facts.ending_suffix)
        } else {
            Box::default()
        };
        let derivational_suffix_starts = derivational_suffix_starts(&graph, &complete);
        let adnominal_derivation_suffix_starts =
            adnominal_derivation_suffix_starts(text.len(), &graph, &path_facts.nominal_prefix);
        let attached_auxiliary_spans = if include_attached_auxiliary {
            attached_auxiliary_spans(
                text.len(),
                &graph,
                &path_facts.predicate_connective_boundaries,
                &path_facts.ending_suffix,
            )
        } else {
            Box::default()
        };
        let nominal_copula_hosts = if include_nominal_copula {
            nominal_copula_hosts(text, &graph, &path_facts.nominal_prefix)
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
            let positions = graph.positions(edge);
            if positions.last() == Some(&StructuralPos::Etm) {
                adnominal_ends.push(edge.span.end);
            }
            let whole_edge = edge.span == (0..text.len());
            let has_one_position = positions
                .iter()
                .filter_map(|position| position.fine_pos())
                .count()
                == 1;
            let whole_nominal_analysis = whole_edge
                && has_one_position
                && positions
                    .iter()
                    .filter_map(|position| position.fine_pos())
                    .all(DataFinePos::is_nominal);
            for pos in positions.iter().filter_map(|position| position.fine_pos()) {
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
                if edge.span == *unit && graph.positions(edge).contains(&StructuralPos::Nnbc) {
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
            && graph
                .starting_at(0)
                .iter()
                .any(|edge| graph.positions(edge) == [StructuralPos::Fine(DataFinePos::Nr)])
        {
            let spans = hangul_numeral_spans(text.len(), &graph);
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
            units: UnitGraph::from_sorted_by(text.len(), units, |unit| unit.span.start),
            nominal_particle_hosts: nominal_paths.particle_hosts,
            complete_nominal_particle_host: nominal_paths.complete_particle_host,
            has_whole_nominal_source_components,
            runtime_spans,
            compound_predicate_components,
            attached_auxiliary_spans,
            nominal_copula_hosts,
            adnominal_ends,
            has_complete_path,
            leading_predicate_spans,
            runtime_nominal_derivation_spans,
            nominal_derivation_predicate_prefixes,
            derivational_suffix_starts,
            adnominal_derivation_suffix_starts,
            numeric_spans,
            numeric_dependent_tail: numeric_path.and_then(|path| path.dependent_tail),
            has_numeral_sequence,
        })
    }

    fn has_whole(&self, pos: DataFinePos) -> bool {
        self.units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos == pos)
    }

    fn has_whole_modifier(&self) -> bool {
        [DataFinePos::Mm, DataFinePos::Mag, DataFinePos::Maj]
            .into_iter()
            .any(|pos| self.has_whole(pos))
    }

    fn has_whole_analysis(&self, span: &Range<usize>) -> bool {
        self.units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.span == *span)
    }

    fn has_whole_nominal_source_component(&self, span: &Range<usize>, pos: DataFinePos) -> bool {
        self.units.all().iter().any(|unit| {
            unit.from_whole_nominal
                && unit.span == *span
                && unit.pos == pos
                && unit.evidence == StructuralEvidence::SourceComponent
        })
    }

    fn has_predicate_ending_at(&self, end: usize) -> bool {
        self.units
            .all()
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StructuralPos {
    Fine(DataFinePos),
    Nnbc,
    Ep,
    Ef,
    Ec,
    Etn,
    Etm,
    Xpn,
    Xsn,
    Xsv,
    Xsa,
    Xr,
    OtherEnding,
    OtherParticle,
    OtherPredicate,
    Other,
}

impl StructuralPos {
    fn parse(value: &str) -> Self {
        if let Some(pos) = DataFinePos::parse(value) {
            return Self::Fine(pos);
        }
        match value {
            "NNBC" => Self::Nnbc,
            "EP" => Self::Ep,
            "EF" => Self::Ef,
            "EC" => Self::Ec,
            "ETN" => Self::Etn,
            "ETM" => Self::Etm,
            "XPN" => Self::Xpn,
            "XSN" => Self::Xsn,
            "XSV" => Self::Xsv,
            "XSA" => Self::Xsa,
            "XR" => Self::Xr,
            _ if value.starts_with('E') => Self::OtherEnding,
            _ if value.starts_with('J') => Self::OtherParticle,
            _ if value.starts_with('V') => Self::OtherPredicate,
            _ => Self::Other,
        }
    }

    const fn fine_pos(self) -> Option<DataFinePos> {
        match self {
            Self::Fine(pos) => Some(pos),
            _ => None,
        }
    }

    const fn is_nominal(self) -> bool {
        matches!(self, Self::Fine(pos) if pos.is_nominal())
    }

    const fn is_predicate(self) -> bool {
        matches!(self, Self::Fine(pos) if pos.is_predicate())
    }

    const fn is_predicate_tag(self) -> bool {
        self.is_predicate() || matches!(self, Self::OtherPredicate)
    }

    const fn is_ending(self) -> bool {
        matches!(
            self,
            Self::Ep | Self::Ef | Self::Ec | Self::Etn | Self::Etm | Self::OtherEnding
        )
    }

    const fn is_particle(self) -> bool {
        matches!(self, Self::Fine(pos) if pos.is_particle()) || matches!(self, Self::OtherParticle)
    }
}

#[derive(Debug)]
struct Edge<'a> {
    span: Range<usize>,
    positions: Range<usize>,
    components: Vec<ComponentPart<'a>>,
}

#[derive(Debug)]
struct EdgeGraph<'a> {
    edges: StartGraph<Edge<'a>>,
    positions: Vec<StructuralPos>,
}

struct NominalPathFacts {
    particle_hosts: Box<[Range<usize>]>,
    complete_particle_host: Option<Range<usize>>,
}

struct CommonPathFacts {
    nominal_prefix: Vec<[bool; 2]>,
    ending_suffix: Vec<bool>,
    particle_suffix: Vec<bool>,
    predicate_connective_boundaries: Vec<bool>,
    exact_nominal_end: Vec<bool>,
}

impl CommonPathFacts {
    fn collect(text: &str, graph: &EdgeGraph<'_>) -> Self {
        let mut nominal_prefix = vec![[false; 2]; text.len() + 1];
        nominal_prefix[0][0] = true;
        let mut predicate_path = vec![false; text.len() + 1];
        let mut predicate_connective_boundaries = vec![false; text.len() + 1];
        let mut exact_nominal_end = vec![false; text.len() + 1];
        for start in 0..text.len() {
            for edge in graph.starting_at(start) {
                let positions = graph.positions(edge);
                for has_nominal in [false, true] {
                    if !nominal_prefix[start][usize::from(has_nominal)] {
                        continue;
                    }
                    let mut next_has_nominal = has_nominal;
                    let valid = positions.iter().all(|pos| {
                        if pos.is_nominal() {
                            next_has_nominal = true;
                            true
                        } else {
                            matches!(
                                pos,
                                StructuralPos::Xpn | StructuralPos::Xsn | StructuralPos::Xr
                            )
                        }
                    });
                    if valid {
                        nominal_prefix[edge.span.end][usize::from(next_has_nominal)] = true;
                    }
                }

                if start == 0 {
                    exact_nominal_end[edge.span.end] |=
                        matches!(positions, [pos] if pos.is_nominal());
                }
                let ends_in_connective = if start == 0 {
                    predicate_path_ends_in_connective(positions)
                } else if predicate_path[start] {
                    ending_path_ends_in_connective(positions)
                } else {
                    None
                };
                if let Some(ends_in_connective) = ends_in_connective {
                    if ends_in_connective {
                        predicate_connective_boundaries[edge.span.end] = true;
                    } else {
                        predicate_path[edge.span.end] = true;
                    }
                }
            }
        }

        let mut ending_suffix = vec![false; text.len() + 1];
        let mut particle_suffix = vec![false; text.len() + 1];
        ending_suffix[text.len()] = true;
        particle_suffix[text.len()] = true;
        for start in (0..text.len()).rev() {
            for edge in graph.starting_at(start) {
                if ending_suffix[edge.span.end]
                    && graph.positions(edge).iter().all(|pos| pos.is_ending())
                {
                    ending_suffix[start] = true;
                }
                if particle_suffix[edge.span.end]
                    && graph.positions(edge).iter().all(|pos| pos.is_particle())
                {
                    particle_suffix[start] = true;
                }
            }
        }

        Self {
            nominal_prefix,
            ending_suffix,
            particle_suffix,
            predicate_connective_boundaries,
            exact_nominal_end,
        }
    }

    fn nominal_paths(&self, text: &str) -> NominalPathFacts {
        let mut particle_hosts = Vec::new();
        let mut complete_particle_host = None;
        for split in text.char_indices().map(|(offset, _)| offset).skip(1) {
            if self.exact_nominal_end[split] && self.particle_suffix[split] {
                particle_hosts.push(0..split);
            }
            if self.nominal_prefix[split][1] && self.particle_suffix[split] {
                complete_particle_host = Some(0..split);
            }
        }
        NominalPathFacts {
            particle_hosts: particle_hosts.into_boxed_slice(),
            complete_particle_host,
        }
    }
}

impl<'a> EdgeGraph<'a> {
    fn collect(
        resource: &'a ComponentResource,
        text: &str,
        node_limit: usize,
    ) -> Result<Self, ConstraintUnavailable> {
        let mut edges = Vec::new();
        let mut positions = Vec::new();
        let mut interned_positions = HashMap::<(usize, usize), Range<usize>>::new();
        let position_limit = node_limit.saturating_mul(8);
        let mut position_limit_exceeded = false;
        for start in text.char_indices().map(|(offset, _)| offset) {
            resource.common_prefix_groups(&text.as_bytes()[start..], |length, analyses| {
                if position_limit_exceeded || length == 0 || start + length > text.len() {
                    return;
                }
                for analysis in analyses {
                    let key = (analysis.pos.as_ptr() as usize, analysis.pos.len());
                    let position_range = if let Some(range) = interned_positions.get(&key) {
                        range.clone()
                    } else {
                        let range_start = positions.len();
                        for position in analysis.pos.split('+') {
                            if positions.len() >= position_limit {
                                position_limit_exceeded = true;
                                positions.truncate(range_start);
                                return;
                            }
                            positions.push(StructuralPos::parse(position));
                        }
                        let range = range_start..positions.len();
                        interned_positions.insert(key, range.clone());
                        range
                    };
                    edges.push(Edge {
                        span: start..start + length,
                        positions: position_range,
                        components: analysis.components,
                    });
                }
            });
            if position_limit_exceeded {
                return Err(ConstraintUnavailable::NodeLimit {
                    actual: position_limit.saturating_add(1),
                    limit: position_limit,
                });
            }
            if edges.len() > node_limit {
                return Err(ConstraintUnavailable::NodeLimit {
                    actual: edges.len(),
                    limit: node_limit,
                });
            }
        }

        Ok(Self::from_edges(text.len(), edges, positions))
    }

    fn from_edges(text_len: usize, edges: Vec<Edge<'a>>, positions: Vec<StructuralPos>) -> Self {
        Self {
            edges: StartGraph::from_sorted_by(text_len, edges, |edge| edge.span.start),
            positions,
        }
    }

    #[cfg(test)]
    fn from_raw_edges(text_len: usize, edges: Vec<(Range<usize>, &'a str)>) -> Self {
        let mut positions = Vec::new();
        let edges = edges
            .into_iter()
            .map(|(span, raw_positions)| {
                let start = positions.len();
                positions.extend(raw_positions.split('+').map(StructuralPos::parse));
                Edge {
                    span,
                    positions: start..positions.len(),
                    components: Vec::new(),
                }
            })
            .collect();
        Self::from_edges(text_len, edges, positions)
    }

    fn edges(&self) -> &[Edge<'a>] {
        self.edges.all()
    }

    fn starting_at(&self, start: usize) -> &[Edge<'a>] {
        self.edges.starting_at(start)
    }

    fn starting_range(&self, start: usize) -> Range<usize> {
        self.edges.starting_range(start)
    }

    fn positions(&self, edge: &Edge<'_>) -> &[StructuralPos] {
        &self.positions[edge.positions.clone()]
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PredicateComponent {
    start: usize,
    end: usize,
    pos: DataFinePos,
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum CompoundTailState {
    Predicate,
    Ending,
    Particle,
}

const COMPOUND_TAIL_STATE_COUNT: usize = 3;

fn compound_predicate_components(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    connective_boundary: &[bool],
) -> Box<[PredicateComponent]> {
    let edges = graph.edges();

    let mut suffix = vec![[false; COMPOUND_TAIL_STATE_COUNT]; text_len + 1];
    for state in compound_tail_states() {
        suffix[text_len][state as usize] = true;
    }
    for start in (0..text_len).rev() {
        for edge in graph.starting_at(start) {
            for state in compound_tail_states() {
                let Some(next) = advance_compound_tail_suffix(state, graph.positions(edge)) else {
                    continue;
                };
                suffix[start][state as usize] |= suffix[edge.span.end][next as usize];
            }
        }
    }

    let mut components = Vec::new();
    for edge in graph.starting_at(0) {
        let Some((pos, state)) = compound_tail_inside_analysis(graph.positions(edge)) else {
            continue;
        };
        if !suffix[edge.span.end][state as usize] {
            continue;
        }
        components.extend(
            edge.components
                .iter()
                .filter(|component| DataFinePos::parse(component.pos) == Some(pos))
                .map(|component| PredicateComponent {
                    start: edge.span.start + component.span.start,
                    end: edge.span.start + component.span.end,
                    pos,
                }),
        );
    }
    for edge in edges
        .iter()
        .filter(|edge| connective_boundary[edge.span.start])
    {
        let Some((&first, suffix_positions)) = graph.positions(edge).split_first() else {
            continue;
        };
        let Some(pos) = first.fine_pos().filter(|pos| pos.is_predicate()) else {
            continue;
        };
        let Some(state) =
            advance_compound_tail_suffix(CompoundTailState::Predicate, suffix_positions)
        else {
            continue;
        };
        if !suffix[edge.span.end][state as usize] {
            continue;
        }
        components.push(PredicateComponent {
            start: edge.span.start,
            end: edge.span.end,
            pos,
        });
        components.extend(
            edge.components
                .iter()
                .filter(|component| DataFinePos::parse(component.pos) == Some(pos))
                .map(|component| PredicateComponent {
                    start: edge.span.start + component.span.start,
                    end: edge.span.start + component.span.end,
                    pos,
                }),
        );
    }
    components.sort_unstable();
    components.dedup();
    components.into_boxed_slice()
}

fn compound_tail_inside_analysis(
    positions: &[StructuralPos],
) -> Option<(DataFinePos, CompoundTailState)> {
    let (first, positions) = positions.split_first()?;
    first.fine_pos().filter(|pos| pos.is_predicate())?;
    let mut connective = false;
    for (index, &position) in positions.iter().enumerate() {
        if position.is_ending() {
            connective = position == StructuralPos::Ec;
            continue;
        }
        let pos = position.fine_pos().filter(|pos| pos.is_predicate())?;
        if !connective {
            return None;
        }
        let mut state = CompoundTailState::Predicate;
        for &suffix in &positions[index + 1..] {
            state = advance_compound_tail_position(state, suffix)?;
        }
        return Some((pos, state));
    }
    None
}

const fn compound_tail_states() -> [CompoundTailState; COMPOUND_TAIL_STATE_COUNT] {
    [
        CompoundTailState::Predicate,
        CompoundTailState::Ending,
        CompoundTailState::Particle,
    ]
}

fn advance_compound_tail_suffix(
    mut state: CompoundTailState,
    positions: &[StructuralPos],
) -> Option<CompoundTailState> {
    for &pos in positions {
        state = advance_compound_tail_position(state, pos)?;
    }
    Some(state)
}

fn advance_compound_tail_position(
    state: CompoundTailState,
    pos: StructuralPos,
) -> Option<CompoundTailState> {
    match (state, pos) {
        (CompoundTailState::Predicate | CompoundTailState::Ending, pos) if pos.is_ending() => {
            Some(CompoundTailState::Ending)
        }
        (CompoundTailState::Ending | CompoundTailState::Particle, pos) if pos.is_particle() => {
            Some(CompoundTailState::Particle)
        }
        _ => None,
    }
}

fn attached_auxiliary_spans(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    connective_boundary: &[bool],
    ending_suffix: &[bool],
) -> Box<[Range<usize>]> {
    let edges = graph.edges();

    let mut spans = edges
        .iter()
        .filter(|edge| {
            let mut positions = graph.positions(edge).iter().copied();
            connective_boundary[edge.span.start]
                && positions.next() == Some(StructuralPos::Fine(DataFinePos::Vx))
                && positions.all(StructuralPos::is_ending)
                && ending_suffix[edge.span.end]
        })
        .map(|edge| edge.span.clone())
        .collect::<Vec<_>>();
    spans.extend(
        edges
            .iter()
            .filter(|edge| {
                edge.span == (0..text_len)
                    && is_attached_auxiliary_whole_path(graph.positions(edge))
            })
            .map(|edge| edge.span.clone()),
    );
    if has_complete_attached_auxiliary_path(text_len, graph) {
        spans.push(0..text_len);
    }
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum AttachedAuxiliaryState {
    Start,
    Root,
    Predicate,
    Connective,
    Auxiliary,
    AuxiliaryEnding,
}

const ATTACHED_AUXILIARY_STATES: [AttachedAuxiliaryState; 6] = [
    AttachedAuxiliaryState::Start,
    AttachedAuxiliaryState::Root,
    AttachedAuxiliaryState::Predicate,
    AttachedAuxiliaryState::Connective,
    AttachedAuxiliaryState::Auxiliary,
    AttachedAuxiliaryState::AuxiliaryEnding,
];
const ATTACHED_AUXILIARY_STATE_COUNT: usize = ATTACHED_AUXILIARY_STATES.len();

fn has_complete_attached_auxiliary_path(text_len: usize, graph: &EdgeGraph<'_>) -> bool {
    let mut reachable = vec![[false; ATTACHED_AUXILIARY_STATE_COUNT]; text_len + 1];
    reachable[0][AttachedAuxiliaryState::Start as usize] = true;
    for start in 0..text_len {
        for edge in graph.starting_at(start) {
            for state in ATTACHED_AUXILIARY_STATES {
                if !reachable[start][state as usize] {
                    continue;
                }
                if let Some(next) = advance_attached_auxiliary_path(state, graph.positions(edge)) {
                    reachable[edge.span.end][next as usize] = true;
                }
            }
        }
    }
    reachable[text_len][AttachedAuxiliaryState::AuxiliaryEnding as usize]
}

fn advance_attached_auxiliary_path(
    mut state: AttachedAuxiliaryState,
    positions: &[StructuralPos],
) -> Option<AttachedAuxiliaryState> {
    for &pos in positions {
        state = match (state, pos) {
            (
                AttachedAuxiliaryState::Start,
                StructuralPos::Fine(DataFinePos::Vv | DataFinePos::Va),
            ) => AttachedAuxiliaryState::Predicate,
            (AttachedAuxiliaryState::Start, StructuralPos::Xr) => AttachedAuxiliaryState::Root,
            (AttachedAuxiliaryState::Root, StructuralPos::Xsv | StructuralPos::Xsa) => {
                AttachedAuxiliaryState::Predicate
            }
            (AttachedAuxiliaryState::Predicate | AttachedAuxiliaryState::Connective, pos)
                if pos.is_ending() =>
            {
                if pos == StructuralPos::Ec {
                    AttachedAuxiliaryState::Connective
                } else {
                    AttachedAuxiliaryState::Predicate
                }
            }
            (AttachedAuxiliaryState::Connective, StructuralPos::Fine(DataFinePos::Vx)) => {
                AttachedAuxiliaryState::Auxiliary
            }
            (AttachedAuxiliaryState::Auxiliary, pos) if pos.is_ending() => {
                AttachedAuxiliaryState::AuxiliaryEnding
            }
            (AttachedAuxiliaryState::AuxiliaryEnding, pos) if pos.is_ending() => {
                AttachedAuxiliaryState::AuxiliaryEnding
            }
            _ => return None,
        };
    }
    Some(state)
}

fn is_attached_auxiliary_whole_path(positions: &[StructuralPos]) -> bool {
    let mut positions = positions.iter().copied();
    match positions.next() {
        Some(StructuralPos::Fine(DataFinePos::Vv | DataFinePos::Va)) => {}
        Some(StructuralPos::Xr)
            if matches!(
                positions.next(),
                Some(StructuralPos::Xsv | StructuralPos::Xsa)
            ) => {}
        _ => return false,
    }

    let mut connective_before_auxiliary = false;
    for position in &mut positions {
        if position == StructuralPos::Fine(DataFinePos::Vx) {
            return connective_before_auxiliary
                && positions.next().is_some_and(StructuralPos::is_ending)
                && positions.all(StructuralPos::is_ending);
        }
        if !position.is_ending() {
            return false;
        }
        connective_before_auxiliary = position == StructuralPos::Ec;
    }
    false
}

fn is_attached_auxiliary_whole_path_raw(pos: &str) -> bool {
    let mut positions = pos.split('+');
    match positions.next() {
        Some("VV" | "VA") => {}
        Some("XR") if matches!(positions.next(), Some("XSV" | "XSA")) => {}
        _ => return false,
    }

    let mut connective_before_auxiliary = false;
    for position in &mut positions {
        if position == "VX" {
            return connective_before_auxiliary
                && positions
                    .next()
                    .is_some_and(|ending| ending.starts_with('E'))
                && positions.all(|ending| ending.starts_with('E'));
        }
        if !position.starts_with('E') {
            return false;
        }
        connective_before_auxiliary = position == "EC";
    }
    false
}

fn leading_predicate_spans(graph: &EdgeGraph<'_>, ending_suffix: &[bool]) -> Box<[Range<usize>]> {
    let mut spans = graph
        .edges()
        .iter()
        .filter_map(|edge| {
            let mut positions = graph.positions(edge).iter().copied();
            (edge.span.start == 0
                && positions.next().is_some_and(StructuralPos::is_predicate)
                && positions.all(StructuralPos::is_ending)
                && ending_suffix[edge.span.end])
                .then(|| edge.span.clone())
        })
        .collect::<Vec<_>>();
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}

fn runtime_nominal_derivation_spans(
    graph: &EdgeGraph<'_>,
    complete: &[bool],
) -> Box<[Range<usize>]> {
    let edges = graph.edges();
    let mut spans = graph
        .starting_range(0)
        .filter(|&index| {
            let edge = &edges[index];
            let positions = graph.positions(edge);
            complete[index]
                && positions.last().is_some_and(|pos| pos.is_nominal())
                && graph.starting_range(edge.span.end).any(|next_index| {
                    let next = &edges[next_index];
                    complete[next_index]
                        && graph.positions(next).first().is_some_and(|pos| {
                            pos.is_predicate_tag()
                                || matches!(pos, StructuralPos::Xsv | StructuralPos::Xsa)
                        })
                })
        })
        .map(|index| edges[index].span.clone())
        .collect::<Vec<_>>();
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum NominalDerivationState {
    Start,
    Nominal,
    DerivedNominal,
}

const NOMINAL_DERIVATION_STATE_COUNT: usize = 3;

fn nominal_derivation_state(index: usize) -> NominalDerivationState {
    match index {
        0 => NominalDerivationState::Start,
        1 => NominalDerivationState::Nominal,
        2 => NominalDerivationState::DerivedNominal,
        _ => unreachable!("invalid nominal derivation state"),
    }
}

fn advance_nominal_derivation(
    state: NominalDerivationState,
    pos: StructuralPos,
) -> Option<NominalDerivationState> {
    match (state, pos) {
        (NominalDerivationState::Start, pos) if pos.is_nominal() => {
            Some(NominalDerivationState::Nominal)
        }
        (NominalDerivationState::Nominal, pos) if pos.is_nominal() => {
            Some(NominalDerivationState::Nominal)
        }
        (
            NominalDerivationState::Nominal | NominalDerivationState::DerivedNominal,
            StructuralPos::Xsn,
        ) => Some(NominalDerivationState::DerivedNominal),
        _ => None,
    }
}

fn nominal_derivation_predicate_prefixes(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    ending_suffix: &[bool],
) -> Box<[Range<usize>]> {
    let mut nominal_prefix = vec![[false; NOMINAL_DERIVATION_STATE_COUNT]; text_len + 1];
    nominal_prefix[0][NominalDerivationState::Start as usize] = true;
    for start in 0..text_len {
        for edge in graph.starting_at(start) {
            for state_index in 0..NOMINAL_DERIVATION_STATE_COUNT {
                if !nominal_prefix[start][state_index] {
                    continue;
                }
                let Some(next) = graph.positions(edge).iter().copied().try_fold(
                    nominal_derivation_state(state_index),
                    advance_nominal_derivation,
                ) else {
                    continue;
                };
                nominal_prefix[edge.span.end][next as usize] = true;
            }
        }
    }

    let mut prefixes = (0..text_len)
        .filter(|&end| {
            nominal_prefix[end][NominalDerivationState::DerivedNominal as usize]
                && graph.starting_at(end).iter().any(|edge| {
                    let mut positions = graph.positions(edge).iter().copied();
                    matches!(
                        positions.next(),
                        Some(StructuralPos::Xsv | StructuralPos::Xsa)
                    ) && positions.clone().all(StructuralPos::is_ending)
                        && ending_suffix[edge.span.end]
                        && (positions.next().is_some() || edge.span.end < text_len)
                })
        })
        .map(|end| 0..end)
        .collect::<Vec<_>>();
    prefixes.sort_unstable_by_key(|span| (span.start, span.end));
    prefixes.dedup();
    prefixes.into_boxed_slice()
}

fn derivational_suffix_starts(graph: &EdgeGraph<'_>, complete: &[bool]) -> Box<[usize]> {
    let mut starts = graph
        .edges()
        .iter()
        .enumerate()
        .filter(|(index, edge)| {
            complete[*index]
                && graph
                    .positions(edge)
                    .first()
                    .is_some_and(|pos| matches!(pos, StructuralPos::Xsv | StructuralPos::Xsa))
        })
        .map(|(_, edge)| edge.span.start)
        .collect::<Vec<_>>();
    starts.sort_unstable();
    starts.dedup();
    starts.into_boxed_slice()
}

fn adnominal_derivation_suffix_starts(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    common_nominal_prefix: &[[bool; 2]],
) -> Box<[usize]> {
    let edges = graph.edges();

    let mut adnominal_ending_suffix = vec![false; text_len + 1];
    for start in (0..text_len).rev() {
        adnominal_ending_suffix[start] = graph.starting_at(start).iter().any(|edge| {
            ending_path_reaches_adnominal_boundary(
                graph.positions(edge),
                edge.span.end,
                text_len,
                &adnominal_ending_suffix,
            )
        });
    }

    let mut starts = edges
        .iter()
        .filter_map(|edge| {
            if !common_nominal_prefix[edge.span.start]
                .iter()
                .any(|&reachable| reachable)
            {
                return None;
            }
            let (first, endings) = graph.positions(edge).split_first()?;
            if !matches!(first, StructuralPos::Xsv | StructuralPos::Xsa) {
                return None;
            }
            let complete = ending_path_reaches_adnominal_boundary(
                endings,
                edge.span.end,
                text_len,
                &adnominal_ending_suffix,
            );
            complete.then_some(edge.span.start)
        })
        .collect::<Vec<_>>();
    starts.sort_unstable();
    starts.dedup();
    starts.into_boxed_slice()
}

fn ending_path_reaches_adnominal_boundary(
    positions: &[StructuralPos],
    end: usize,
    text_len: usize,
    suffix: &[bool],
) -> bool {
    let mut saw_adnominal = false;
    let mut last = None;
    for &position in positions {
        if !position.is_ending() {
            return false;
        }
        saw_adnominal |= position == StructuralPos::Etm;
        last = Some(position);
    }
    if saw_adnominal {
        last == Some(StructuralPos::Etm) && end == text_len
    } else {
        suffix[end]
    }
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

fn nominal_copula_hosts(
    text: &str,
    graph: &EdgeGraph<'_>,
    common_nominal_prefix: &[[bool; 2]],
) -> Box<[Range<usize>]> {
    if !text
        .char_indices()
        .skip(1)
        .any(|(_, character)| matches!(character, '이' | '입'))
    {
        return Box::default();
    }
    let text_len = text.len();
    let mut suffix = vec![[false; COPULA_SUFFIX_STATE_COUNT]; text_len + 1];
    suffix[text_len][CopulaSuffixState::Ending as usize] = true;
    suffix[text_len][CopulaSuffixState::Particle as usize] = true;
    for start in (0..text_len).rev() {
        for edge in graph.starting_at(start) {
            for state in copula_suffix_states() {
                let Some(next) = advance_copula_suffix(state, graph.positions(edge)) else {
                    continue;
                };
                suffix[start][state as usize] |= suffix[edge.span.end][next as usize];
            }
        }
    }

    common_nominal_prefix
        .iter()
        .enumerate()
        .skip(1)
        .take(text_len.saturating_sub(1))
        .filter_map(|(end, states)| {
            (states.iter().any(|&reachable| reachable)
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
    positions: &[StructuralPos],
) -> Option<CopulaSuffixState> {
    for &pos in positions {
        state = match (state, pos) {
            (CopulaSuffixState::Start, StructuralPos::Fine(DataFinePos::Vcp)) => {
                CopulaSuffixState::Copula
            }
            (CopulaSuffixState::Copula | CopulaSuffixState::Ending, pos) if pos.is_ending() => {
                CopulaSuffixState::Ending
            }
            (CopulaSuffixState::Ending | CopulaSuffixState::Particle, pos) if pos.is_particle() => {
                CopulaSuffixState::Particle
            }
            _ => return None,
        };
    }
    Some(state)
}

fn predicate_path_ends_in_connective(positions: &[StructuralPos]) -> Option<bool> {
    let (first, positions) = positions.split_first()?;
    if !matches!(
        first,
        StructuralPos::Fine(DataFinePos::Vv | DataFinePos::Va)
    ) {
        return None;
    }
    ending_positions_end_in_connective(positions)
}

fn ending_path_ends_in_connective(positions: &[StructuralPos]) -> Option<bool> {
    ending_positions_end_in_connective(positions)
}

fn ending_positions_end_in_connective(positions: &[StructuralPos]) -> Option<bool> {
    let mut connective = false;
    for &position in positions {
        match position {
            StructuralPos::Ep if !connective => {}
            StructuralPos::Ec if !connective => connective = true,
            _ => return None,
        }
    }
    Some(connective)
}

fn forward_positions(text_len: usize, graph: &EdgeGraph<'_>) -> Vec<bool> {
    let mut forward = vec![false; text_len + 1];
    forward[0] = true;
    for start in 0..text_len {
        if !forward[start] {
            continue;
        }
        for edge in graph.starting_at(start) {
            forward[edge.span.end] = true;
        }
    }
    forward
}

fn forward_positions_with_prefix(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    prefix_end: usize,
) -> Vec<bool> {
    let mut forward = vec![false; text_len + 1];
    forward[0] = true;
    forward[prefix_end] = true;
    for start in 0..text_len {
        if !forward[start] {
            continue;
        }
        for edge in graph.starting_at(start) {
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

fn numeral_path_transition(
    state: NumeralPathState,
    positions: &[StructuralPos],
) -> Option<NumeralPathState> {
    let particle = positions.iter().all(|pos| pos.is_particle());
    match (state, positions) {
        (NumeralPathState::Start, [StructuralPos::Fine(DataFinePos::Nr)]) => {
            Some(NumeralPathState::OneNumeral)
        }
        (
            NumeralPathState::OneNumeral | NumeralPathState::ManyNumerals,
            [StructuralPos::Fine(DataFinePos::Nr)],
        ) => Some(NumeralPathState::ManyNumerals),
        (
            NumeralPathState::OneNumeral | NumeralPathState::ManyNumerals,
            [StructuralPos::Fine(DataFinePos::Nnb) | StructuralPos::Nnbc],
        ) => Some(NumeralPathState::Unit),
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

fn hangul_numeral_spans(text_len: usize, graph: &EdgeGraph<'_>) -> Vec<Range<usize>> {
    numeral_sequence_spans(text_len, 0, graph, false)
}

fn numeral_sequence_spans(
    text_len: usize,
    sequence_start: usize,
    graph: &EdgeGraph<'_>,
    require_unit: bool,
) -> Vec<Range<usize>> {
    let mut forward = vec![[false; NUMERAL_PATH_STATE_COUNT]; text_len + 1];
    forward[sequence_start][NumeralPathState::Start as usize] = true;
    for start in sequence_start..text_len {
        for edge in graph.starting_at(start) {
            for state_index in 0..NUMERAL_PATH_STATE_COUNT {
                if !forward[start][state_index] {
                    continue;
                }
                let state = numeral_path_state(state_index);
                if let Some(next) = numeral_path_transition(state, graph.positions(edge)) {
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
        for edge in graph.starting_at(start) {
            for state_index in 0..NUMERAL_PATH_STATE_COUNT {
                let state = numeral_path_state(state_index);
                let Some(next) = numeral_path_transition(state, graph.positions(edge)) else {
                    continue;
                };
                backward[start][state_index] |= backward[edge.span.end][next as usize];
            }
        }
    }

    let mut spans = Vec::new();
    for edge in graph
        .edges()
        .iter()
        .filter(|edge| graph.positions(edge) == [StructuralPos::Fine(DataFinePos::Nr)])
    {
        let belongs_to_complete_path = (0..NUMERAL_PATH_STATE_COUNT).any(|state_index| {
            if !forward[edge.span.start][state_index] {
                return false;
            }
            let state = numeral_path_state(state_index);
            numeral_path_transition(state, graph.positions(edge))
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

fn complete_edges(text_len: usize, graph: &EdgeGraph<'_>, forward: &[bool]) -> Vec<bool> {
    let mut backward = vec![false; text_len + 1];
    backward[text_len] = true;
    for start in (0..text_len).rev() {
        backward[start] = graph
            .starting_at(start)
            .iter()
            .any(|edge| backward[edge.span.end]);
    }
    graph
        .edges()
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
        for unit in evidence.units.all() {
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
        if supports.len() == support_start && is_predicate_nominalization(pattern) {
            for unit in evidence
                .units
                .all()
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
            && (query_nominal_particle_path(pattern, spans)
                || composed_nominal_subpath(pattern, spans, evidence)
                || nominal_derivation_before_predicate(pattern, spans, evidence)
                || ((!requires_aligned_component_evidence(pattern) || spans.core == spans.token)
                    && evidence.runtime_spans.contains(&spans.core))
                || attached_auxiliary_is_supported(pattern, spans, evidence)
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

fn requires_aligned_component_evidence(pattern: &QueryMorphPattern) -> bool {
    matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Mm | DataFinePos::Mag | DataFinePos::Maj
    )
}

#[derive(Clone, Debug)]
enum StructureSelection {
    Whole,
    Adverb,
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
    NumericUnitDependentNoun {
        unit: Range<usize>,
        tail: Range<usize>,
    },
    NumeralSequence {
        fallback: Box<StructureSelection>,
    },
    AdnominalDerivation {
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
            Self::AdnominalDerivation { fallback } => fallback.graph_nominal_host(),
            _ => None,
        }
    }

    fn accepts(
        &self,
        support: &ConstraintSupport,
        spans: &CandidateSpans,
        patterns: &[QueryMorphPattern],
        text: &str,
        evidence: &TokenEvidence,
    ) -> bool {
        let Some(pattern) = patterns.get(support.pattern_index) else {
            return false;
        };
        if query_nominal_particle_path(pattern, spans) {
            return true;
        }
        if composed_nominal_subpath(pattern, spans, evidence) {
            return true;
        }
        if nominal_derivation_before_predicate(pattern, spans, evidence) {
            return true;
        }
        if compound_predicate_component_is_supported(pattern, spans, evidence) {
            return true;
        }
        match self {
            Self::Whole => support.evidence == StructuralEvidence::Whole,
            Self::Adverb => {
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
                        && !component_is_shadowed_by_predicate(
                            support.evidence,
                            pattern,
                            spans,
                            evidence,
                        )
                        && ((*allow_whole_nominal_source_components
                            && support.evidence == StructuralEvidence::SourceComponent
                            && evidence.has_whole_nominal_source_component(
                                &spans.core,
                                pattern.fine_pos,
                            ))
                            || (matches!(
                                pattern.fine_pos,
                                DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
                            ) && !evidence.has_whole_analysis(&spans.token)
                                && (modifier_led_nominal_component_is_on_preferred_path(
                                    &spans.core,
                                    selected,
                                    text,
                                    evidence,
                                ) || (selected.start == spans.token.start
                                    && selected.end == spans.core.start
                                    && modifier_led_nominal_component_is_on_preferred_path(
                                        &spans.core,
                                        &spans.token,
                                        text,
                                        evidence,
                                    ))))
                            || spans.core == *selected
                            || (evidence.nominal_particle_hosts.contains(&spans.core)
                                && spans.consumed == spans.token
                                && matches!(
                                    pattern.continuation,
                                    MorphContinuation::NominalParticles
                                ))
                            || (spans.core.start == selected.start
                                && spans.consumed.end == selected.end
                                && evidence.units.all().iter().any(|unit| {
                                    unit.span == (spans.core.end..selected.end)
                                        && unit.pos.is_particle()
                                }))
                            || attached_nominal_suffix_is_supported(pattern, spans, evidence)
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
                    || (is_predicate_nominalization(pattern)
                        && spans.anchor.start >= selected.start
                        && spans.anchor.end <= selected.end
                        && (spans.consumed.end == spans.token.end
                            || spans.anchor.start > selected.start
                            || spans.anchor.end < selected.end
                            || (spans.anchor != spans.token
                                && evidence.units.all().iter().any(|unit| {
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
                        && runtime_position_is_supported(pattern, spans, text, evidence))
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
            Self::NumericUnitDependentNoun { unit, tail } => {
                (matches!(pattern.fine_pos, DataFinePos::Nnb | DataFinePos::Nr)
                    && spans.core == *unit)
                    || (pattern.fine_pos == DataFinePos::Nnb
                        && spans.core == *tail
                        && spans.consumed.end == spans.token.end)
            }
            Self::NumeralSequence { fallback } => {
                (pattern.fine_pos == DataFinePos::Nr
                    && evidence.numeric_spans.contains(&spans.core))
                    || fallback.accepts(support, spans, patterns, text, evidence)
            }
            Self::AdnominalDerivation { fallback } => {
                !(support.evidence == StructuralEvidence::RuntimeComponent
                    && pattern.fine_pos.is_nominal()
                    && evidence
                        .adnominal_derivation_suffix_starts
                        .binary_search(&spans.core.start)
                        .is_ok())
                    && fallback.accepts(support, spans, patterns, text, evidence)
            }
            Self::RuntimeCompatible { graph_nominal_host } => match support.evidence {
                StructuralEvidence::Whole => true,
                StructuralEvidence::SourceComponent => {
                    !component_is_shadowed_by_predicate(support.evidence, pattern, spans, evidence)
                }
                StructuralEvidence::RuntimeComponent => {
                    !component_is_shadowed_by_predicate(support.evidence, pattern, spans, evidence)
                        && runtime_position_is_supported(pattern, spans, text, evidence)
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

fn compound_predicate_component_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    matches!(pattern.continuation, MorphContinuation::Predicate { .. })
        && !evidence.has_whole_modifier()
        && spans.core.start > spans.token.start
        && spans.consumed.start == spans.core.start
        && spans.consumed.end == spans.token.end
        && evidence
            .compound_predicate_components
            .binary_search(&PredicateComponent {
                start: spans.core.start,
                end: spans.core.end,
                pos: pattern.fine_pos,
            })
            .is_ok()
}

fn component_is_shadowed_by_predicate(
    support: StructuralEvidence,
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    let unsupported_runtime_component = support == StructuralEvidence::RuntimeComponent
        && (((pattern.fine_pos.is_nominal() && pattern.lexical_form.as_ref() == "못")
            || (matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
                && pattern.lexical_form.chars().count() == 1))
            && evidence
                .runtime_nominal_derivation_spans
                .contains(&spans.core)
            || matches!(pattern.fine_pos, DataFinePos::Mag | DataFinePos::Maj));
    let source_backed_nonderivational_nominal = support == StructuralEvidence::SourceComponent
        && pattern.fine_pos.is_nominal()
        && pattern.lexical_form.as_ref() == "못";
    (unsupported_runtime_component || source_backed_nonderivational_nominal)
        && evidence
            .leading_predicate_spans
            .iter()
            .any(|predicate| predicate.start == spans.core.start && predicate.end > spans.core.end)
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
    component_is_on_preferred_path(core, selected, evidence, |unit| {
        unit.pos.is_nominal() && unit.span != *selected
    })
}

fn modifier_led_nominal_component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    text: &str,
    evidence: &TokenEvidence,
) -> bool {
    if core.start == selected.start {
        return false;
    }
    let mut has_leading_modifier = false;
    for unit in evidence.units.all() {
        if unit.span == *selected && unit.evidence == StructuralEvidence::Whole {
            return false;
        }
        has_leading_modifier |= unit.pos == DataFinePos::Mm
            && unit.span.start == selected.start
            && matches!(
                unit.evidence,
                StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
            );
    }
    if !has_leading_modifier {
        return false;
    }
    let direct_modifier = selected.start..core.start;
    let single_syllable_direct_modifier = core.end == selected.end
        && evidence.units.all().iter().any(|unit| {
            unit.pos == DataFinePos::Mm
                && unit.span == direct_modifier
                && text
                    .get(unit.span.clone())
                    .is_some_and(|surface| surface.chars().count() == 1)
        });
    if single_syllable_direct_modifier
        && !evidence.units.all().iter().any(|unit| {
            unit.pos == DataFinePos::Nr
                && unit.span == direct_modifier
                && matches!(
                    unit.evidence,
                    StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
                )
        })
    {
        return false;
    }
    component_is_on_preferred_path(core, selected, evidence, |unit| {
        (unit.pos == DataFinePos::Mm && unit.span.start == selected.start)
            || (matches!(
                unit.pos,
                DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
            ) && unit.span.start > selected.start)
    })
}

fn component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
    accepts: impl Copy + Fn(&Unit) -> bool,
) -> bool {
    evidence
        .units
        .contains_on_preferred_path(core, selected, accepts)
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
    if attached_nominal_suffix_is_supported(pattern, spans, evidence) {
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
        && evidence.units.all().iter().any(|unit| {
            unit.pos == DataFinePos::Nnp
                && unit.span.start == host.start
                && unit.span.end == spans.core.start
                && unit.span.len() > spans.core.len()
        })
        && evidence.units.all().iter().any(|unit| {
            unit.pos.is_particle()
                && unit.span.start == spans.core.end
                && unit.span.end <= spans.consumed.end
        })
}

fn attached_nominal_suffix_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    let nominal_host = spans.token.start..spans.core.end;
    pattern.fine_pos == DataFinePos::Nng
        && pattern.lexical_form.chars().count() == 1
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && evidence.has_complete_path
        && spans.core.start > spans.token.start
        && spans.consumed.end == spans.token.end
        && spans.consumed.end > spans.core.end
        && !evidence.units.all().iter().any(|unit| {
            unit.span == nominal_host && unit.evidence != StructuralEvidence::SourceComponent
        })
        && is_nikl_attached_nominal_suffix(&pattern.lexical_form)
        && minimum_unit_path(
            &(spans.token.start..spans.core.start),
            evidence,
            DataFinePos::is_nominal,
        )
        .is_some()
        && minimum_unit_path(
            &(spans.core.end..spans.consumed.end),
            evidence,
            DataFinePos::is_particle,
        )
        .is_some()
}

fn is_nikl_attached_nominal_suffix(lexical_form: &str) -> bool {
    static SURFACES: OnceLock<Box<[&'static str]>> = OnceLock::new();
    let surfaces = SURFACES.get_or_init(|| {
        let mut surfaces = NIKL_ATTACHED_NOMINAL_SUFFIXES
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once('\t').map(|(surface, _)| surface))
            .filter(|surface| !surface.is_empty())
            .collect::<Vec<_>>();
        surfaces.sort_unstable();
        surfaces.dedup();
        surfaces.into_boxed_slice()
    });
    surfaces.binary_search(&lexical_form).is_ok()
}

fn runtime_position_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    text: &str,
    evidence: &TokenEvidence,
) -> bool {
    let starts_token = spans.core.start == spans.token.start;
    let leading_only = matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm | DataFinePos::Mag
    );
    let modifier_without_nominal_tail = pattern.fine_pos == DataFinePos::Mm
        && spans.core != spans.token
        && !modifier_has_complete_nominal_tail(&spans.core, &spans.token, evidence);
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
    let attached_auxiliary = attached_auxiliary_is_supported(pattern, spans, evidence);
    let trailing_predicate_subspan = predicate
        && spans.consumed.end != spans.token.end
        && !terminal_predicate_component
        && !whole_predicate_continuation;
    let internal_runtime_predicate = predicate
        && (pattern.fine_pos != DataFinePos::Vcp || evidence.has_whole(DataFinePos::Mag))
        && spans.core.start != spans.token.start
        && spans.consumed == spans.core
        && !attached_auxiliary
        && !is_predicate_nominalization(pattern);
    let modifier_before_predicate = predicate
        && !copula_nominal_host
        && !attached_auxiliary
        && spans.core.start != spans.token.start
        && evidence.units.all().iter().any(|unit| {
            unit.span.end == spans.core.start
                && matches!(unit.pos, DataFinePos::Mag | DataFinePos::Maj)
        });
    let exact_component_prefix = (matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm
    ) || (pattern.fine_pos == DataFinePos::Mag
        && evidence.has_complete_path)
        || (matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
            && pattern.lexical_form.chars().count() > 1))
        && starts_token
        && !evidence
            .units
            .all()
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
    let unlexicalized_internal_one_syllable_nominal =
        matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
            && pattern.lexical_form.chars().count() == 1
            && matches!(pattern.continuation, MorphContinuation::NominalParticles)
            && spans.core.start > spans.token.start
            && spans.consumed == spans.core
            && !evidence.has_whole_analysis(&spans.token);
    let glued_dependent_noun =
        pattern.fine_pos == DataFinePos::Nnb && evidence.has_adnominal_ending_at(spans.core.start);
    let terminal_nominal_in_predicate_frame = pattern.fine_pos.is_nominal()
        && pattern.lexical_form.chars().count() == 1
        && spans.core.start > spans.token.start
        && spans.core.end == spans.token.end
        && evidence.units.all().iter().any(|unit| {
            unit.pos.is_predicate()
                && ((unit.span.start == spans.token.start && unit.span.end <= spans.core.start)
                    || (unit.span.start < spans.core.start && unit.span.end >= spans.core.end))
        })
        && !glued_dependent_noun
        && !modifier_led_nominal_component_is_on_preferred_path(
            &spans.core,
            &spans.token,
            text,
            evidence,
        );

    (!leading_only || starts_token)
        && !modifier_without_nominal_tail
        && !trailing_predicate_subspan
        && !internal_runtime_predicate
        && !modifier_before_predicate
        && !trailing_exact_subspan
        && !trailing_nominal_chain
        && !nominal_after_predicate
        && !unlexicalized_internal_one_syllable_nominal
        && !terminal_nominal_in_predicate_frame
}

fn modifier_has_complete_nominal_tail(
    core: &Range<usize>,
    token: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    if core.start != token.start || core.end >= token.end {
        return false;
    }
    evidence
        .units
        .all()
        .iter()
        .map(|unit| unit.span.end)
        .filter(|&host_end| host_end > core.end && host_end <= token.end)
        .any(|host_end| {
            minimum_unit_path(&(core.end..host_end), evidence, DataFinePos::is_nominal).is_some()
                && minimum_unit_path(&(host_end..token.end), evidence, DataFinePos::is_particle)
                    .is_some()
        })
}

fn attached_auxiliary_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos == DataFinePos::Vx
        && !evidence.has_whole(DataFinePos::Mag)
        && !evidence.has_whole(DataFinePos::Maj)
        && evidence.attached_auxiliary_spans.iter().any(|frame| {
            frame == &spans.core
                || (pattern.lexical_form.as_ref() == "지"
                    && frame == &spans.token
                    && frame.start < spans.core.start
                    && spans.core.end <= frame.end)
        })
}

fn whole_predicate_continuation(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    evidence.units.all().iter().any(|unit| {
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
            .all()
            .iter()
            .any(|unit| unit.span == (spans.token.start..spans.core.start) && unit.pos.is_nominal())
}

fn is_predicate_nominalization(pattern: &QueryMorphPattern) -> bool {
    matches!(
        pattern.continuation,
        MorphContinuation::Predicate {
            nominal_particles: true,
            ..
        }
    )
}

fn query_nominal_particle_path(pattern: &QueryMorphPattern, spans: &CandidateSpans) -> bool {
    pattern.fine_pos.is_nominal()
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && spans.core.start == spans.token.start
        && spans.core.end < spans.consumed.end
        && spans.consumed == spans.token
}

fn nominal_derivation_before_predicate(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos.is_nominal()
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && spans.core.start == spans.token.start
        && evidence
            .nominal_derivation_predicate_prefixes
            .binary_search_by_key(&(spans.core.start, spans.core.end), |prefix| {
                (prefix.start, prefix.end)
            })
            .is_ok()
}

fn composed_nominal_subpath(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    if !pattern.fine_pos.is_nominal()
        || !matches!(pattern.continuation, MorphContinuation::NominalParticles)
        || evidence
            .derivational_suffix_starts
            .binary_search(&spans.core.end)
            .is_ok()
        || minimum_unit_path(&spans.core, evidence, DataFinePos::is_nominal)
            .is_none_or(|units| units < 2)
    {
        return false;
    }
    evidence
        .units
        .all()
        .iter()
        .map(|unit| unit.span.end)
        .filter(|&host_end| host_end >= spans.core.end && host_end <= spans.token.end)
        .any(|host_end| {
            minimum_unit_path(
                &(spans.token.start..host_end),
                evidence,
                DataFinePos::is_nominal,
            )
            .is_some()
                && minimum_unit_path(
                    &(host_end..spans.token.end),
                    evidence,
                    DataFinePos::is_particle,
                )
                .is_some()
        })
}

fn minimum_unit_path(
    span: &Range<usize>,
    evidence: &TokenEvidence,
    accepts: impl Copy + Fn(DataFinePos) -> bool,
) -> Option<usize> {
    evidence.units.minimum_path_len(span, accepts)
}

fn select_structure(
    resource: &ComponentResource,
    context: BoundedTokenContext<'_>,
    evidence: &TokenEvidence,
) -> StructureSelection {
    if (context.previous == Some(context.current) || context.next == Some(context.current))
        && evidence.has_whole(DataFinePos::Mag)
    {
        return StructureSelection::Adverb;
    }
    if evidence.has_whole(DataFinePos::Mag)
        && evidence
            .units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos.is_nominal())
        && context
            .next
            .is_some_and(|next| complete_ha_predicate_path(resource, next))
    {
        return StructureSelection::Adverb;
    }
    let next_starts_nominal = context.next.is_some_and(|next| {
        let exact_nominal =
            exact_analysis_starts_with_pos(resource, next, |pos| pos.starts_with('N'));
        let exact_competitor =
            exact_analysis_starts_with_pos(resource, next, |pos| !pos.starts_with('N'));
        let complete_nominal = complete_nominal_host(resource, next)
            || complete_nominal_particle_host(resource, next).is_some();
        !nominal_particle_hosts(resource, next).is_empty()
            || (!exact_competitor && (exact_nominal || complete_nominal))
    });
    let particle_host = evidence.nominal_particle_hosts.last().cloned();
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
    if !evidence.has_numeral_sequence {
        if let (Some(unit), Some(tail)) = (
            evidence.numeric_spans.first(),
            evidence.numeric_dependent_tail.as_ref(),
        ) {
            return StructureSelection::NumericUnitDependentNoun {
                unit: unit.clone(),
                tail: tail.clone(),
            };
        }
        if let Some(unit) = evidence.numeric_spans.first() {
            return StructureSelection::NumericUnit { unit: unit.clone() };
        }
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
            graph_nominal_host: evidence.complete_nominal_particle_host.clone(),
        }
    };
    let fallback = if next_starts_nominal && !evidence.adnominal_derivation_suffix_starts.is_empty()
    {
        StructureSelection::AdnominalDerivation {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    };
    if evidence.has_numeral_sequence {
        StructureSelection::NumeralSequence {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    }
}

fn numeric_unit_path(resource: &ComponentResource, text: &str) -> Option<NumericUnitPath> {
    let numeric_end = text.bytes().take_while(u8::is_ascii_digit).count();
    if numeric_end == 0 || numeric_end == text.len() {
        return None;
    }
    let mut selected = None;
    resource.common_prefixes(&text.as_bytes()[numeric_end..], |unit_length, analyses| {
        if !analyses
            .iter()
            .any(|analysis| matches!(analysis.pos, "NNB" | "NNBC" | "NR"))
        {
            return;
        }
        let unit_end = numeric_end + unit_length;
        if complete_suffix(resource, &text[unit_end..], |pos| pos.starts_with('J')) {
            select_numeric_unit_path(
                &mut selected,
                NumericUnitPath {
                    unit: numeric_end..unit_end,
                    dependent_tail: None,
                },
            );
        }
        resource.common_prefixes(&text.as_bytes()[unit_end..], |tail_length, analyses| {
            if !analyses
                .iter()
                .any(|analysis| matches!(analysis.pos, "NNB" | "NNBC"))
            {
                return;
            }
            let tail_end = unit_end + tail_length;
            if !complete_suffix(resource, &text[tail_end..], |pos| pos.starts_with('J')) {
                return;
            }
            select_numeric_unit_path(
                &mut selected,
                NumericUnitPath {
                    unit: numeric_end..unit_end,
                    dependent_tail: Some(unit_end..tail_end),
                },
            );
        });
    });
    selected
}

fn select_numeric_unit_path(selected: &mut Option<NumericUnitPath>, candidate: NumericUnitPath) {
    let rank = |path: &NumericUnitPath| {
        (
            path.dependent_tail
                .as_ref()
                .map_or(path.unit.end, |tail| tail.end),
            path.unit.end,
            path.dependent_tail.is_some(),
        )
    };
    if selected
        .as_ref()
        .is_none_or(|current| rank(&candidate) > rank(current))
    {
        *selected = Some(candidate);
    }
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

fn complete_ha_predicate_path(resource: &ComponentResource, text: &str) -> bool {
    ["하", "해", "했"].into_iter().any(|surface| {
        text.starts_with(surface)
            && exact_analysis_starts_with_pos(resource, surface, |pos| pos == "VV")
            && complete_suffix(resource, &text[surface.len()..], |pos| pos.starts_with('E'))
    })
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

fn nominal_particle_hosts(resource: &ComponentResource, current: &str) -> Vec<Range<usize>> {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && complete_suffix(resource, &current[split..], |pos| pos.starts_with('J'))
        })
        .map(|end| 0..end)
        .collect()
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
    let mut reachable = vec![false; suffix.len() + 1];
    reachable[0] = true;
    for start in 0..suffix.len() {
        if !reachable[start] {
            continue;
        }
        resource.common_prefixes(&suffix.as_bytes()[start..], |length, analyses| {
            let Some(end) = start
                .checked_add(length)
                .filter(|&end| length > 0 && end <= suffix.len())
            else {
                return;
            };
            if analyses
                .iter()
                .any(|analysis| analysis.pos.split('+').all(accepts))
            {
                reachable[end] = true;
            }
        });
    }
    reachable[suffix.len()]
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
