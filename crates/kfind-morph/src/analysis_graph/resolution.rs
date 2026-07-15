use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::OnceLock;

use kfind_data::{
    DataFinePos, MorphologyGraphAnalysis, MorphologyGraphExpressionKind, MorphologyGraphPosClass,
    MorphologyGraphResource,
};

use crate::ContinuationState;

use super::paths::{Node, TokenGraph};
use super::{
    AdjacentSide, AdjacentTokenConstraint, CandidateSpans, ConstraintAmbiguity,
    ConstraintEvidenceKind, ConstraintNodeSource, ConstraintProof, ConstraintResolution,
    ConstraintUnavailable, CopularFrameRole, MorphContinuation, QueryMorphPattern,
};

mod attached_nominal;

use attached_nominal::{
    attached_nominal_frame_prefix, can_attach_nominal_frame, match_attached_nominal_frame,
    maximal_attached_successor_end,
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
    AttachedNominalFrame {
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
    NominalParticleHosts {
        selected: Vec<Range<usize>>,
    },
    Competing,
}

#[derive(Debug, Default)]
pub(super) struct PreparedTokenSummary {
    nominal_particle_hosts: OnceLock<Vec<Range<usize>>>,
}

impl PreparedTokenSummary {
    pub fn nominal_particle_hosts<'a>(
        &'a self,
        current_text: &str,
        current: &TokenGraph<'_>,
    ) -> &'a [Range<usize>] {
        self.nominal_particle_hosts
            .get_or_init(|| nominal_particle_host_candidates(current_text, current))
    }
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
    particle_hosts: &[Range<usize>],
    previous: Option<&TokenGraph<'_>>,
    next: Option<&TokenGraph<'_>>,
) -> ContextSelection {
    let repeated = repeated_selection(context, current);
    let copular = previous
        .zip(next)
        .and_then(|(previous, next)| copular_selection(context.current, previous, current, next));
    match (
        repeated.is_some(),
        copular.is_some(),
        !particle_hosts.is_empty(),
    ) {
        (false, false, false) => ContextSelection::None,
        (true, false, false) => ContextSelection::Repeated {
            side: repeated.expect("present repeated selection"),
        },
        (false, true, false) => {
            let (nominal, copula) = copular.expect("present copular selection");
            ContextSelection::Copular { nominal, copula }
        }
        (false, false, true) => ContextSelection::NominalParticleHosts {
            selected: particle_hosts.to_vec(),
        },
        _ => ContextSelection::Competing,
    }
}

pub(super) fn prepare_token_summary() -> PreparedTokenSummary {
    PreparedTokenSummary::default()
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QueryLexicalUnit {
    surface: String,
    pos: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QueryLexicalTrace {
    units: Vec<QueryLexicalUnit>,
}

#[derive(Debug)]
pub(super) struct PreparedQueryTraces {
    by_pattern: Vec<Vec<QueryLexicalTrace>>,
}

static EMPTY_QUERY_TRACES: PreparedQueryTraces = PreparedQueryTraces {
    by_pattern: Vec::new(),
};

impl PreparedQueryTraces {
    fn for_pattern(&self, pattern_index: usize) -> &[QueryLexicalTrace] {
        self.by_pattern
            .get(pattern_index)
            .map_or(&[], Vec::as_slice)
    }

    fn has_runtime_traces(&self) -> bool {
        self.by_pattern.iter().any(|traces| !traces.is_empty())
    }
}

pub(super) fn empty_query_traces() -> &'static PreparedQueryTraces {
    &EMPTY_QUERY_TRACES
}

pub(super) fn has_runtime_lexical_path(graph: &TokenGraph<'_>, spans: &CandidateSpans) -> bool {
    if spans.core.start != spans.token.start || spans.core.start == spans.core.end {
        return false;
    }
    let mut reaches_core_end = vec![false; graph.node_count()];
    for index in (0..graph.node_count()).rev() {
        let node = &graph.nodes()[index];
        if !graph.is_on_complete_path(index)
            || node.span.start < spans.core.start
            || node.span.end > spans.core.end
        {
            continue;
        }
        reaches_core_end[index] = node.span.end == spans.core.end
            || graph
                .successors(index)
                .iter()
                .any(|&successor| reaches_core_end[successor]);
    }
    graph.nodes().iter().enumerate().any(|(index, node)| {
        graph.is_on_complete_path(index)
            && node.span.start == spans.core.start
            && node.span.end < spans.core.end
            && graph
                .successors(index)
                .iter()
                .any(|&successor| reaches_core_end[successor])
    })
}

pub(super) fn prepare_query_traces(
    resource: &MorphologyGraphResource,
    patterns: &[QueryMorphPattern],
    node_limit: usize,
    trace_limit: usize,
) -> PreparedQueryTraces {
    let by_pattern = patterns
        .iter()
        .map(|pattern| query_lexical_traces(resource, pattern, node_limit, trace_limit))
        .collect();
    PreparedQueryTraces { by_pattern }
}

fn query_lexical_traces(
    resource: &MorphologyGraphResource,
    pattern: &QueryMorphPattern,
    node_limit: usize,
    trace_limit: usize,
) -> Vec<QueryLexicalTrace> {
    if !pattern.fine_pos.is_nominal()
        && !matches!(pattern.fine_pos, DataFinePos::Vv | DataFinePos::Va)
    {
        return Vec::new();
    }
    if query_node_limit_exceeded(resource, &pattern.lexical_form, node_limit) {
        return Vec::new();
    }
    let mut traces = BTreeSet::new();
    let mut units = Vec::new();
    collect_query_lexical_traces(
        resource,
        &pattern.lexical_form,
        pattern.fine_pos,
        0,
        None,
        0,
        PredicateLexicalStage::Start,
        trace_limit,
        &mut units,
        &mut traces,
    );
    traces.into_iter().collect()
}

fn query_node_limit_exceeded(
    resource: &MorphologyGraphResource,
    lexical_form: &str,
    node_limit: usize,
) -> bool {
    let mut node_count = 0_usize;
    for (start, _) in lexical_form.char_indices() {
        resource.common_prefixes(&lexical_form.as_bytes()[start..], |length, _, analyses| {
            if lexical_form.get(start..start + length).is_some() {
                node_count = node_count.saturating_add(analyses.len());
            }
        });
        if node_count > node_limit {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PredicateLexicalStage {
    Start,
    NominalBase,
    Predicate,
    Bridge,
}

#[allow(clippy::too_many_arguments)]
fn collect_query_lexical_traces(
    resource: &MorphologyGraphResource,
    lexical_form: &str,
    fine_pos: DataFinePos,
    offset: usize,
    previous_end: Option<MorphologyGraphPosClass>,
    path_node_count: usize,
    predicate_stage: PredicateLexicalStage,
    trace_limit: usize,
    units: &mut Vec<QueryLexicalUnit>,
    traces: &mut BTreeSet<QueryLexicalTrace>,
) {
    if traces.len() >= trace_limit {
        return;
    }
    resource.common_prefixes(&lexical_form.as_bytes()[offset..], |length, _, analyses| {
        let end = offset + length;
        if end > lexical_form.len() || traces.len() >= trace_limit {
            return;
        }
        for analysis in analyses {
            if analysis.components.is_empty() && analysis.pos.contains('+') {
                continue;
            }
            let start_class = resource.transition_class(effective_graph_start_pos(analysis));
            if previous_end.is_some()
                && !previous_end
                    .zip(start_class)
                    .is_some_and(|(previous, start)| {
                        resource.allows_transition_classes(previous, start)
                    })
            {
                continue;
            }
            if fine_pos.is_nominal()
                && (analysis.pos.contains('+')
                    || !source_pos(analysis.pos).is_some_and(DataFinePos::is_nominal))
            {
                continue;
            }
            let previous_len = units.len();
            let Some(next_stage) = append_query_units(
                lexical_form,
                offset,
                end,
                analysis,
                fine_pos,
                predicate_stage,
                units,
            ) else {
                continue;
            };
            let next_node_count = path_node_count + 1;
            if end == lexical_form.len() {
                let accepted = next_node_count >= 2
                    && (fine_pos.is_nominal() || next_stage == PredicateLexicalStage::Predicate);
                if accepted {
                    traces.insert(QueryLexicalTrace {
                        units: units.clone(),
                    });
                }
            } else if let Some(end_class) =
                resource.transition_class(effective_graph_end_pos(analysis))
            {
                collect_query_lexical_traces(
                    resource,
                    lexical_form,
                    fine_pos,
                    end,
                    Some(end_class),
                    next_node_count,
                    next_stage,
                    trace_limit,
                    units,
                    traces,
                );
            }
            units.truncate(previous_len);
            if traces.len() >= trace_limit {
                return;
            }
        }
    });
}

fn append_query_units(
    lexical_form: &str,
    start: usize,
    end: usize,
    analysis: &MorphologyGraphAnalysis<'_>,
    fine_pos: DataFinePos,
    mut stage: PredicateLexicalStage,
    units: &mut Vec<QueryLexicalUnit>,
) -> Option<PredicateLexicalStage> {
    let previous_len = units.len();
    if analysis.components.is_empty() {
        let surface = lexical_form.get(start..end)?;
        for pos in analysis.pos.split('+') {
            if matches!(fine_pos, DataFinePos::Vv | DataFinePos::Va) {
                let Some(next) = advance_predicate_lexical_stage(stage, pos) else {
                    units.truncate(previous_len);
                    return None;
                };
                stage = next;
            }
            units.push(QueryLexicalUnit {
                surface: surface.to_owned(),
                pos: pos.to_owned(),
            });
        }
    } else {
        for component in &analysis.components {
            if matches!(fine_pos, DataFinePos::Vv | DataFinePos::Va) {
                let Some(next) = advance_predicate_lexical_stage(stage, component.pos) else {
                    units.truncate(previous_len);
                    return None;
                };
                stage = next;
            }
            units.push(QueryLexicalUnit {
                surface: component.surface.to_owned(),
                pos: component.pos.to_owned(),
            });
        }
    }
    Some(stage)
}

fn advance_predicate_lexical_stage(
    stage: PredicateLexicalStage,
    pos: &str,
) -> Option<PredicateLexicalStage> {
    match (stage, pos) {
        (
            PredicateLexicalStage::Start | PredicateLexicalStage::NominalBase,
            "XPN" | "XR" | "NNG" | "NNP",
        ) => Some(PredicateLexicalStage::NominalBase),
        (PredicateLexicalStage::Start, "VV" | "VA" | "VX") => {
            Some(PredicateLexicalStage::Predicate)
        }
        (PredicateLexicalStage::NominalBase, "XSV" | "XSA")
        | (PredicateLexicalStage::Predicate, "VX")
        | (PredicateLexicalStage::Bridge, "VX") => Some(PredicateLexicalStage::Predicate),
        (PredicateLexicalStage::Predicate, "EC") => Some(PredicateLexicalStage::Bridge),
        _ => None,
    }
}

fn effective_graph_start_pos<'a>(analysis: &'a MorphologyGraphAnalysis<'a>) -> &'a str {
    if analysis.start_pos == "*" {
        analysis.pos.split('+').next().unwrap_or("*")
    } else {
        analysis.start_pos
    }
}

fn effective_graph_end_pos<'a>(analysis: &'a MorphologyGraphAnalysis<'a>) -> &'a str {
    if analysis.end_pos == "*" {
        analysis.pos.split('+').next_back().unwrap_or("*")
    } else {
        analysis.end_pos
    }
}

pub(super) fn needs_nominal_particle_context(
    patterns: &[QueryMorphPattern],
    spans: &CandidateSpans,
) -> bool {
    patterns.iter().any(|pattern| {
        pattern.fine_pos.is_nominal()
            || (is_predicate_pos(pattern.fine_pos) && spans.core.start > spans.token.start)
    })
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
    query_traces: &PreparedQueryTraces,
    context: &ContextSelection,
    proof_limit: usize,
) -> ConstraintResolution {
    let unknown_node_count = graph.unknown_node_count();
    let evaluation = evaluate_known(graph, spans, patterns, query_traces, context, proof_limit);
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
            known_node_count: graph.node_count() - unknown_node_count,
            unknown_node_count,
            nodes: proof_nodes,
            paths: proof_paths,
        },
    }
}

pub(super) fn decide_known(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    query_traces: &PreparedQueryTraces,
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
    let runtime_paths = if query_traces.has_runtime_traces() {
        runtime_lexical_candidate_paths(graph, spans, query_traces)
    } else {
        Vec::new()
    };
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        let Some(context_match) = context_match(context, pattern, spans) else {
            continue;
        };
        let context_resolved = !matches!(context_match, ContextMatch::None);
        let attached_nominal_allowed = can_attach_nominal_frame(graph, pattern, spans);
        for (node_index, node) in graph.nodes().iter().enumerate() {
            if !graph.is_on_complete_path(node_index)
                || node.span.start > spans.core.start
                || node.span.end < spans.core.end
            {
                continue;
            }
            let lexical_path = [node_index];
            let mut traversal_path = DecisionTraversalPath {
                last: node_index,
                units: path_units(&lexical_path, graph.nodes()),
            };
            for support in source_support_candidates(
                &lexical_path,
                graph.nodes(),
                &traversal_path.units,
                spans,
                pattern,
            ) {
                if extend_decision_support(
                    graph,
                    spans,
                    IndexedPattern {
                        index: pattern_index,
                        pattern,
                        context_resolved,
                        attached_nominal_allowed,
                    },
                    &mut traversal_path,
                    &support,
                    &mut proofs,
                    proof_limit,
                ) {
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
        let traces = query_traces.for_pattern(pattern_index);
        if traces.is_empty() || spans.core.start != spans.token.start {
            continue;
        }
        for lexical_path in &runtime_paths {
            let mut traversal_path = DecisionTraversalPath {
                last: *lexical_path.last().expect("lexical path is non-empty"),
                units: path_units(lexical_path, graph.nodes()),
            };
            for support in runtime_lexical_candidates(&traversal_path.units, spans, traces) {
                if extend_decision_support(
                    graph,
                    spans,
                    IndexedPattern {
                        index: pattern_index,
                        pattern,
                        context_resolved,
                        attached_nominal_allowed,
                    },
                    &mut traversal_path,
                    &support,
                    &mut proofs,
                    proof_limit,
                ) {
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
        proofs
            .iter()
            .map(|proof| (proof.support.span_relation, proof.context_resolved)),
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
    context_resolved: bool,
}

#[derive(Clone, Copy)]
struct IndexedPattern<'a> {
    index: usize,
    pattern: &'a QueryMorphPattern,
    context_resolved: bool,
    attached_nominal_allowed: bool,
}

struct DecisionTraversalPath<'a> {
    last: usize,
    units: Vec<Unit<'a>>,
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
    query_traces: &PreparedQueryTraces,
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
    let paths = candidate_witness_paths(graph, spans, patterns, query_traces);
    let attached_nominal_allowed = patterns
        .iter()
        .map(|pattern| can_attach_nominal_frame(graph, pattern, spans))
        .collect::<Vec<_>>();
    for (path_index, path) in paths.iter().enumerate() {
        let units = path_units(path, graph.nodes());
        for (pattern_index, pattern) in patterns.iter().enumerate() {
            for candidate in support_candidates(
                path,
                graph.nodes(),
                &units,
                spans,
                pattern,
                query_traces.for_pattern(pattern_index),
            ) {
                let Some(continuation) = continuation_proof(
                    pattern,
                    spans,
                    &units,
                    &candidate,
                    attached_nominal_allowed[pattern_index],
                ) else {
                    continue;
                };
                let Some(external_context) = context_proof(context, pattern, spans) else {
                    continue;
                };
                let context_proof = continuation
                    .attached_nominal
                    .map_or(external_context, |selected| {
                        Some(ConstraintContextProof::AttachedNominalFrame { selected })
                    });
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
                    continuation: continuation.proof,
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
        analyses
            .iter()
            .map(|analysis| (analysis.span_relation, analysis.context.is_some())),
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
    relations: impl IntoIterator<Item = (ConstraintSpanRelation, bool)>,
) -> ConstraintOutcome {
    let mut has_stable = false;
    let mut has_opaque = false;
    let mut all_non_whole = true;
    let mut context_resolved = false;
    for (relation, resolved) in relations {
        has_stable |= relation != ConstraintSpanRelation::OpaqueExpression;
        has_opaque |= relation == ConstraintSpanRelation::OpaqueExpression;
        all_non_whole &= relation != ConstraintSpanRelation::Whole;
        context_resolved |= resolved;
    }
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
    query_traces: &PreparedQueryTraces,
) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    let lexical_paths = lexical_candidate_paths(graph, spans, query_traces);
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        let attached_nominal_allowed = can_attach_nominal_frame(graph, pattern, spans);
        for lexical_path in &lexical_paths {
            let units = path_units(lexical_path, graph.nodes());
            for support in support_candidates(
                lexical_path,
                graph.nodes(),
                &units,
                spans,
                pattern,
                query_traces.for_pattern(pattern_index),
            ) {
                extend_supported_path(
                    graph,
                    spans,
                    pattern,
                    lexical_path.clone(),
                    support,
                    attached_nominal_allowed,
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
    query_traces: &PreparedQueryTraces,
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
            extend_lexical_path(graph, spans, query_traces, vec![index], &mut paths);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn runtime_lexical_candidate_paths(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    query_traces: &PreparedQueryTraces,
) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    for (index, node) in graph.nodes().iter().enumerate() {
        if graph.is_on_complete_path(index)
            && node.span.start == spans.core.start
            && node.span.end < spans.core.end
        {
            extend_lexical_path(graph, spans, query_traces, vec![index], &mut paths);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn extend_lexical_path(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    query_traces: &PreparedQueryTraces,
    required: Vec<usize>,
    paths: &mut Vec<Vec<usize>>,
) {
    let last = *required.last().expect("lexical path is non-empty");
    let end = graph.nodes()[last].span.end;
    if end == spans.core.end {
        paths.push(required);
        return;
    }
    if end > spans.core.end || !lexical_path_can_match(graph, &required, query_traces) {
        return;
    }
    for &successor in graph.successors(last) {
        let next = &graph.nodes()[successor];
        if next.span.end > spans.core.end || !graph.is_on_complete_path(successor) {
            continue;
        }
        let mut extended = required.clone();
        extended.push(successor);
        extend_lexical_path(graph, spans, query_traces, extended, paths);
    }
}

fn lexical_path_can_match(
    graph: &TokenGraph<'_>,
    path: &[usize],
    query_traces: &PreparedQueryTraces,
) -> bool {
    let units = path_units(path, graph.nodes());
    query_traces.by_pattern.iter().flatten().any(|trace| {
        units.len() <= trace.units.len()
            && units
                .iter()
                .zip(&trace.units)
                .all(|(source, query)| source_unit_matches_query(source, query))
    })
}

fn extend_supported_path(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    pattern: &QueryMorphPattern,
    required: Vec<usize>,
    support: SupportCandidate,
    attached_nominal_allowed: bool,
    paths: &mut Vec<Vec<usize>>,
) {
    let last = *required.last().expect("supported path is non-empty");
    let units = path_units(&required, graph.nodes());
    let required_end = support_path_end(pattern, spans);
    if graph.nodes()[last].span.end >= required_end {
        if continuation_proof(pattern, spans, &units, &support, attached_nominal_allowed).is_none()
        {
            return;
        }
        if let Some(path) = graph.witness_path_through(&required)
            && !paths.contains(&path)
        {
            paths.push(path);
        }
        return;
    }
    if !continuation_prefix_possible(pattern, spans, &units, &support, attached_nominal_allowed) {
        return;
    }
    let successors = graph.successors(last);
    let maximal_attached_end = maximal_attached_successor_end(
        graph,
        successors,
        pattern,
        spans,
        &units,
        &support,
        attached_nominal_allowed,
    );
    for &successor in successors {
        let next = &graph.nodes()[successor];
        if next.span.end > required_end
            || !graph.is_on_complete_path(successor)
            || maximal_attached_end.is_some_and(|end| next.span.end < end)
        {
            continue;
        }
        let mut extended = required.clone();
        extended.push(successor);
        extend_supported_path(
            graph,
            spans,
            pattern,
            extended,
            support.clone(),
            attached_nominal_allowed,
            paths,
        );
    }
}

fn extend_decision_support<'a>(
    graph: &TokenGraph<'a>,
    spans: &CandidateSpans,
    pattern: IndexedPattern<'_>,
    path: &mut DecisionTraversalPath<'a>,
    support: &SupportCandidate,
    proofs: &mut BTreeSet<CompactSupportProof>,
    proof_limit: usize,
) -> bool {
    let required_end = support_path_end(pattern.pattern, spans);
    if graph.nodes()[path.last].span.end >= required_end {
        let Some(continuation) = continuation_units(
            pattern.pattern,
            spans,
            &path.units,
            support,
            pattern.attached_nominal_allowed,
        ) else {
            return false;
        };
        let proof = CompactSupportProof {
            support: ConstraintSupport {
                pattern_index: pattern.index,
                evidence: support.evidence,
                span_relation: support.relation,
            },
            source_node_index: support.source_node_index,
            lexical_source_node_indices: support.source_node_indices.clone(),
            component_index: support.component_index,
            continuation: continuation
                .units
                .into_iter()
                .map(|unit| CompactMorphUnit {
                    pos_slot: unit.pos_slot,
                    span: unit.span.as_ref().map(|span| (span.start, span.end)),
                    source_node_index: unit.source_node_index,
                    component_index: unit.component_index,
                })
                .collect(),
            context_resolved: pattern.context_resolved || continuation.attached_nominal.is_some(),
        };
        return proofs.insert(proof) && proofs.len() > proof_limit;
    }
    if !continuation_prefix_possible(
        pattern.pattern,
        spans,
        &path.units,
        support,
        pattern.attached_nominal_allowed,
    ) {
        return false;
    }
    let previous_last = path.last;
    let successors = graph.successors(previous_last);
    let maximal_attached_end = maximal_attached_successor_end(
        graph,
        successors,
        pattern.pattern,
        spans,
        &path.units,
        support,
        pattern.attached_nominal_allowed,
    );
    for &successor in successors {
        let next = &graph.nodes()[successor];
        if next.span.end > required_end
            || !graph.is_on_complete_path(successor)
            || maximal_attached_end.is_some_and(|end| next.span.end < end)
        {
            continue;
        }
        let previous_len = path.units.len();
        let node_position = path
            .units
            .last()
            .map_or(0, |unit| unit.node_position.saturating_add(1));
        append_node_units(&mut path.units, node_position, successor, graph.nodes());
        path.last = successor;
        if extend_decision_support(graph, spans, pattern, path, support, proofs, proof_limit) {
            path.units.truncate(previous_len);
            path.last = previous_last;
            return true;
        }
        path.units.truncate(previous_len);
        path.last = previous_last;
    }
    false
}

fn continuation_prefix_possible(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &[Unit<'_>],
    support: &SupportCandidate,
    attached_nominal_allowed: bool,
) -> bool {
    let Some(selected) =
        continuation_prefix_units(pattern, spans, units, support, attached_nominal_allowed)
    else {
        return false;
    };
    match pattern.continuation {
        MorphContinuation::Exact => selected.is_empty(),
        MorphContinuation::NominalParticles => nominal_prefix(selected.iter().map(|unit| unit.pos)),
        MorphContinuation::Predicate { .. } => {
            predicate_prefix(selected.iter().map(|unit| unit.pos))
                || (attached_nominal_allowed && attached_nominal_frame_prefix(&selected))
        }
    }
}

fn continuation_prefix_units<'units, 'data>(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &'units [Unit<'data>],
    support: &SupportCandidate,
    attached_nominal_allowed: bool,
) -> Option<Vec<&'units Unit<'data>>> {
    let partial_end = units
        .iter()
        .skip(support.unit_index + 1)
        .map(|unit| unit.coverage.end)
        .max()
        .unwrap_or(spans.core.end)
        .min(spans.consumed.end);
    let selected = suffix_unit_view(
        pattern,
        units,
        support,
        spans.anchor.clone(),
        spans.core.end..partial_end,
        attached_nominal_allowed,
    )?;
    Some(selected.iter().collect())
}

fn support_path_end(_pattern: &QueryMorphPattern, spans: &CandidateSpans) -> usize {
    spans.consumed.end
}

fn nominal_prefix<'a>(positions: impl IntoIterator<Item = &'a str>) -> bool {
    let mut particles = false;
    positions.into_iter().all(|pos| {
        if pos == "XSN" && !particles {
            true
        } else if pos.starts_with('J') {
            particles = true;
            true
        } else {
            false
        }
    })
}

fn predicate_prefix<'a>(positions: impl IntoIterator<Item = &'a str>) -> bool {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Stage {
        Predicate,
        Nominalized,
        Terminal,
    }
    let mut stage = Stage::Predicate;
    positions.into_iter().all(|pos| match pos {
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
    source: ConstraintNodeSource,
}

struct ContinuationUnitView<'units, 'data> {
    units: &'units [Unit<'data>],
    start: usize,
    anchor: Range<usize>,
    suffix: Range<usize>,
    include_opaque_nominalizer_tail: bool,
    include_opaque_adnominal_tail: bool,
    source_node_index: usize,
    component_index: Option<usize>,
}

impl<'units, 'data> ContinuationUnitView<'units, 'data> {
    fn iter(&self) -> impl Iterator<Item = &'units Unit<'data>> + '_ {
        self.units[self.start..].iter().filter(|unit| {
            self.is_opaque_anchor_tail(unit)
                || (self.suffix.start < self.suffix.end
                    && unit.coverage.end > self.suffix.start
                    && unit.coverage.start < self.suffix.end)
        })
    }

    fn is_opaque_anchor_tail(&self, unit: &Unit<'_>) -> bool {
        ((self.include_opaque_nominalizer_tail && unit.pos == "ETN")
            || (self.include_opaque_adnominal_tail && unit.pos == "ETM"))
            && unit.source_node_index == self.source_node_index
            && unit.span.is_none()
            && self
                .component_index
                .zip(unit.component_index)
                .is_some_and(|(support, candidate)| candidate > support)
            && self.anchor.start <= unit.coverage.start
            && unit.coverage.end <= self.anchor.end
    }
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
    query_traces: &[QueryLexicalTrace],
) -> Vec<SupportCandidate> {
    let mut matches = source_support_candidates(path, nodes, units, spans, pattern);
    if spans.core.start == spans.token.start {
        matches.extend(runtime_lexical_candidates(units, spans, query_traces));
    }
    matches
}

fn source_support_candidates(
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
    matches
}

fn runtime_lexical_candidates(
    units: &[Unit<'_>],
    spans: &CandidateSpans,
    query_traces: &[QueryLexicalTrace],
) -> Vec<SupportCandidate> {
    let mut matches = Vec::new();
    for trace in query_traces {
        if trace.units.is_empty() || trace.units.len() > units.len() {
            continue;
        }
        let lexical_units = &units[..trace.units.len()];
        if !lexical_units
            .iter()
            .zip(&trace.units)
            .all(|(source, query)| source_unit_matches_query(source, query))
        {
            continue;
        }
        let first = &lexical_units[0];
        let last = lexical_units
            .last()
            .expect("query lexical trace is non-empty");
        if first.coverage.start != spans.core.start || last.coverage.end != spans.core.end {
            continue;
        }
        if units[trace.units.len()..].iter().any(|unit| {
            unit.node_position != last.node_position && unit.coverage.start < spans.core.end
        }) {
            continue;
        }
        let lexical_refs = lexical_units.iter().collect::<Vec<_>>();
        if !enclosing_coverage(&lexical_refs, spans.core.clone()) {
            continue;
        }
        let mut node_positions = Vec::new();
        let mut source_node_indices = Vec::new();
        for unit in lexical_units {
            if node_positions.last() != Some(&unit.node_position) {
                node_positions.push(unit.node_position);
                source_node_indices.push(unit.source_node_index);
            }
        }
        let has_opaque = lexical_units.iter().any(|unit| unit.opaque);
        matches.push(SupportCandidate {
            node_position: last.node_position,
            source_node_index: last.source_node_index,
            lexical_node_positions: node_positions,
            source_node_indices,
            component_index: last.component_index,
            unit_index: trace.units.len() - 1,
            evidence: if has_opaque {
                ConstraintEvidenceKind::OpaqueExpression
            } else {
                ConstraintEvidenceKind::RuntimeComposed
            },
            relation: ConstraintSpanRelation::RuntimeComponent,
            opaque: has_opaque,
        });
    }
    matches.sort_by_key(|candidate| {
        (
            candidate.node_position,
            candidate.component_index,
            candidate.source_node_indices.clone(),
        )
    });
    matches.dedup_by(|left, right| {
        left.node_position == right.node_position
            && left.component_index == right.component_index
            && left.source_node_indices == right.source_node_indices
    });
    matches
}

fn source_unit_matches_query(source: &Unit<'_>, query: &QueryLexicalUnit) -> bool {
    source.source == ConstraintNodeSource::Source
        && source.surface == query.surface
        && source.pos == query.pos
}

fn continuation_proof(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &[Unit<'_>],
    support: &SupportCandidate,
    attached_nominal_allowed: bool,
) -> Option<MatchedContinuationProof> {
    let selected = continuation_units(pattern, spans, units, support, attached_nominal_allowed)?;
    Some(MatchedContinuationProof {
        proof: ConstraintContinuationProof {
            contract: pattern.continuation,
            units: selected
                .units
                .into_iter()
                .map(|unit| ConstraintMorphUnitProof {
                    pos: unit.pos.to_owned(),
                    span: unit.span.clone(),
                    source_node_index: unit.source_node_index,
                    component_index: unit.component_index,
                })
                .collect(),
        },
        attached_nominal: selected.attached_nominal,
    })
}

struct MatchedContinuationProof {
    proof: ConstraintContinuationProof,
    attached_nominal: Option<Range<usize>>,
}

struct MatchedContinuation<'units, 'data> {
    units: Vec<&'units Unit<'data>>,
    attached_nominal: Option<Range<usize>>,
}

fn continuation_units<'units, 'data>(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &'units [Unit<'data>],
    support: &SupportCandidate,
    attached_nominal_allowed: bool,
) -> Option<MatchedContinuation<'units, 'data>> {
    let suffix = spans.core.end..spans.consumed.end;
    let selected = suffix_unit_view(
        pattern,
        units,
        support,
        spans.anchor.clone(),
        suffix,
        attached_nominal_allowed,
    )?
    .iter()
    .collect::<Vec<_>>();
    match pattern.continuation {
        MorphContinuation::Exact => {
            (spans.anchor == spans.core && spans.consumed == spans.anchor && selected.is_empty())
                .then_some(MatchedContinuation {
                    units: selected,
                    attached_nominal: None,
                })
        }
        MorphContinuation::NominalParticles => ((spans.consumed == spans.anchor
            && selected.is_empty())
            || (spans.consumed.end == spans.token.end
                && nominal_continuation(&selected, units[support.unit_index].surface)))
        .then_some(MatchedContinuation {
            units: selected,
            attached_nominal: None,
        }),
        MorphContinuation::Predicate {
            state,
            nominal_particles,
        } => {
            if spans.consumed.end == spans.token.end
                && predicate_continuation(state, nominal_particles, spans, &selected)
            {
                return Some(MatchedContinuation {
                    units: selected,
                    attached_nominal: None,
                });
            }
            if !attached_nominal_allowed {
                return None;
            }
            match_attached_nominal_frame(state, nominal_particles, spans, &selected).map(
                |(predicate, selected)| MatchedContinuation {
                    units: predicate,
                    attached_nominal: Some(selected),
                },
            )
        }
    }
}

fn suffix_unit_view<'units, 'data>(
    pattern: &QueryMorphPattern,
    units: &'units [Unit<'data>],
    support: &SupportCandidate,
    anchor: Range<usize>,
    suffix: Range<usize>,
    include_attached_nominal: bool,
) -> Option<ContinuationUnitView<'units, 'data>> {
    let selected = ContinuationUnitView {
        units,
        start: support.unit_index.saturating_add(1).min(units.len()),
        anchor,
        suffix,
        include_opaque_nominalizer_tail: matches!(
            pattern.continuation,
            MorphContinuation::Predicate {
                nominal_particles: true,
                ..
            }
        ),
        include_opaque_adnominal_tail: include_attached_nominal,
        source_node_index: support.source_node_index,
        component_index: support.component_index,
    };
    if selected.suffix.start == selected.suffix.end {
        return Some(selected);
    }
    let mut has_selected = false;
    for unit in selected.iter() {
        has_selected = true;
        if unit.span.is_none() && !support.opaque {
            return None;
        }
        if !support.opaque
            && (unit.coverage.start < selected.suffix.start
                || unit.coverage.end > selected.suffix.end)
        {
            return None;
        }
    }
    if support.opaque {
        return has_selected.then_some(selected);
    }
    let mut end = selected.suffix.start;
    loop {
        let next_end = selected
            .iter()
            .filter(|unit| unit.coverage.start <= end)
            .map(|unit| unit.coverage.end)
            .max()
            .unwrap_or(end);
        if next_end == end {
            break;
        }
        end = next_end;
    }
    (end == selected.suffix.end).then_some(selected)
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
) -> bool {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Stage {
        Predicate,
        Nominalized,
        Terminal,
    }
    let mut stage = Stage::Predicate;
    for unit in units {
        match unit.pos {
            "EP" | "EC" | "VX" | "XSV" | "XSA" if stage == Stage::Predicate => {}
            "EF" | "ETM" if stage == Stage::Predicate => stage = Stage::Terminal,
            "ETN" if stage == Stage::Predicate => stage = Stage::Nominalized,
            pos if pos.starts_with('J') && nominal_particles && stage == Stage::Nominalized => {}
            _ => return false,
        }
    }
    let state_allowed = match state {
        ContinuationState::Terminal if nominal_particles => units
            .iter()
            .filter(|unit| unit.coverage.start >= spans.anchor.end)
            .all(|unit| unit.pos.starts_with('J')),
        ContinuationState::Terminal => !units
            .iter()
            .any(|unit| unit.coverage.start >= spans.anchor.end),
        ContinuationState::Eu => units
            .iter()
            .any(|unit| unit.coverage.start >= spans.anchor.end),
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
        ContextSelection::NominalParticleHosts { selected } => {
            let is_nominal = pattern.fine_pos.is_nominal();
            let is_predicate = is_predicate_pos(pattern.fine_pos);
            if is_nominal
                && let Some(selected) = selected.iter().find(|selected| spans.core == **selected)
            {
                Some(ContextMatch::NominalParticleHost { selected })
            } else if (is_nominal
                && selected.iter().any(|selected| {
                    selected.start <= spans.core.start
                        && spans.core.end < selected.end
                        && selected.end <= spans.consumed.end
                }))
                || (is_predicate && spans.core.start == 0)
                || spans.core == spans.token
                || (!is_nominal && !is_predicate)
            {
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
    let side = match (
        context.previous == Some(context.current),
        context.next == Some(context.current),
    ) {
        (true, true) => Some(AdjacentSide::Either),
        (true, false) => Some(AdjacentSide::Previous),
        (false, true) => Some(AdjacentSide::Next),
        (false, false) => None,
    }?;
    has_exact_pos(current, context.current.len(), "MAG").then_some(side)
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

fn nominal_particle_host_candidates(
    current_text: &str,
    current: &TokenGraph<'_>,
) -> Vec<Range<usize>> {
    if !has_complete_pos_matching(current, |pos| {
        source_pos(pos).is_some_and(DataFinePos::is_nominal)
    }) || !has_complete_pos_matching(current, |pos| pos.starts_with('J'))
    {
        return Vec::new();
    }
    let mut splits = BTreeSet::new();
    for (node_index, node) in current.nodes().iter().enumerate() {
        if !current.is_on_complete_path(node_index) || node.span.start != 0 {
            continue;
        }
        let units = path_units(&[node_index], current.nodes());
        if node.span.end < current_text.len() {
            let split = node.span.end;
            let prefix = units.iter().collect::<Vec<_>>();
            if let [host] = prefix.as_slice()
                && source_pos(host.pos).is_some_and(DataFinePos::is_nominal)
                && exact_coverage(&prefix, 0..split)
                && has_particle_suffix_path(
                    current,
                    node_index,
                    current_text.len(),
                    host.surface,
                    0,
                )
            {
                splits.insert(split);
            }
        }
        for (split, _) in current_text
            .char_indices()
            .skip(1)
            .take_while(|(split, _)| *split < node.span.end)
        {
            let prefix = units
                .iter()
                .filter(|unit| unit.coverage.end <= split)
                .collect::<Vec<_>>();
            let suffix = units
                .iter()
                .filter(|unit| unit.coverage.start >= split)
                .collect::<Vec<_>>();
            let Some(host) = prefix.first().filter(|_| prefix.len() == 1) else {
                continue;
            };
            let Some(case_particles) =
                particle_suffix_case_particles(&suffix, split..node.span.end, host.surface)
            else {
                continue;
            };
            if source_pos(host.pos).is_some_and(DataFinePos::is_nominal)
                && exact_coverage(&prefix, 0..split)
                && (node.span.end == current_text.len()
                    || has_particle_suffix_path(
                        current,
                        node_index,
                        current_text.len(),
                        host.surface,
                        case_particles,
                    ))
            {
                splits.insert(split);
            }
        }
    }
    splits.into_iter().map(|split| 0..split).collect()
}

fn has_particle_suffix_path(
    graph: &TokenGraph<'_>,
    host_index: usize,
    token_len: usize,
    host_surface: &str,
    case_particles: usize,
) -> bool {
    let mut stack = graph
        .successors(host_index)
        .iter()
        .copied()
        .map(|index| (index, case_particles))
        .collect::<Vec<_>>();
    let mut visited = vec![[false; 2]; graph.nodes().len()];
    while let Some((index, previous_case_particles)) = stack.pop() {
        if previous_case_particles > 1
            || visited[index][previous_case_particles]
            || !graph.is_on_complete_path(index)
        {
            continue;
        }
        visited[index][previous_case_particles] = true;
        let units = path_units(&[index], graph.nodes());
        let Some(case_particles) = particle_suffix_case_particles(
            &units.iter().collect::<Vec<_>>(),
            graph.nodes()[index].span.clone(),
            host_surface,
        ) else {
            continue;
        };
        let total_case_particles = previous_case_particles + case_particles;
        if total_case_particles > 1 {
            continue;
        }
        if graph.nodes()[index].span.end == token_len {
            return true;
        }
        stack.extend(
            graph
                .successors(index)
                .iter()
                .copied()
                .map(|next| (next, total_case_particles)),
        );
    }
    false
}

fn particle_suffix_case_particles(
    units: &[&Unit<'_>],
    expected: Range<usize>,
    host_surface: &str,
) -> Option<usize> {
    if units.is_empty()
        || units.iter().any(|unit| !unit.pos.starts_with('J'))
        || !enclosing_coverage(units, expected)
    {
        return None;
    }
    let host = host_surface.chars().next_back();
    let mut case_particles = 0_usize;
    for unit in units {
        case_particles += usize::from(is_case_particle(unit.pos, unit.surface));
        if case_particles > 1
            || host.is_some_and(|host| !particle_allomorph_accepts(unit.surface, host))
        {
            return None;
        }
    }
    Some(case_particles)
}

fn is_predicate_pos(pos: DataFinePos) -> bool {
    matches!(
        pos,
        DataFinePos::Vv | DataFinePos::Va | DataFinePos::Vx | DataFinePos::Vcp | DataFinePos::Vcn
    )
}

fn has_exact_pos(graph: &TokenGraph<'_>, token_len: usize, expected: &str) -> bool {
    graph.nodes().iter().enumerate().any(|(index, node)| {
        graph.is_on_complete_path(index)
            && node.span == (0..token_len)
            && node.pos.split('+').eq(std::iter::once(expected))
    })
}

fn has_complete_pos(graph: &TokenGraph<'_>, expected: &str) -> bool {
    has_complete_pos_matching(graph, |pos| pos == expected)
}

fn has_complete_pos_matching(graph: &TokenGraph<'_>, accepts: impl Fn(&str) -> bool) -> bool {
    graph.nodes().iter().enumerate().any(|(index, node)| {
        graph.is_on_complete_path(index)
            && (node.pos.split('+').any(&accepts)
                || node
                    .components
                    .iter()
                    .any(|component| accepts(component.pos)))
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

fn path_units<'a>(path: &[usize], nodes: &[Node<'a>]) -> Vec<Unit<'a>> {
    let mut units = Vec::new();
    for (node_position, &node_index) in path.iter().enumerate() {
        append_node_units(&mut units, node_position, node_index, nodes);
    }
    units
}

fn append_node_units<'a>(
    units: &mut Vec<Unit<'a>>,
    node_position: usize,
    node_index: usize,
    nodes: &[Node<'a>],
) {
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
            source: node.source,
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
                    source: node.source,
                }),
        );
    }
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
