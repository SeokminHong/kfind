use std::ops::Range;
use std::sync::Arc;

use kfind_data::{ComponentResource, DataFinePos};

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
}

impl ConstraintResolver {
    #[must_use]
    pub fn new(resource: Arc<ComponentResource>) -> Self {
        Self { resource }
    }

    #[must_use]
    pub fn resource(&self) -> &ComponentResource {
        &self.resource
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> ConstraintDecision {
        if !spans.is_valid_for(context.current)
            || spans.token != (0..context.current.len())
            || patterns.iter().any(|pattern| !pattern.is_well_formed())
        {
            return ConstraintDecision::unavailable(ConstraintUnavailable::InvalidSpans);
        }
        let evidence = match TokenEvidence::collect(&self.resource, context.current, node_limit) {
            Ok(evidence) => evidence,
            Err(reason) => return ConstraintDecision::unavailable(reason),
        };
        let raw = collect_pattern_supports(&evidence, &spans, patterns);
        if raw.is_empty() {
            return ConstraintDecision {
                outcome: ConstraintOutcome::Contradicted,
                supported: Vec::new(),
            };
        }
        let selection = select_structure(&self.resource, context, &evidence);
        let mut supported = raw
            .into_iter()
            .filter(|support| selection.accepts(support, &spans, patterns))
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Unit {
    span: Range<usize>,
    pos: DataFinePos,
    evidence: StructuralEvidence,
}

#[derive(Debug, Default)]
struct TokenEvidence {
    units: Vec<Unit>,
    runtime_spans: Vec<Range<usize>>,
    has_complete_path: bool,
}

impl TokenEvidence {
    fn collect(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
    ) -> Result<Self, ConstraintUnavailable> {
        let mut edges = Vec::new();
        for start in text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(text.len()))
        {
            if start == text.len() {
                continue;
            }
            resource.common_prefixes(&text.as_bytes()[start..], |length, analyses| {
                if length == 0 || start + length > text.len() {
                    return;
                }
                for analysis in analyses {
                    edges.push(Edge {
                        span: start..start + length,
                        pos: analysis.pos.to_owned(),
                        components: analysis
                            .components
                            .iter()
                            .map(|component| OwnedComponent {
                                span: component.span.clone(),
                                pos: component.pos.to_owned(),
                            })
                            .collect(),
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
        let forward = forward_positions(text.len(), &edges);
        let complete = complete_edges(text.len(), &edges, &forward);
        let has_complete_path = forward[text.len()];
        let mut units = Vec::new();
        let mut runtime_spans = Vec::new();
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
            let whole_edge = edge.span == (0..text.len());
            if let Some(pos) = DataFinePos::parse(&edge.pos) {
                units.push(Unit {
                    span: edge.span.clone(),
                    pos,
                    evidence: if whole_edge {
                        StructuralEvidence::Whole
                    } else {
                        StructuralEvidence::RuntimeComponent
                    },
                });
            }
            for component in &edge.components {
                let Some(pos) = DataFinePos::parse(&component.pos) else {
                    continue;
                };
                units.push(Unit {
                    span: edge.span.start + component.span.start
                        ..edge.span.start + component.span.end,
                    pos,
                    evidence: StructuralEvidence::SourceComponent,
                });
            }
        }
        units.sort_unstable_by_key(|unit| {
            (
                unit.span.start,
                unit.span.end,
                unit.pos,
                unit.evidence as u8,
            )
        });
        units.dedup();
        runtime_spans.sort_unstable_by_key(|span| (span.start, span.end));
        runtime_spans.dedup();
        Ok(Self {
            units,
            runtime_spans,
            has_complete_path,
        })
    }

    fn has_whole(&self, pos: DataFinePos) -> bool {
        self.units
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos == pos)
    }
}

#[derive(Debug)]
struct Edge {
    span: Range<usize>,
    pos: String,
    components: Vec<OwnedComponent>,
}

#[derive(Debug)]
struct OwnedComponent {
    span: Range<usize>,
    pos: String,
}

fn forward_positions(text_len: usize, edges: &[Edge]) -> Vec<bool> {
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

fn complete_edges(text_len: usize, edges: &[Edge], forward: &[bool]) -> Vec<bool> {
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
        if supports.len() == support_start
            && pattern.component_capability.allows_runtime()
            && (evidence.runtime_spans.contains(&spans.core)
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
    All,
    Whole,
    RepeatedAdverb,
    NominalSpan {
        selected: Range<usize>,
        allow_components: bool,
    },
    CopularFrame {
        nominal: Range<usize>,
        copula: Range<usize>,
    },
}

impl StructureSelection {
    fn accepts(
        &self,
        support: &ConstraintSupport,
        spans: &CandidateSpans,
        patterns: &[QueryMorphPattern],
    ) -> bool {
        let Some(pattern) = patterns.get(support.pattern_index) else {
            return false;
        };
        match self {
            Self::All => true,
            Self::Whole => support.evidence == StructuralEvidence::Whole,
            Self::RepeatedAdverb => {
                support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Mag
            }
            Self::NominalSpan {
                selected,
                allow_components,
            } => {
                (spans.core == *selected
                    || (*allow_components
                        && spans.core.start >= selected.start
                        && spans.core.end <= selected.end))
                    && pattern.fine_pos.is_nominal()
            }
            Self::CopularFrame { nominal, copula } => {
                (spans.core == *nominal && pattern.fine_pos.is_nominal())
                    || (spans.core == *copula && pattern.fine_pos == DataFinePos::Vcp)
            }
        }
    }
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
    if let Some((nominal, copula)) = copular_frame(resource, context) {
        return StructureSelection::CopularFrame { nominal, copula };
    }
    if let Some(host) = nominal_particle_host(resource, context.current) {
        let allow_components =
            unique_copular_split(resource, &context.current[host.clone()]).is_none();
        return StructureSelection::NominalSpan {
            selected: host,
            allow_components,
        };
    }
    if evidence
        .units
        .iter()
        .any(|unit| unit.evidence == StructuralEvidence::Whole)
        && unique_copular_split(resource, context.current).is_some()
    {
        StructureSelection::Whole
    } else {
        StructureSelection::All
    }
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
