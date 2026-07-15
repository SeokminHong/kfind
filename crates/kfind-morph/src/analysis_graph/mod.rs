use std::ops::Range;
use std::sync::{Arc, OnceLock};

use kfind_data::{MorphologyGraphExpressionKind, MorphologyGraphResource};

use crate::lattice::LocalLatticeError;
use crate::lattice::unknown::UnknownDictionary;

mod paths;
mod pattern;
mod resolution;

use paths::{TokenGraph, TokenGraphError};
pub use pattern::{
    AdjacentSide, AdjacentTokenConstraint, CandidateSpans, CandidateTokenRelation,
    ComponentCapability, CopularFrameRole, MorphContinuation, QueryMorphPattern,
};
pub use resolution::{
    BoundedTokenContext, ConstraintContextProof, ConstraintContinuationProof, ConstraintDecision,
    ConstraintMorphUnitProof, ConstraintOutcome, ConstraintSpanRelation, ConstraintSupport,
    ProductPolicy, SupportedAnalysis, SupportedAnalysisSet,
};

pub const DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT: usize = 4_096;
pub const DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintAmbiguity {
    CompetingAnalyses,
    CompoundExposure,
    LexicalCompetition,
    OpaqueExpression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintUnavailable {
    InvalidPattern,
    InvalidUnknownModel,
    NodeLimit { actual: usize, limit: usize },
    PathLimit { actual: usize, limit: usize },
    NoCompletePath,
    UnknownOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintComponentProof {
    pub surface: String,
    pub pos: String,
    pub span: Option<Range<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConstraintEvidenceKind {
    SourceWhole,
    SourceComponent,
    RuntimeComposed,
    OpaqueExpression,
    Contradiction,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConstraintNodeSource {
    Source,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintNodeProof {
    pub surface: String,
    pub span: Range<usize>,
    pub pos: String,
    pub start_pos: String,
    pub end_pos: String,
    pub source: ConstraintNodeSource,
    pub expression_kind: Option<MorphologyGraphExpressionKind>,
    pub components: Vec<ConstraintComponentProof>,
    pub matches_query_node: bool,
    pub matches_source_component: bool,
    pub has_opaque_expression: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintPathProof {
    pub evidence: ConstraintEvidenceKind,
    pub node_indices: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintProof {
    pub known_node_count: usize,
    pub unknown_node_count: usize,
    pub nodes: Vec<ConstraintNodeProof>,
    pub paths: Vec<ConstraintPathProof>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintResolution {
    pub outcome: ConstraintOutcome,
    pub supported: SupportedAnalysisSet,
    pub proof: ConstraintProof,
}

impl ConstraintResolution {
    #[must_use]
    pub fn decision(&self) -> ConstraintDecision {
        ConstraintDecision::from_resolution(self)
    }
}

#[derive(Debug)]
enum PreparedTokenState<'a> {
    Known {
        graph: TokenGraph<'a>,
        summary: resolution::PreparedTokenSummary,
    },
    Unavailable {
        reason: ConstraintUnavailable,
        known_node_count: usize,
        unknown_node_count: usize,
        proof: Option<TokenGraph<'a>>,
    },
}

#[derive(Debug)]
pub struct PreparedTokenAnalysis<'a> {
    current: &'a str,
    node_limit: usize,
    state: PreparedTokenState<'a>,
}

#[derive(Debug)]
pub struct ConstraintResolver {
    resource: Arc<MorphologyGraphResource>,
    unknown: OnceLock<Result<UnknownDictionary, LocalLatticeError>>,
}

impl ConstraintResolver {
    #[must_use]
    pub fn new(resource: Arc<MorphologyGraphResource>) -> Self {
        Self {
            resource,
            unknown: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn resource(&self) -> &MorphologyGraphResource {
        &self.resource
    }

    #[must_use]
    pub fn resolve(
        &self,
        text: &str,
        target: Range<usize>,
        candidate: Range<usize>,
        pattern: &QueryMorphPattern,
        node_limit: usize,
    ) -> ConstraintResolution {
        self.resolve_patterns(
            text,
            target,
            candidate,
            std::slice::from_ref(pattern),
            node_limit,
        )
    }

    #[must_use]
    pub fn resolve_patterns(
        &self,
        text: &str,
        target: Range<usize>,
        candidate: Range<usize>,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> ConstraintResolution {
        self.resolve_candidate_with_limits(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: target.clone(),
                anchor: target,
                consumed: candidate,
                token: 0..text.len(),
            },
            patterns,
            node_limit,
            DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT,
        )
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> ConstraintResolution {
        self.resolve_candidate_with_limits(
            context,
            spans,
            patterns,
            node_limit,
            DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT,
        )
    }

    #[must_use]
    pub fn resolve_candidate_with_limits(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
        path_limit: usize,
    ) -> ConstraintResolution {
        let prepared = self.prepare_token(context.current, node_limit);
        self.resolve_prepared_candidate_with_limits(&prepared, context, spans, patterns, path_limit)
    }

    #[must_use]
    pub fn resolve_prepared_candidate(
        &self,
        prepared: &PreparedTokenAnalysis<'_>,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
    ) -> ConstraintResolution {
        self.resolve_prepared_candidate_with_limits(
            prepared,
            context,
            spans,
            patterns,
            DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT,
        )
    }

    #[must_use]
    pub fn resolve_prepared_candidate_with_limits(
        &self,
        prepared: &PreparedTokenAnalysis<'_>,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        path_limit: usize,
    ) -> ConstraintResolution {
        if !valid_candidate_request(prepared, context, &spans, patterns, path_limit) {
            return unavailable(
                ConstraintUnavailable::InvalidPattern,
                0,
                0,
                Vec::new(),
                Vec::new(),
            );
        }
        match &prepared.state {
            PreparedTokenState::Known { graph, summary } => {
                let selection = match self.select_prepared_context(
                    graph,
                    summary,
                    context,
                    &spans,
                    patterns,
                    prepared.node_limit,
                ) {
                    Ok(selection) => selection,
                    Err(reason) => {
                        return unavailable(
                            reason,
                            graph.node_count(),
                            0,
                            graph.proof_nodes(),
                            graph.proof_paths(),
                        );
                    }
                };
                resolution::resolve_known(graph, &spans, patterns, &selection, path_limit)
            }
            PreparedTokenState::Unavailable {
                reason,
                known_node_count,
                unknown_node_count,
                proof,
            } => {
                if matches!(
                    reason,
                    ConstraintUnavailable::UnknownOnly | ConstraintUnavailable::NoCompletePath
                ) && let Some(graph) = self.hybrid_prefix_graph(prepared, spans.core.start)
                {
                    let summary = resolution::prepare_token_summary();
                    let selection = match self.select_prepared_context(
                        &graph,
                        &summary,
                        context,
                        &spans,
                        patterns,
                        prepared.node_limit,
                    ) {
                        Ok(selection) => selection,
                        Err(reason) => {
                            return unavailable(
                                reason,
                                graph.node_count() - graph.unknown_node_count(),
                                graph.unknown_node_count(),
                                graph.proof_nodes(),
                                graph.proof_paths(),
                            );
                        }
                    };
                    return resolution::resolve_known(
                        &graph, &spans, patterns, &selection, path_limit,
                    );
                }
                unavailable(
                    *reason,
                    *known_node_count,
                    *unknown_node_count,
                    proof
                        .as_ref()
                        .map_or_else(Vec::new, TokenGraph::proof_nodes),
                    proof
                        .as_ref()
                        .map_or_else(Vec::new, TokenGraph::proof_paths),
                )
            }
        }
    }

    #[must_use]
    pub fn decide_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> ConstraintDecision {
        self.decide_candidate_with_limits(
            context,
            spans,
            patterns,
            node_limit,
            DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT,
        )
    }

    #[must_use]
    pub fn decide_candidate_with_limits(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
        path_limit: usize,
    ) -> ConstraintDecision {
        let prepared = self.prepare_token(context.current, node_limit);
        self.decide_prepared_candidate_with_limits(&prepared, context, spans, patterns, path_limit)
    }

    #[must_use]
    pub fn decide_prepared_candidate(
        &self,
        prepared: &PreparedTokenAnalysis<'_>,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
    ) -> ConstraintDecision {
        self.decide_prepared_candidate_with_limits(
            prepared,
            context,
            spans,
            patterns,
            DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT,
        )
    }

    #[must_use]
    pub fn decide_prepared_candidate_with_limits(
        &self,
        prepared: &PreparedTokenAnalysis<'_>,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        path_limit: usize,
    ) -> ConstraintDecision {
        if !valid_candidate_request(prepared, context, &spans, patterns, path_limit) {
            return ConstraintDecision {
                outcome: ConstraintOutcome::Unavailable(ConstraintUnavailable::InvalidPattern),
                supported: Vec::new(),
            };
        }
        match &prepared.state {
            PreparedTokenState::Known { graph, summary } => {
                let selection = match self.select_prepared_context(
                    graph,
                    summary,
                    context,
                    &spans,
                    patterns,
                    prepared.node_limit,
                ) {
                    Ok(selection) => selection,
                    Err(reason) => {
                        return ConstraintDecision {
                            outcome: ConstraintOutcome::Unavailable(reason),
                            supported: Vec::new(),
                        };
                    }
                };
                resolution::decide_known(graph, &spans, patterns, &selection, path_limit)
            }
            PreparedTokenState::Unavailable { reason, .. } => {
                if matches!(
                    reason,
                    ConstraintUnavailable::UnknownOnly | ConstraintUnavailable::NoCompletePath
                ) && let Some(graph) = self.hybrid_prefix_graph(prepared, spans.core.start)
                {
                    let summary = resolution::prepare_token_summary();
                    let selection = match self.select_prepared_context(
                        &graph,
                        &summary,
                        context,
                        &spans,
                        patterns,
                        prepared.node_limit,
                    ) {
                        Ok(selection) => selection,
                        Err(reason) => {
                            return ConstraintDecision {
                                outcome: ConstraintOutcome::Unavailable(reason),
                                supported: Vec::new(),
                            };
                        }
                    };
                    return resolution::decide_known(
                        &graph, &spans, patterns, &selection, path_limit,
                    );
                }
                ConstraintDecision {
                    outcome: ConstraintOutcome::Unavailable(*reason),
                    supported: Vec::new(),
                }
            }
        }
    }

    #[must_use]
    pub fn prepare_token<'a>(
        &'a self,
        current: &'a str,
        node_limit: usize,
    ) -> PreparedTokenAnalysis<'a> {
        if node_limit == 0 {
            return PreparedTokenAnalysis {
                current,
                node_limit,
                state: PreparedTokenState::Unavailable {
                    reason: ConstraintUnavailable::InvalidPattern,
                    known_node_count: 0,
                    unknown_node_count: 0,
                    proof: None,
                },
            };
        }
        let known = match TokenGraph::known(&self.resource, current, node_limit) {
            Ok(graph) => graph,
            Err(error) => {
                return PreparedTokenAnalysis {
                    current,
                    node_limit,
                    state: PreparedTokenState::Unavailable {
                        reason: graph_error(error, node_limit),
                        known_node_count: graph_error_actual(error),
                        unknown_node_count: 0,
                        proof: None,
                    },
                };
            }
        };
        if known.has_complete_paths() {
            let summary = resolution::prepare_token_summary();
            return PreparedTokenAnalysis {
                current,
                node_limit,
                state: PreparedTokenState::Known {
                    graph: known,
                    summary,
                },
            };
        }
        let unknown = match self.unknown() {
            Ok(unknown) => unknown,
            Err(_) => {
                return PreparedTokenAnalysis {
                    current,
                    node_limit,
                    state: PreparedTokenState::Unavailable {
                        reason: ConstraintUnavailable::InvalidUnknownModel,
                        known_node_count: known.node_count(),
                        unknown_node_count: 0,
                        proof: None,
                    },
                };
            }
        };
        let fallback = match TokenGraph::with_unknown(&self.resource, current, unknown, node_limit)
        {
            Ok(graph) => graph,
            Err(error) => {
                return PreparedTokenAnalysis {
                    current,
                    node_limit,
                    state: PreparedTokenState::Unavailable {
                        reason: graph_error(error, node_limit),
                        known_node_count: known.node_count(),
                        unknown_node_count: graph_error_actual(error)
                            .saturating_sub(known.node_count()),
                        proof: None,
                    },
                };
            }
        };
        let reason = if fallback.has_complete_paths() {
            ConstraintUnavailable::UnknownOnly
        } else {
            ConstraintUnavailable::NoCompletePath
        };
        PreparedTokenAnalysis {
            current,
            node_limit,
            state: PreparedTokenState::Unavailable {
                reason,
                known_node_count: known.node_count(),
                unknown_node_count: fallback.unknown_node_count(),
                proof: Some(fallback),
            },
        }
    }

    fn select_prepared_context(
        &self,
        current: &TokenGraph<'_>,
        summary: &resolution::PreparedTokenSummary,
        context: BoundedTokenContext<'_>,
        spans: &CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> Result<resolution::ContextSelection, ConstraintUnavailable> {
        let (previous, next) = match (
            resolution::needs_copular_context(current, patterns),
            context.previous,
            context.next,
        ) {
            (true, Some(previous), Some(next)) => {
                let previous = TokenGraph::known(&self.resource, previous, node_limit)
                    .map_err(|error| graph_error(error, node_limit))?;
                let next = TokenGraph::known(&self.resource, next, node_limit)
                    .map_err(|error| graph_error(error, node_limit))?;
                (Some(previous), Some(next))
            }
            _ => (None, None),
        };
        let particle_hosts = if resolution::needs_nominal_particle_context(patterns, spans) {
            summary.nominal_particle_hosts(context.current, current)
        } else {
            &[]
        };
        Ok(resolution::select_context(
            context,
            current,
            particle_hosts,
            previous.as_ref(),
            next.as_ref(),
        ))
    }

    fn hybrid_prefix_graph<'a>(
        &'a self,
        prepared: &PreparedTokenAnalysis<'a>,
        prefix_end: usize,
    ) -> Option<TokenGraph<'a>> {
        if prefix_end == 0 {
            return None;
        }
        let unknown = self.unknown().ok()?;
        let graph = TokenGraph::with_unknown_prefix(
            &self.resource,
            prepared.current,
            unknown,
            prefix_end,
            prepared.node_limit,
        )
        .ok()?;
        graph.has_complete_paths().then_some(graph)
    }

    fn unknown(&self) -> Result<&UnknownDictionary, &LocalLatticeError> {
        self.unknown
            .get_or_init(|| {
                UnknownDictionary::parse(
                    self.resource.char_def(),
                    self.resource.unk_def(),
                    self.resource.left_contexts(),
                    self.resource.right_contexts(),
                )
            })
            .as_ref()
    }
}

fn valid_candidate_request(
    prepared: &PreparedTokenAnalysis<'_>,
    context: BoundedTokenContext<'_>,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    path_limit: usize,
) -> bool {
    !patterns.is_empty()
        && patterns.iter().all(QueryMorphPattern::is_well_formed)
        && context.current == prepared.current
        && spans.is_valid_for(context.current)
        && spans.token == (0..context.current.len())
        && prepared.node_limit > 0
        && path_limit > 0
}

fn graph_error(error: TokenGraphError, node_limit: usize) -> ConstraintUnavailable {
    match error {
        TokenGraphError::NodeLimit { actual } => ConstraintUnavailable::NodeLimit {
            actual,
            limit: node_limit,
        },
    }
}

fn graph_error_actual(error: TokenGraphError) -> usize {
    match error {
        TokenGraphError::NodeLimit { actual } => actual,
    }
}

fn unavailable(
    reason: ConstraintUnavailable,
    known_node_count: usize,
    unknown_node_count: usize,
    nodes: Vec<ConstraintNodeProof>,
    paths: Vec<ConstraintPathProof>,
) -> ConstraintResolution {
    ConstraintResolution {
        outcome: ConstraintOutcome::Unavailable(reason),
        supported: SupportedAnalysisSet::default(),
        proof: ConstraintProof {
            known_node_count,
            unknown_node_count,
            nodes,
            paths,
        },
    }
}

#[cfg(test)]
mod tests;
