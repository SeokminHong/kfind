use std::collections::BTreeSet;
use std::ops::Range;

use kfind_data::{DataFinePos, MorphologyGraphExpressionKind};

use crate::ContinuationState;

use super::paths::{Node, TokenGraph};
use super::{
    AdjacentSide, AdjacentTokenConstraint, CandidateSpans, ConstraintAmbiguity,
    ConstraintEvidenceKind, ConstraintNodeSource, ConstraintProof, ConstraintResolution,
    ConstraintUnavailable, CopularFrameRole, MorphContinuation, QueryMorphPattern,
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
    pub source_node_index: usize,
    pub component_index: Option<usize>,
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
    NominalParticleHost {
        selected: Range<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportedAnalysis {
    pub pattern_index: usize,
    pub path_index: usize,
    pub node_index: usize,
    pub source_node_index: usize,
    pub lexical_node_indices: Vec<usize>,
    pub lexical_source_node_indices: Vec<usize>,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConstraintSupport {
    pub pattern_index: usize,
    pub evidence: ConstraintEvidenceKind,
    pub span_relation: ConstraintSpanRelation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintDecision {
    pub outcome: ConstraintOutcome,
    pub supported: Vec<ConstraintSupport>,
}

impl ConstraintDecision {
    pub(super) fn from_resolution(resolution: &ConstraintResolution) -> Self {
        Self::from_analyses(resolution.outcome, &resolution.supported.analyses)
    }

    fn from_analyses(outcome: ConstraintOutcome, analyses: &[SupportedAnalysis]) -> Self {
        Self::from_supports(
            outcome,
            analyses.iter().map(|analysis| ConstraintSupport {
                pattern_index: analysis.pattern_index,
                evidence: analysis.evidence,
                span_relation: analysis.span_relation,
            }),
        )
    }

    fn from_supports(
        outcome: ConstraintOutcome,
        supports: impl IntoIterator<Item = ConstraintSupport>,
    ) -> Self {
        let mut supported = supports.into_iter().collect::<Vec<_>>();
        supported.sort_unstable();
        supported.dedup();
        Self { outcome, supported }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductPolicy {
    Whole,
    ExplicitComponent,
    PossibleAnalysis,
    UnambiguousAnalysis,
}

impl ProductPolicy {
    #[must_use]
    pub fn accepts(
        self,
        resolution: &ConstraintResolution,
        patterns: &[QueryMorphPattern],
    ) -> bool {
        self.accepts_supports(
            resolution.outcome,
            resolution
                .supported
                .analyses
                .iter()
                .map(|analysis| (analysis.pattern_index, analysis.span_relation)),
            patterns,
        )
    }

    #[must_use]
    pub fn accepts_decision(
        self,
        decision: &ConstraintDecision,
        patterns: &[QueryMorphPattern],
    ) -> bool {
        self.accepts_supports(
            decision.outcome,
            decision
                .supported
                .iter()
                .map(|support| (support.pattern_index, support.span_relation)),
            patterns,
        )
    }

    fn accepts_supports(
        self,
        outcome: ConstraintOutcome,
        supports: impl Iterator<Item = (usize, ConstraintSpanRelation)>,
        patterns: &[QueryMorphPattern],
    ) -> bool {
        if self == Self::UnambiguousAnalysis && outcome != ConstraintOutcome::Supported {
            return false;
        }
        supports.into_iter().any(|(pattern_index, relation)| {
            let Some(pattern) = patterns.get(pattern_index) else {
                return false;
            };
            match self {
                Self::Whole => relation == ConstraintSpanRelation::Whole,
                Self::ExplicitComponent => match relation {
                    ConstraintSpanRelation::Whole => true,
                    ConstraintSpanRelation::SourceComponent => {
                        pattern.component_capability.allows_source()
                    }
                    ConstraintSpanRelation::RuntimeComponent => {
                        pattern.component_capability.allows_runtime()
                    }
                    ConstraintSpanRelation::OpaqueExpression => false,
                },
                Self::PossibleAnalysis | Self::UnambiguousAnalysis => {
                    relation != ConstraintSpanRelation::OpaqueExpression
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
    NominalParticleHost {
        selected: Range<usize>,
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
    current: &TokenGraph<'_>,
    previous: Option<&TokenGraph<'_>>,
    next: Option<&TokenGraph<'_>>,
) -> ContextSelection {
    let repeated = repeated_selection(context, current);
    let copular = previous
        .zip(next)
        .and_then(|(previous, next)| copular_selection(context.current, previous, current, next));
    let particle_host = nominal_particle_host_selection(context.current, current);
    match (
        repeated.is_some(),
        copular.is_some(),
        particle_host.is_some(),
    ) {
        (false, false, false) => ContextSelection::None,
        (true, false, false) => ContextSelection::Repeated {
            side: repeated.expect("present repeated selection"),
        },
        (false, true, false) => {
            let (nominal, copula) = copular.expect("present copular selection");
            ContextSelection::Copular { nominal, copula }
        }
        (false, false, true) => ContextSelection::NominalParticleHost {
            selected: particle_host.expect("present particle host selection"),
        },
        _ => ContextSelection::Competing,
    }
}

pub(super) fn needs_copular_context(
    current: &TokenGraph<'_>,
    patterns: &[QueryMorphPattern],
) -> bool {
    let requested = patterns.iter().any(|pattern| {
        pattern
            .adjacent
            .iter()
            .any(|constraint| matches!(constraint, AdjacentTokenConstraint::CopularFrame { .. }))
    });
    requested && has_complete_pos(current, "VCP") && has_complete_pos(current, "ETM")
}

pub(super) fn resolve_known(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    context: &ContextSelection,
    proof_limit: usize,
) -> ConstraintResolution {
    let evaluation = evaluate_known(graph, spans, patterns, context, proof_limit);
    let mut proof_paths = evaluation.paths.as_ref().map_or_else(
        || graph.proof_paths(),
        |paths| {
            let proofs = path_proofs(graph, paths);
            if proofs.is_empty() {
                graph.proof_paths()
            } else {
                proofs
            }
        },
    );
    let mut proof_nodes = graph.proof_nodes();
    annotate_proof(&mut proof_nodes, &mut proof_paths, &evaluation.analyses);
    ConstraintResolution {
        outcome: evaluation.outcome,
        supported: SupportedAnalysisSet {
            analyses: evaluation.analyses,
        },
        proof: ConstraintProof {
            known_node_count: graph.node_count(),
            unknown_node_count: 0,
            nodes: proof_nodes,
            paths: proof_paths,
        },
    }
}

pub(super) fn decide_known(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    context: &ContextSelection,
    proof_limit: usize,
) -> ConstraintDecision {
    if *context == ContextSelection::Competing {
        return ConstraintDecision {
            outcome: ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompetingAnalyses),
            supported: Vec::new(),
        };
    }
    let mut proofs = BTreeSet::<CompactSupportProof>::new();
    for path in candidate_decision_paths(graph, spans, patterns) {
        let units = path_units(&path, graph.nodes());
        for (pattern_index, pattern) in patterns.iter().enumerate() {
            for candidate in support_candidates(&path, graph.nodes(), &units, spans, pattern) {
                let Some(continuation) = continuation_units(pattern, spans, &units, &candidate)
                else {
                    continue;
                };
                if context_match(context, pattern, spans).is_none() {
                    continue;
                }
                let proof = CompactSupportProof {
                    support: ConstraintSupport {
                        pattern_index,
                        evidence: candidate.evidence,
                        span_relation: candidate.relation,
                    },
                    source_node_index: candidate.source_node_index,
                    lexical_source_node_indices: candidate.source_node_indices,
                    component_index: candidate.component_index,
                    continuation: continuation
                        .into_iter()
                        .map(|unit| CompactMorphUnit {
                            pos_slot: unit.pos_slot,
                            span: unit.span.as_ref().map(|span| (span.start, span.end)),
                            source_node_index: unit.source_node_index,
                            component_index: unit.component_index,
                        })
                        .collect(),
                };
                if proofs.insert(proof) && proofs.len() > proof_limit {
                    return ConstraintDecision {
                        outcome: ConstraintOutcome::Unavailable(ConstraintUnavailable::PathLimit {
                            actual: proofs.len(),
                            limit: proof_limit,
                        }),
                        supported: Vec::new(),
                    };
                }
            }
        }
    }
    let outcome = support_outcome(
        graph,
        spans,
        patterns,
        context,
        proofs.iter().map(|proof| proof.support.span_relation),
    );
    ConstraintDecision::from_supports(outcome, proofs.into_iter().map(|proof| proof.support))
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct CompactMorphUnit {
    pos_slot: usize,
    span: Option<(usize, usize)>,
    source_node_index: usize,
    component_index: Option<usize>,
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct CompactSupportProof {
    support: ConstraintSupport,
    source_node_index: usize,
    lexical_source_node_indices: Vec<usize>,
    component_index: Option<usize>,
    continuation: Vec<CompactMorphUnit>,
}

struct KnownEvaluation {
    outcome: ConstraintOutcome,
    analyses: Vec<SupportedAnalysis>,
    paths: Option<Vec<Vec<usize>>>,
}

fn evaluate_known(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    context: &ContextSelection,
    proof_limit: usize,
) -> KnownEvaluation {
    if *context == ContextSelection::Competing {
        return KnownEvaluation {
            outcome: ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompetingAnalyses),
            analyses: Vec::new(),
            paths: None,
        };
    }
    let mut analyses = Vec::new();
    let paths = candidate_witness_paths(graph, spans, patterns);
    for (path_index, path) in paths.iter().enumerate() {
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
                    source_node_index: candidate.source_node_index,
                    lexical_node_indices: candidate.lexical_node_positions.clone(),
                    lexical_source_node_indices: candidate.source_node_indices.clone(),
                    component_index: candidate.component_index,
                    evidence: candidate.evidence,
                    span_relation: candidate.relation,
                    support_span: spans.core.clone(),
                    continuation,
                    context: context_proof,
                };
                if !analyses
                    .iter()
                    .any(|existing| same_supported_analysis(existing, &analysis))
                {
                    analyses.push(analysis);
                    if analyses.len() > proof_limit {
                        return KnownEvaluation {
                            outcome: ConstraintOutcome::Unavailable(
                                ConstraintUnavailable::PathLimit {
                                    actual: analyses.len(),
                                    limit: proof_limit,
                                },
                            ),
                            analyses: Vec::new(),
                            paths: None,
                        };
                    }
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
    let outcome = support_outcome(
        graph,
        spans,
        patterns,
        context,
        analyses.iter().map(|analysis| analysis.span_relation),
    );
    KnownEvaluation {
        outcome,
        analyses,
        paths: Some(paths),
    }
}

fn support_outcome(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    context: &ContextSelection,
    relations: impl IntoIterator<Item = ConstraintSpanRelation>,
) -> ConstraintOutcome {
    let mut has_stable = false;
    let mut has_opaque = false;
    let mut all_non_whole = true;
    for relation in relations {
        has_stable |= relation != ConstraintSpanRelation::OpaqueExpression;
        has_opaque |= relation == ConstraintSpanRelation::OpaqueExpression;
        all_non_whole &= relation != ConstraintSpanRelation::Whole;
    }
    let context_resolved = *context != ContextSelection::None;
    let has_compound_exposure =
        has_stable && !context_resolved && spans.consumed != spans.token && all_non_whole;
    let has_lexical_competition =
        has_stable && !context_resolved && lexical_competition(graph, spans, patterns);
    if has_compound_exposure {
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    } else if has_lexical_competition {
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::LexicalCompetition)
    } else if has_stable {
        ConstraintOutcome::Supported
    } else if has_opaque {
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::OpaqueExpression)
    } else {
        ConstraintOutcome::Contradicted
    }
}

fn candidate_witness_paths(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    let lexical_paths = lexical_candidate_paths(graph, spans, patterns);
    for pattern in patterns {
        for lexical_path in &lexical_paths {
            let units = path_units(lexical_path, graph.nodes());
            for support in support_candidates(lexical_path, graph.nodes(), &units, spans, pattern) {
                extend_supported_path(
                    graph,
                    spans,
                    pattern,
                    lexical_path.clone(),
                    support,
                    &mut paths,
                );
            }
        }
    }
    paths
}

fn candidate_decision_paths(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
) -> BTreeSet<Vec<usize>> {
    let mut paths = BTreeSet::new();
    let lexical_paths = lexical_candidate_paths(graph, spans, patterns);
    for pattern in patterns {
        for lexical_path in &lexical_paths {
            let units = path_units(lexical_path, graph.nodes());
            for support in support_candidates(lexical_path, graph.nodes(), &units, spans, pattern) {
                extend_decision_path(
                    graph,
                    spans,
                    pattern,
                    lexical_path.clone(),
                    support,
                    &mut paths,
                );
            }
        }
    }
    paths
}

fn lexical_candidate_paths(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    for (index, node) in graph.nodes().iter().enumerate() {
        if !graph.is_on_complete_path(index) {
            continue;
        }
        if node.span.start <= spans.core.start && spans.core.end <= node.span.end {
            paths.push(vec![index]);
        }
        if node.span.start == spans.core.start && node.span.end <= spans.core.end {
            extend_lexical_path(graph, spans, patterns, vec![index], &mut paths);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn extend_lexical_path(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    required: Vec<usize>,
    paths: &mut Vec<Vec<usize>>,
) {
    let last = *required.last().expect("lexical path is non-empty");
    let end = graph.nodes()[last].span.end;
    if end == spans.core.end {
        paths.push(required);
        return;
    }
    if end > spans.core.end || !lexical_path_can_match(graph, &required, patterns) {
        return;
    }
    for &successor in graph.successors(last) {
        let next = &graph.nodes()[successor];
        if next.span.end > spans.core.end || !graph.is_on_complete_path(successor) {
            continue;
        }
        let mut extended = required.clone();
        extended.push(successor);
        extend_lexical_path(graph, spans, patterns, extended, paths);
    }
}

fn lexical_path_can_match(
    graph: &TokenGraph<'_>,
    path: &[usize],
    patterns: &[QueryMorphPattern],
) -> bool {
    let surface = path
        .iter()
        .map(|&index| graph.nodes()[index].surface)
        .collect::<String>();
    let canonical = path
        .iter()
        .flat_map(|&index| graph.nodes()[index].components.iter())
        .map(|component| component.surface)
        .collect::<String>();
    patterns.iter().any(|pattern| {
        pattern.lexical_form.starts_with(&surface)
            || (!canonical.is_empty() && pattern.lexical_form.starts_with(&canonical))
    })
}

fn extend_supported_path(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    pattern: &QueryMorphPattern,
    required: Vec<usize>,
    support: SupportCandidate,
    paths: &mut Vec<Vec<usize>>,
) {
    let last = *required.last().expect("supported path is non-empty");
    let units = path_units(&required, graph.nodes());
    if graph.nodes()[last].span.end >= spans.consumed.end {
        if continuation_proof(pattern, spans, &units, &support).is_none() {
            return;
        }
        if let Some(path) = graph.witness_path_through(&required)
            && !paths.contains(&path)
        {
            paths.push(path);
        }
        return;
    }
    if !continuation_prefix_possible(pattern, spans, &units, &support) {
        return;
    }
    for &successor in graph.successors(last) {
        let next = &graph.nodes()[successor];
        if next.span.end > spans.consumed.end || !graph.is_on_complete_path(successor) {
            continue;
        }
        let mut extended = required.clone();
        extended.push(successor);
        extend_supported_path(graph, spans, pattern, extended, support.clone(), paths);
    }
}

fn extend_decision_path(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    pattern: &QueryMorphPattern,
    required: Vec<usize>,
    support: SupportCandidate,
    paths: &mut BTreeSet<Vec<usize>>,
) {
    let last = *required.last().expect("supported path is non-empty");
    let units = path_units(&required, graph.nodes());
    if graph.nodes()[last].span.end >= spans.consumed.end {
        if continuation_units(pattern, spans, &units, &support).is_some() {
            paths.insert(required);
        }
        return;
    }
    if !continuation_prefix_possible(pattern, spans, &units, &support) {
        return;
    }
    for &successor in graph.successors(last) {
        let next = &graph.nodes()[successor];
        if next.span.end > spans.consumed.end || !graph.is_on_complete_path(successor) {
            continue;
        }
        let mut extended = required.clone();
        extended.push(successor);
        extend_decision_path(graph, spans, pattern, extended, support.clone(), paths);
    }
}

fn continuation_prefix_possible(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &[Unit<'_>],
    support: &SupportCandidate,
) -> bool {
    let partial_end = units
        .iter()
        .skip(support.unit_index + 1)
        .map(|unit| unit.coverage.end)
        .max()
        .unwrap_or(spans.core.end)
        .min(spans.consumed.end);
    let Some(selected) = suffix_units(units, support, &(spans.core.end..partial_end)) else {
        return false;
    };
    let positions = selected.iter().map(|unit| unit.pos).collect::<Vec<_>>();
    match pattern.continuation {
        MorphContinuation::Exact => selected.is_empty(),
        MorphContinuation::NominalParticles => nominal_prefix(&positions),
        MorphContinuation::Predicate { .. } => predicate_prefix(&positions),
    }
}

fn nominal_prefix(positions: &[&str]) -> bool {
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

fn predicate_prefix(positions: &[&str]) -> bool {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Stage {
        Predicate,
        Nominalized,
        Terminal,
    }
    let mut stage = Stage::Predicate;
    positions.iter().all(|pos| match *pos {
        "EP" | "EC" | "VX" | "XSV" | "XSA" if stage == Stage::Predicate => true,
        "EF" | "ETM" if stage == Stage::Predicate => {
            stage = Stage::Terminal;
            true
        }
        "ETN" if stage == Stage::Predicate => {
            stage = Stage::Nominalized;
            true
        }
        pos if pos.starts_with('J') && stage == Stage::Nominalized => true,
        _ => false,
    })
}

fn path_proofs(graph: &TokenGraph<'_>, paths: &[Vec<usize>]) -> Vec<super::ConstraintPathProof> {
    paths
        .iter()
        .map(|path| super::ConstraintPathProof {
            evidence: if path
                .iter()
                .any(|&index| graph.nodes()[index].source == ConstraintNodeSource::Unknown)
            {
                ConstraintEvidenceKind::Unknown
            } else {
                ConstraintEvidenceKind::Contradiction
            },
            node_indices: path.clone(),
        })
        .collect()
}

fn lexical_competition(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
) -> bool {
    graph.nodes().iter().enumerate().any(|(index, node)| {
        graph.is_on_complete_path(index)
            && node.source == ConstraintNodeSource::Source
            && node.span == spans.token
            && (node.pos.split('+').any(|pos| {
                source_pos(pos).is_some_and(|fine_pos| {
                    !fine_pos.is_particle()
                        && !patterns.iter().any(|pattern| pattern.fine_pos == fine_pos)
                })
            }) || node.components.iter().any(|component| {
                source_pos(component.pos).is_some_and(|fine_pos| {
                    !fine_pos.is_particle()
                        && patterns.iter().any(|pattern| pattern.fine_pos == fine_pos)
                        && !patterns.iter().any(|pattern| {
                            pattern.fine_pos == fine_pos
                                && pattern.lexical_form.as_ref() == component.surface
                        })
                })
            }))
    })
}

fn same_supported_analysis(left: &SupportedAnalysis, right: &SupportedAnalysis) -> bool {
    left.pattern_index == right.pattern_index
        && left.source_node_index == right.source_node_index
        && left.lexical_source_node_indices == right.lexical_source_node_indices
        && left.component_index == right.component_index
        && left.evidence == right.evidence
        && left.span_relation == right.span_relation
        && left.support_span == right.support_span
        && left.continuation == right.continuation
        && left.context == right.context
}

#[derive(Clone, Debug)]
struct Unit<'a> {
    node_position: usize,
    source_node_index: usize,
    component_index: Option<usize>,
    surface: &'a str,
    pos: &'a str,
    pos_slot: usize,
    span: Option<Range<usize>>,
    coverage: Range<usize>,
    opaque: bool,
}

#[derive(Clone, Debug)]
struct SupportCandidate {
    node_position: usize,
    source_node_index: usize,
    lexical_node_positions: Vec<usize>,
    source_node_indices: Vec<usize>,
    component_index: Option<usize>,
    unit_index: usize,
    evidence: ConstraintEvidenceKind,
    relation: ConstraintSpanRelation,
    opaque: bool,
}

fn support_candidates(
    path: &[usize],
    nodes: &[Node<'_>],
    units: &[Unit<'_>],
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
                source_node_index: node_index,
                lexical_node_positions: vec![node_position],
                source_node_indices: vec![node_index],
                component_index: None,
                unit_index,
                evidence: if relation == ConstraintSpanRelation::Whole {
                    ConstraintEvidenceKind::SourceWhole
                } else {
                    ConstraintEvidenceKind::RuntimeComposed
                },
                relation,
                opaque: false,
            });
        }
        for (component_index, component) in node.components.iter().enumerate() {
            if component.surface != pattern.lexical_form.as_ref()
                || !source_pos_matches(component.pos, pattern.fine_pos)
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
                    source_node_index: node_index,
                    lexical_node_positions: vec![node_position],
                    source_node_indices: vec![node_index],
                    component_index: Some(component_index),
                    unit_index,
                    evidence: ConstraintEvidenceKind::SourceComponent,
                    relation: ConstraintSpanRelation::SourceComponent,
                    opaque: false,
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
                let enclosing_span_is_returned = spans.anchor.start <= node.span.start
                    && node.span.end <= spans.anchor.end
                    && spans.consumed.start <= node.span.start
                    && node.span.end <= spans.consumed.end;
                matches.push(SupportCandidate {
                    node_position,
                    source_node_index: node_index,
                    lexical_node_positions: vec![node_position],
                    source_node_indices: vec![node_index],
                    component_index: Some(component_index),
                    unit_index,
                    evidence: ConstraintEvidenceKind::OpaqueExpression,
                    relation: if enclosing_span_is_returned {
                        ConstraintSpanRelation::RuntimeComponent
                    } else {
                        ConstraintSpanRelation::OpaqueExpression
                    },
                    opaque: true,
                });
            }
        }
    }
    matches.extend(runtime_lexical_candidates(units, spans, pattern));
    matches
}

fn runtime_lexical_candidates(
    units: &[Unit<'_>],
    spans: &CandidateSpans,
    pattern: &QueryMorphPattern,
) -> Vec<SupportCandidate> {
    let mut matches = Vec::new();
    for start in 0..units.len() {
        let first = &units[start];
        if first.coverage.start != spans.core.start {
            continue;
        }
        let mut surface = String::new();
        let mut positions = Vec::new();
        let mut node_positions = Vec::new();
        let mut source_node_indices = Vec::new();
        let mut has_opaque = false;
        let mut end = spans.core.start;
        let mut previous_node = None;
        for (unit_index, unit) in units.iter().enumerate().skip(start) {
            if previous_node == Some(unit.node_position) {
                if unit.coverage.end != end {
                    break;
                }
            } else {
                if unit.coverage.start != end {
                    break;
                }
                end = unit.coverage.end;
                node_positions.push(unit.node_position);
                source_node_indices.push(unit.source_node_index);
                previous_node = Some(unit.node_position);
            }
            if end > spans.core.end {
                break;
            }
            surface.push_str(unit.surface);
            positions.push(unit.pos);
            has_opaque |= unit.opaque;
            if end == spans.core.end {
                let lexical_match = node_positions.len() > 1
                    && surface == pattern.lexical_form.as_ref()
                    && runtime_lexical_pos_matches(&positions, pattern.fine_pos);
                if lexical_match {
                    matches.push(SupportCandidate {
                        node_position: unit.node_position,
                        source_node_index: unit.source_node_index,
                        lexical_node_positions: node_positions.clone(),
                        source_node_indices: source_node_indices.clone(),
                        component_index: None,
                        unit_index,
                        evidence: if has_opaque {
                            ConstraintEvidenceKind::OpaqueExpression
                        } else {
                            ConstraintEvidenceKind::RuntimeComposed
                        },
                        relation: ConstraintSpanRelation::RuntimeComponent,
                        opaque: has_opaque,
                    });
                }
                if lexical_match || !pattern.lexical_form.starts_with(&surface) {
                    break;
                }
            }
        }
    }
    matches
}

fn runtime_lexical_pos_matches(positions: &[&str], query: DataFinePos) -> bool {
    let Some((suffix, base)) = positions.split_last() else {
        return false;
    };
    let base_allowed = !base.is_empty()
        && base
            .iter()
            .all(|pos| matches!(*pos, "XPN" | "XR" | "NNG" | "NNP"));
    match query {
        DataFinePos::Vv => base_allowed && *suffix == "XSV",
        DataFinePos::Va => base_allowed && *suffix == "XSA",
        _ => false,
    }
}

fn continuation_proof(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &[Unit<'_>],
    support: &SupportCandidate,
) -> Option<ConstraintContinuationProof> {
    let selected = continuation_units(pattern, spans, units, support)?;
    Some(ConstraintContinuationProof {
        contract: pattern.continuation,
        units: selected
            .into_iter()
            .map(|unit| ConstraintMorphUnitProof {
                pos: unit.pos.to_owned(),
                span: unit.span.clone(),
                source_node_index: unit.source_node_index,
                component_index: unit.component_index,
            })
            .collect(),
    })
}

fn continuation_units<'a>(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &'a [Unit<'a>],
    support: &SupportCandidate,
) -> Option<Vec<&'a Unit<'a>>> {
    let suffix = spans.core.end..spans.consumed.end;
    let selected = suffix_units(units, support, &suffix)?;
    let positions = selected.iter().map(|unit| unit.pos).collect::<Vec<_>>();
    let accepted = match pattern.continuation {
        MorphContinuation::Exact => {
            spans.anchor == spans.core && spans.consumed == spans.anchor && positions.is_empty()
        }
        MorphContinuation::NominalParticles => {
            (spans.consumed == spans.anchor && selected.is_empty())
                || (spans.consumed.end == spans.token.end
                    && nominal_continuation(&selected, units[support.unit_index].surface))
        }
        MorphContinuation::Predicate {
            state,
            nominal_particles,
        } => {
            spans.consumed.end == spans.token.end
                && predicate_continuation(state, nominal_particles, spans, &selected, &positions)
        }
    };
    accepted.then_some(selected)
}

fn suffix_units<'a>(
    units: &'a [Unit<'a>],
    support: &SupportCandidate,
    suffix: &Range<usize>,
) -> Option<Vec<&'a Unit<'a>>> {
    if suffix.start == suffix.end {
        return Some(Vec::new());
    }
    let mut selected = Vec::new();
    for unit in units.iter().skip(support.unit_index + 1) {
        if unit.coverage.end <= suffix.start || unit.coverage.start >= suffix.end {
            continue;
        }
        if unit.span.is_none() && !support.opaque {
            return None;
        }
        if !support.opaque && (unit.coverage.start < suffix.start || unit.coverage.end > suffix.end)
        {
            return None;
        }
        selected.push(unit);
    }
    if support.opaque {
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

fn nominal_continuation(units: &[&Unit<'_>], support_surface: &str) -> bool {
    let mut particles = false;
    let morphology_allowed = units.iter().all(|unit| {
        if unit.pos == "XSN" && !particles {
            true
        } else if unit.pos.starts_with('J') {
            particles = true;
            true
        } else {
            false
        }
    });
    morphology_allowed && valid_particle_sequence(units, support_surface)
}

fn predicate_continuation(
    state: ContinuationState,
    nominal_particles: bool,
    spans: &CandidateSpans,
    units: &[&Unit<'_>],
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
        .map(|unit| unit.pos)
        .collect::<Vec<_>>();
    let state_allowed = match state {
        ContinuationState::Terminal if nominal_particles => {
            post_anchor.iter().all(|pos| pos.starts_with('J'))
        }
        ContinuationState::Terminal => post_anchor.is_empty(),
        ContinuationState::Eu => !post_anchor.is_empty(),
        ContinuationState::AOrEo
        | ContinuationState::Past
        | ContinuationState::Future
        | ContinuationState::Declarative => true,
    };
    state_allowed && valid_particle_sequence(units, "")
}

fn valid_particle_sequence(units: &[&Unit<'_>], support_surface: &str) -> bool {
    let Some(first_particle) = units.iter().position(|unit| unit.pos.starts_with('J')) else {
        return true;
    };
    let host_surface = units[..first_particle]
        .iter()
        .rev()
        .find_map(|unit| (!unit.surface.is_empty()).then_some(unit.surface))
        .unwrap_or(support_surface);
    let Some(host) = host_surface.chars().next_back() else {
        return true;
    };
    let mut case_particles = 0_usize;
    for unit in &units[first_particle..] {
        if !unit.pos.starts_with('J') {
            return false;
        }
        if is_case_particle(unit.pos, unit.surface) {
            case_particles += 1;
            if case_particles > 1 {
                return false;
            }
        }
        if !particle_allomorph_accepts(unit.surface, host) {
            return false;
        }
    }
    true
}

fn is_case_particle(pos: &str, surface: &str) -> bool {
    matches!(pos, "JKS" | "JKC" | "JKG" | "JKO" | "JKB" | "JKV" | "JKQ")
        || matches!(surface, "이" | "가" | "을" | "를" | "으로" | "로")
}

fn particle_allomorph_accepts(surface: &str, host: char) -> bool {
    if crate::decompose_syllable(host).is_none() {
        return true;
    }
    match surface {
        "이" | "을" => crate::has_final(host),
        "가" | "를" => !crate::has_final(host),
        "으로" => crate::has_final(host) && !crate::has_rieul_final(host),
        "로" => !crate::has_final(host) || crate::has_rieul_final(host),
        _ => true,
    }
}

fn context_proof(
    selection: &ContextSelection,
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
) -> Option<Option<ConstraintContextProof>> {
    context_match(selection, pattern, spans).map(|matched| match matched {
        ContextMatch::None => None,
        ContextMatch::Repeated { side } => Some(ConstraintContextProof::RepeatedToken { side }),
        ContextMatch::Copular { role, selected } => Some(ConstraintContextProof::CopularFrame {
            role,
            selected: selected.clone(),
        }),
        ContextMatch::NominalParticleHost { selected } => {
            Some(ConstraintContextProof::NominalParticleHost {
                selected: selected.clone(),
            })
        }
    })
}

enum ContextMatch<'a> {
    None,
    Repeated {
        side: AdjacentSide,
    },
    Copular {
        role: CopularFrameRole,
        selected: &'a Range<usize>,
    },
    NominalParticleHost {
        selected: &'a Range<usize>,
    },
}

fn context_match<'a>(
    selection: &'a ContextSelection,
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
) -> Option<ContextMatch<'a>> {
    match selection {
        ContextSelection::None => Some(ContextMatch::None),
        ContextSelection::Competing => None,
        ContextSelection::Repeated { side } => {
            pattern
                .adjacent
                .iter()
                .find_map(|constraint| match constraint {
                    AdjacentTokenConstraint::RepeatedToken { side: required }
                        if side_matches(*required, *side) && spans.core == spans.token =>
                    {
                        Some(ContextMatch::Repeated { side: *side })
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
                    } if spans.core == *nominal => Some(ContextMatch::Copular {
                        role: CopularFrameRole::Nominal,
                        selected: nominal,
                    }),
                    AdjacentTokenConstraint::CopularFrame {
                        role: CopularFrameRole::Copula,
                    } if spans.core == *copula => Some(ContextMatch::Copular {
                        role: CopularFrameRole::Copula,
                        selected: copula,
                    }),
                    _ => None,
                })
        }
        ContextSelection::NominalParticleHost { selected } => {
            if pattern.fine_pos.is_nominal() && spans.core == *selected {
                Some(ContextMatch::NominalParticleHost { selected })
            } else if is_predicate_pos(pattern.fine_pos) && spans.core.start == 0 {
                Some(ContextMatch::None)
            } else {
                None
            }
        }
    }
}

fn side_matches(required: AdjacentSide, actual: AdjacentSide) -> bool {
    required == AdjacentSide::Either || actual == AdjacentSide::Either || required == actual
}

fn repeated_selection(
    context: BoundedTokenContext<'_>,
    current: &TokenGraph<'_>,
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
    previous: &TokenGraph<'_>,
    current: &TokenGraph<'_>,
    next: &TokenGraph<'_>,
) -> Option<(Range<usize>, Range<usize>)> {
    if !has_pos_sequence(previous, &["VCN", "EC"])
        || !starts_with_pos(next, |pos| matches!(pos, "NNB" | "NNBC"))
    {
        return None;
    }
    let mut splits = BTreeSet::new();
    for path in current.witness_paths() {
        let units = path_units(&path, current.nodes());
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
                && source_pos(prefix[0].pos).is_some_and(DataFinePos::is_nominal)
                && exact_coverage(&prefix, 0..split)
                && suffix.iter().map(|unit| unit.pos).eq(["VCP", "ETM"])
                && enclosing_coverage(&suffix, split..current_text.len())
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

fn nominal_particle_host_selection(
    current_text: &str,
    current: &TokenGraph<'_>,
) -> Option<Range<usize>> {
    let mut splits = BTreeSet::new();
    for path in current.witness_paths() {
        let units = path_units(&path, current.nodes());
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
                && source_pos(prefix[0].pos).is_some_and(DataFinePos::is_nominal)
                && exact_coverage(&prefix, 0..split)
                && !suffix.is_empty()
                && suffix.iter().all(|unit| unit.pos.starts_with('J'))
                && enclosing_coverage(&suffix, split..current_text.len())
                && valid_particle_sequence(&suffix, prefix[0].surface)
            {
                splits.insert(split);
            }
        }
    }
    (splits.len() == 1).then(|| {
        let split = *splits.first().expect("single nominal particle split");
        0..split
    })
}

fn is_predicate_pos(pos: DataFinePos) -> bool {
    matches!(
        pos,
        DataFinePos::Vv | DataFinePos::Va | DataFinePos::Vx | DataFinePos::Vcp | DataFinePos::Vcn
    )
}

fn has_exact_pos(graph: &TokenGraph<'_>, expected: &str) -> bool {
    graph.witness_paths().iter().any(|path| {
        path.len() == 1
            && graph.nodes()[path[0]]
                .pos
                .split('+')
                .eq(std::iter::once(expected))
    })
}

fn has_complete_pos(graph: &TokenGraph<'_>, expected: &str) -> bool {
    graph.nodes().iter().enumerate().any(|(index, node)| {
        graph.is_on_complete_path(index)
            && (node.pos.split('+').any(|pos| pos == expected)
                || node
                    .components
                    .iter()
                    .any(|component| component.pos == expected))
    })
}

fn has_pos_sequence(graph: &TokenGraph<'_>, expected: &[&str]) -> bool {
    graph.witness_paths().iter().any(|path| {
        path.iter()
            .flat_map(|&index| graph.nodes()[index].pos.split('+'))
            .eq(expected.iter().copied())
    })
}

fn starts_with_pos(graph: &TokenGraph<'_>, accepts: impl Fn(&str) -> bool) -> bool {
    graph.witness_paths().iter().any(|path| {
        path.first()
            .and_then(|&index| graph.nodes()[index].pos.split('+').next())
            .is_some_and(&accepts)
    })
}

fn path_units<'a>(path: &[usize], nodes: &'a [Node<'a>]) -> Vec<Unit<'a>> {
    let mut units = Vec::new();
    for (node_position, &node_index) in path.iter().enumerate() {
        let node = &nodes[node_index];
        if node.components.is_empty() {
            units.extend(node.pos.split('+').enumerate().map(|(pos_slot, pos)| Unit {
                node_position,
                source_node_index: node_index,
                component_index: None,
                surface: node.surface,
                pos,
                pos_slot,
                span: Some(node.span.clone()),
                coverage: node.span.clone(),
                opaque: false,
            }));
        } else {
            units.extend(
                node.components
                    .iter()
                    .enumerate()
                    .map(|(component_index, component)| Unit {
                        node_position,
                        source_node_index: node_index,
                        component_index: Some(component_index),
                        surface: component.surface,
                        pos: component.pos,
                        pos_slot: 0,
                        span: component.span.clone(),
                        coverage: component.span.clone().unwrap_or_else(|| node.span.clone()),
                        opaque: component.span.is_none(),
                    }),
            );
        }
    }
    units
}

fn exact_coverage(units: &[&Unit<'_>], expected: Range<usize>) -> bool {
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

fn enclosing_coverage(units: &[&Unit<'_>], expected: Range<usize>) -> bool {
    let mut spans = units
        .iter()
        .map(|unit| unit.coverage.clone())
        .collect::<Vec<_>>();
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

fn annotate_proof(
    nodes: &mut [super::ConstraintNodeProof],
    paths: &mut [super::ConstraintPathProof],
    analyses: &[SupportedAnalysis],
) {
    for analysis in analyses {
        let Some(path) = paths.get_mut(analysis.path_index) else {
            continue;
        };
        path.evidence = evidence_preference(path.evidence, analysis.evidence);
        for &node_index in &analysis.lexical_source_node_indices {
            let Some(node) = nodes.get_mut(node_index) else {
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
