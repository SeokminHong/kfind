use std::collections::BTreeSet;
use std::ops::Range;

use kfind_data::{DataFinePos, MorphologyGraphExpressionKind};

use crate::ContinuationState;

use super::paths::{Node, TokenGraph};
use super::{
    AdjacentSide, AdjacentTokenConstraint, CandidateSpans, ConstraintAmbiguity,
    ConstraintEvidenceKind, ConstraintNodeSource, ConstraintProof, ConstraintResolution,
    CopularFrameRole, MorphContinuation, QueryMorphPattern,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintOutcome {
    Supported,
    Contradicted,
    Ambiguous(ConstraintAmbiguity),
    Unavailable(super::ConstraintUnavailable),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConstraintSpanRelation {
    Whole,
    SourceComponent,
    RuntimeComponent,
    OpaqueExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintMorphUnitProof {
    pub pos: String,
    pub span: Option<Range<usize>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintContinuationProof {
    pub contract: MorphContinuation,
    pub units: Vec<ConstraintMorphUnitProof>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstraintContextProof {
    RepeatedToken {
        side: AdjacentSide,
    },
    CopularFrame {
        role: CopularFrameRole,
        selected: Range<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportedAnalysis {
    pub pattern_index: usize,
    pub path_index: usize,
    pub node_index: usize,
    pub component_index: Option<usize>,
    pub evidence: ConstraintEvidenceKind,
    pub span_relation: ConstraintSpanRelation,
    pub support_span: Range<usize>,
    pub continuation: ConstraintContinuationProof,
    pub context: Option<ConstraintContextProof>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SupportedAnalysisSet {
    pub analyses: Vec<SupportedAnalysis>,
}

impl SupportedAnalysisSet {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.analyses.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductPolicy {
    Whole,
    ExplicitComponent,
    PossibleAnalysis,
}

impl ProductPolicy {
    #[must_use]
    pub fn accepts(
        self,
        resolution: &ConstraintResolution,
        patterns: &[QueryMorphPattern],
    ) -> bool {
        resolution.supported.analyses.iter().any(|analysis| {
            let Some(pattern) = patterns.get(analysis.pattern_index) else {
                return false;
            };
            match self {
                Self::Whole => analysis.span_relation == ConstraintSpanRelation::Whole,
                Self::ExplicitComponent => match analysis.span_relation {
                    ConstraintSpanRelation::Whole => true,
                    ConstraintSpanRelation::SourceComponent => {
                        pattern.component_capability.allows_source()
                    }
                    ConstraintSpanRelation::RuntimeComponent => {
                        pattern.component_capability.allows_runtime()
                    }
                    ConstraintSpanRelation::OpaqueExpression => false,
                },
                Self::PossibleAnalysis => {
                    analysis.span_relation != ConstraintSpanRelation::OpaqueExpression
                }
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ContextSelection {
    None,
    Repeated {
        side: AdjacentSide,
    },
    Copular {
        nominal: Range<usize>,
        copula: Range<usize>,
    },
    Competing,
}

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

pub(super) fn select_context(
    context: BoundedTokenContext<'_>,
    current: &TokenGraph,
    previous: Option<&TokenGraph>,
    next: Option<&TokenGraph>,
) -> ContextSelection {
    let repeated = repeated_selection(context, current);
    let copular = previous
        .zip(next)
        .and_then(|(previous, next)| copular_selection(context.current, previous, current, next));
    match (repeated, copular) {
        (Some(side), None) => ContextSelection::Repeated { side },
        (None, Some((nominal, copula))) => ContextSelection::Copular { nominal, copula },
        (None, None) => ContextSelection::None,
        (Some(_), Some(_)) => ContextSelection::Competing,
    }
}

pub(super) fn resolve_known(
    graph: &TokenGraph,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    context: &ContextSelection,
) -> ConstraintResolution {
    if *context == ContextSelection::Competing {
        return ConstraintResolution {
            outcome: ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompetingAnalyses),
            supported: SupportedAnalysisSet::default(),
            proof: ConstraintProof {
                known_node_count: graph.node_count(),
                unknown_node_count: 0,
                paths: graph.proof_paths(),
            },
        };
    }
    let mut analyses = Vec::new();
    for (path_index, path) in graph.complete_paths().iter().enumerate() {
        let units = path_units(path, graph.nodes());
        for (pattern_index, pattern) in patterns.iter().enumerate() {
            for candidate in support_candidates(path, graph.nodes(), &units, spans, pattern) {
                let Some(continuation) = continuation_proof(pattern, spans, &units, &candidate)
                else {
                    continue;
                };
                let Some(context_proof) = context_proof(context, pattern, spans) else {
                    continue;
                };
                let analysis = SupportedAnalysis {
                    pattern_index,
                    path_index,
                    node_index: candidate.node_position,
                    component_index: candidate.component_index,
                    evidence: candidate.evidence,
                    span_relation: candidate.relation,
                    support_span: spans.core.clone(),
                    continuation,
                    context: context_proof,
                };
                if !analyses.contains(&analysis) {
                    analyses.push(analysis);
                }
            }
        }
    }
    analyses.sort_by_key(|analysis| {
        (
            analysis.path_index,
            analysis.pattern_index,
            analysis.node_index,
            analysis.component_index,
            analysis.span_relation,
        )
    });
    let mut proof_paths = graph.proof_paths();
    annotate_proof(&mut proof_paths, &analyses);
    let has_stable = analyses
        .iter()
        .any(|analysis| analysis.span_relation != ConstraintSpanRelation::OpaqueExpression);
    let has_opaque = analyses
        .iter()
        .any(|analysis| analysis.span_relation == ConstraintSpanRelation::OpaqueExpression);
    let outcome = if has_stable {
        ConstraintOutcome::Supported
    } else if has_opaque {
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::OpaqueExpression)
    } else {
        ConstraintOutcome::Contradicted
    };
    ConstraintResolution {
        outcome,
        supported: SupportedAnalysisSet { analyses },
        proof: ConstraintProof {
            known_node_count: graph.node_count(),
            unknown_node_count: 0,
            paths: proof_paths,
        },
    }
}

#[derive(Clone, Debug)]
struct Unit {
    node_position: usize,
    component_index: Option<usize>,
    pos: String,
    span: Option<Range<usize>>,
    coverage: Range<usize>,
}

#[derive(Clone, Copy, Debug)]
struct SupportCandidate {
    node_position: usize,
    component_index: Option<usize>,
    unit_index: usize,
    evidence: ConstraintEvidenceKind,
    relation: ConstraintSpanRelation,
}

fn support_candidates(
    path: &[usize],
    nodes: &[Node],
    units: &[Unit],
    spans: &CandidateSpans,
    pattern: &QueryMorphPattern,
) -> Vec<SupportCandidate> {
    let mut matches = Vec::new();
    for (node_position, &node_index) in path.iter().enumerate() {
        let node = &nodes[node_index];
        if node.source != ConstraintNodeSource::Source {
            continue;
        }
        if node.span == spans.core
            && node.surface == pattern.lexical_form.as_ref()
            && node
                .pos
                .split('+')
                .any(|pos| source_pos_matches(pos, pattern.fine_pos))
            && let Some(unit_index) = units
                .iter()
                .rposition(|unit| unit.node_position == node_position)
        {
            let relation = if path.len() == 1 && node.span == spans.token {
                ConstraintSpanRelation::Whole
            } else {
                ConstraintSpanRelation::RuntimeComponent
            };
            matches.push(SupportCandidate {
                node_position,
                component_index: None,
                unit_index,
                evidence: if relation == ConstraintSpanRelation::Whole {
                    ConstraintEvidenceKind::SourceWhole
                } else {
                    ConstraintEvidenceKind::RuntimeComposed
                },
                relation,
            });
        }
        for (component_index, component) in node.components.iter().enumerate() {
            if component.surface != pattern.lexical_form.as_ref()
                || !source_pos_matches(&component.pos, pattern.fine_pos)
            {
                continue;
            }
            let Some(unit_index) = units.iter().position(|unit| {
                unit.node_position == node_position && unit.component_index == Some(component_index)
            }) else {
                continue;
            };
            if component.span.as_ref() == Some(&spans.core) {
                matches.push(SupportCandidate {
                    node_position,
                    component_index: Some(component_index),
                    unit_index,
                    evidence: ConstraintEvidenceKind::SourceComponent,
                    relation: ConstraintSpanRelation::SourceComponent,
                });
            } else if component.span.is_none()
                && node.span.start <= spans.core.start
                && spans.core.end <= node.span.end
                && matches!(
                    node.expression_kind,
                    Some(
                        MorphologyGraphExpressionKind::Fused
                            | MorphologyGraphExpressionKind::Unaligned
                    )
                )
            {
                matches.push(SupportCandidate {
                    node_position,
                    component_index: Some(component_index),
                    unit_index,
                    evidence: ConstraintEvidenceKind::OpaqueExpression,
                    relation: ConstraintSpanRelation::OpaqueExpression,
                });
            }
        }
    }
    matches
}

fn continuation_proof(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &[Unit],
    support: &SupportCandidate,
) -> Option<ConstraintContinuationProof> {
    let suffix = spans.core.end..spans.consumed.end;
    let selected = suffix_units(units, support, &suffix)?;
    let positions = selected
        .iter()
        .map(|unit| unit.pos.as_str())
        .collect::<Vec<_>>();
    let accepted = match pattern.continuation {
        MorphContinuation::Exact => {
            spans.anchor == spans.core && spans.consumed == spans.anchor && positions.is_empty()
        }
        MorphContinuation::NominalParticles => {
            spans.consumed.end == spans.token.end && nominal_continuation(&positions)
        }
        MorphContinuation::Predicate {
            state,
            nominal_particles,
        } => {
            spans.consumed.end == spans.token.end
                && predicate_continuation(state, nominal_particles, spans, &selected, &positions)
        }
    };
    accepted.then(|| ConstraintContinuationProof {
        contract: pattern.continuation,
        units: selected
            .into_iter()
            .map(|unit| ConstraintMorphUnitProof {
                pos: unit.pos.clone(),
                span: unit.span.clone(),
            })
            .collect(),
    })
}

fn suffix_units<'a>(
    units: &'a [Unit],
    support: &SupportCandidate,
    suffix: &Range<usize>,
) -> Option<Vec<&'a Unit>> {
    if suffix.start == suffix.end {
        return Some(Vec::new());
    }
    let mut selected = Vec::new();
    for unit in units.iter().skip(support.unit_index + 1) {
        if unit.coverage.end <= suffix.start || unit.coverage.start >= suffix.end {
            continue;
        }
        if unit.span.is_none() && support.relation != ConstraintSpanRelation::OpaqueExpression {
            return None;
        }
        if support.relation != ConstraintSpanRelation::OpaqueExpression
            && (unit.coverage.start < suffix.start || unit.coverage.end > suffix.end)
        {
            return None;
        }
        selected.push(unit);
    }
    if support.relation == ConstraintSpanRelation::OpaqueExpression {
        return (!selected.is_empty()).then_some(selected);
    }
    let mut coverage = selected
        .iter()
        .map(|unit| unit.coverage.clone())
        .collect::<Vec<_>>();
    coverage.sort_by_key(|span| (span.start, span.end));
    coverage.dedup();
    let mut end = suffix.start;
    for span in coverage {
        if span.start > end {
            return None;
        }
        end = end.max(span.end);
    }
    (end == suffix.end).then_some(selected)
}

fn nominal_continuation(positions: &[&str]) -> bool {
    let mut particles = false;
    positions.iter().all(|pos| {
        if *pos == "XSN" && !particles {
            true
        } else if pos.starts_with('J') {
            particles = true;
            true
        } else {
            false
        }
    })
}

fn predicate_continuation(
    state: ContinuationState,
    nominal_particles: bool,
    spans: &CandidateSpans,
    units: &[&Unit],
    positions: &[&str],
) -> bool {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Stage {
        Predicate,
        Nominalized,
        Terminal,
    }
    let mut stage = Stage::Predicate;
    for pos in positions {
        match *pos {
            "EP" | "EC" | "VX" | "XSV" | "XSA" if stage == Stage::Predicate => {}
            "EF" | "ETM" if stage == Stage::Predicate => stage = Stage::Terminal,
            "ETN" if stage == Stage::Predicate => stage = Stage::Nominalized,
            pos if pos.starts_with('J') && nominal_particles && stage == Stage::Nominalized => {}
            _ => return false,
        }
    }
    let post_anchor = units
        .iter()
        .filter(|unit| unit.coverage.start >= spans.anchor.end)
        .map(|unit| unit.pos.as_str())
        .collect::<Vec<_>>();
    match state {
        ContinuationState::Terminal if nominal_particles => {
            post_anchor.iter().all(|pos| pos.starts_with('J'))
        }
        ContinuationState::Terminal => post_anchor.is_empty(),
        ContinuationState::Eu => !post_anchor.is_empty(),
        ContinuationState::AOrEo
        | ContinuationState::Past
        | ContinuationState::Future
        | ContinuationState::Declarative => true,
    }
}

fn context_proof(
    selection: &ContextSelection,
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
) -> Option<Option<ConstraintContextProof>> {
    match selection {
        ContextSelection::None => Some(None),
        ContextSelection::Competing => None,
        ContextSelection::Repeated { side } => {
            pattern
                .adjacent
                .iter()
                .find_map(|constraint| match constraint {
                    AdjacentTokenConstraint::RepeatedToken { side: required }
                        if side_matches(*required, *side) && spans.core == spans.token =>
                    {
                        Some(Some(ConstraintContextProof::RepeatedToken { side: *side }))
                    }
                    _ => None,
                })
        }
        ContextSelection::Copular { nominal, copula } => {
            pattern
                .adjacent
                .iter()
                .find_map(|constraint| match constraint {
                    AdjacentTokenConstraint::CopularFrame {
                        role: CopularFrameRole::Nominal,
                    } if spans.core == *nominal => {
                        Some(Some(ConstraintContextProof::CopularFrame {
                            role: CopularFrameRole::Nominal,
                            selected: nominal.clone(),
                        }))
                    }
                    AdjacentTokenConstraint::CopularFrame {
                        role: CopularFrameRole::Copula,
                    } if spans.core == *copula => {
                        Some(Some(ConstraintContextProof::CopularFrame {
                            role: CopularFrameRole::Copula,
                            selected: copula.clone(),
                        }))
                    }
                    _ => None,
                })
        }
    }
}

fn side_matches(required: AdjacentSide, actual: AdjacentSide) -> bool {
    required == AdjacentSide::Either || actual == AdjacentSide::Either || required == actual
}

fn repeated_selection(
    context: BoundedTokenContext<'_>,
    current: &TokenGraph,
) -> Option<AdjacentSide> {
    if !has_exact_pos(current, "MAG") {
        return None;
    }
    match (
        context.previous == Some(context.current),
        context.next == Some(context.current),
    ) {
        (true, true) => Some(AdjacentSide::Either),
        (true, false) => Some(AdjacentSide::Previous),
        (false, true) => Some(AdjacentSide::Next),
        (false, false) => None,
    }
}

fn copular_selection(
    current_text: &str,
    previous: &TokenGraph,
    current: &TokenGraph,
    next: &TokenGraph,
) -> Option<(Range<usize>, Range<usize>)> {
    if !has_pos_sequence(previous, &["VCN", "EC"])
        || !starts_with_pos(next, |pos| matches!(pos, "NNB" | "NNBC"))
    {
        return None;
    }
    let mut splits = BTreeSet::new();
    for path in current.complete_paths() {
        let units = path_units(path, current.nodes());
        for (split, _) in current_text.char_indices().skip(1) {
            let prefix = units
                .iter()
                .filter(|unit| unit.coverage.end <= split)
                .collect::<Vec<_>>();
            let suffix = units
                .iter()
                .filter(|unit| unit.coverage.start >= split)
                .collect::<Vec<_>>();
            if prefix.len() == 1
                && source_pos(prefix[0].pos.as_str()).is_some_and(DataFinePos::is_nominal)
                && exact_coverage(&prefix, 0..split)
                && suffix
                    .iter()
                    .map(|unit| unit.pos.as_str())
                    .eq(["VCP", "ETM"])
                && exact_coverage(&suffix, split..current_text.len())
            {
                splits.insert(split);
            }
        }
    }
    (splits.len() == 1).then(|| {
        let split = *splits.first().expect("single copular split");
        (0..split, split..current_text.len())
    })
}

fn has_exact_pos(graph: &TokenGraph, expected: &str) -> bool {
    graph.complete_paths().iter().any(|path| {
        path.len() == 1
            && graph.nodes()[path[0]]
                .pos
                .split('+')
                .eq(std::iter::once(expected))
    })
}

fn has_pos_sequence(graph: &TokenGraph, expected: &[&str]) -> bool {
    graph.complete_paths().iter().any(|path| {
        path.iter()
            .flat_map(|&index| graph.nodes()[index].pos.split('+'))
            .eq(expected.iter().copied())
    })
}

fn starts_with_pos(graph: &TokenGraph, accepts: impl Fn(&str) -> bool) -> bool {
    graph.complete_paths().iter().any(|path| {
        path.first()
            .and_then(|&index| graph.nodes()[index].pos.split('+').next())
            .is_some_and(&accepts)
    })
}

fn path_units(path: &[usize], nodes: &[Node]) -> Vec<Unit> {
    let mut units = Vec::new();
    for (node_position, &node_index) in path.iter().enumerate() {
        let node = &nodes[node_index];
        if node.components.is_empty() {
            units.extend(node.pos.split('+').map(|pos| Unit {
                node_position,
                component_index: None,
                pos: pos.to_owned(),
                span: Some(node.span.clone()),
                coverage: node.span.clone(),
            }));
        } else {
            units.extend(
                node.components
                    .iter()
                    .enumerate()
                    .map(|(component_index, component)| Unit {
                        node_position,
                        component_index: Some(component_index),
                        pos: component.pos.clone(),
                        span: component.span.clone(),
                        coverage: component.span.clone().unwrap_or_else(|| node.span.clone()),
                    }),
            );
        }
    }
    units
}

fn exact_coverage(units: &[&Unit], expected: Range<usize>) -> bool {
    let mut spans = units
        .iter()
        .filter_map(|unit| unit.span.clone())
        .collect::<Vec<_>>();
    if spans.len() != units.len() {
        return false;
    }
    spans.sort_by_key(|span| (span.start, span.end));
    spans.dedup();
    let mut end = expected.start;
    for span in spans {
        if span.start > end || span.start < expected.start || span.end > expected.end {
            return false;
        }
        end = end.max(span.end);
    }
    end == expected.end
}

fn source_pos_matches(source: &str, query: DataFinePos) -> bool {
    source_pos(source) == Some(query) || (source == "NNBC" && query == DataFinePos::Nnb)
}

fn source_pos(source: &str) -> Option<DataFinePos> {
    DataFinePos::parse(source)
}

fn annotate_proof(paths: &mut [super::ConstraintPathProof], analyses: &[SupportedAnalysis]) {
    for analysis in analyses {
        let Some(path) = paths.get_mut(analysis.path_index) else {
            continue;
        };
        path.evidence = evidence_preference(path.evidence, analysis.evidence);
        let Some(node) = path.nodes.get_mut(analysis.node_index) else {
            continue;
        };
        match analysis.span_relation {
            ConstraintSpanRelation::Whole | ConstraintSpanRelation::RuntimeComponent => {
                node.matches_query_node = true;
            }
            ConstraintSpanRelation::SourceComponent => node.matches_source_component = true,
            ConstraintSpanRelation::OpaqueExpression => node.has_opaque_expression = true,
        }
    }
}

fn evidence_preference(
    current: ConstraintEvidenceKind,
    candidate: ConstraintEvidenceKind,
) -> ConstraintEvidenceKind {
    fn rank(evidence: ConstraintEvidenceKind) -> u8 {
        match evidence {
            ConstraintEvidenceKind::SourceWhole => 5,
            ConstraintEvidenceKind::SourceComponent => 4,
            ConstraintEvidenceKind::RuntimeComposed => 3,
            ConstraintEvidenceKind::OpaqueExpression => 2,
            ConstraintEvidenceKind::Contradiction => 1,
            ConstraintEvidenceKind::Unknown => 0,
        }
    }
    if rank(candidate) > rank(current) {
        candidate
    } else {
        current
    }
}
