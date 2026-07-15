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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

enum CandidateAnalysis {
    Known {
        graph: TokenGraph,
        context: resolution::ContextSelection,
    },
    Unavailable {
        reason: ConstraintUnavailable,
        known_node_count: usize,
        unknown_node_count: usize,
        proof: Option<TokenGraph>,
    },
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
        match self.analyze_candidate(context, &spans, patterns, node_limit, path_limit) {
            CandidateAnalysis::Known { graph, context } => {
                resolution::resolve_known(&graph, &spans, patterns, &context, path_limit)
            }
            CandidateAnalysis::Unavailable {
                reason,
                known_node_count,
                unknown_node_count,
                proof,
            } => unavailable(
                reason,
                known_node_count,
                unknown_node_count,
                proof
                    .as_ref()
                    .map_or_else(Vec::new, TokenGraph::proof_nodes),
                proof
                    .as_ref()
                    .map_or_else(Vec::new, TokenGraph::proof_paths),
            ),
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
        match self.analyze_candidate(context, &spans, patterns, node_limit, path_limit) {
            CandidateAnalysis::Known { graph, context } => {
                resolution::decide_known(&graph, &spans, patterns, &context, path_limit)
            }
            CandidateAnalysis::Unavailable { reason, .. } => ConstraintDecision {
                outcome: ConstraintOutcome::Unavailable(reason),
                supported: Vec::new(),
            },
        }
    }

    fn analyze_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        spans: &CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
        path_limit: usize,
    ) -> CandidateAnalysis {
        if patterns.is_empty()
            || !patterns.iter().all(QueryMorphPattern::is_well_formed)
            || !spans.is_valid_for(context.current)
            || spans.token != (0..context.current.len())
            || node_limit == 0
            || path_limit == 0
        {
            return CandidateAnalysis::Unavailable {
                reason: ConstraintUnavailable::InvalidPattern,
                known_node_count: 0,
                unknown_node_count: 0,
                proof: None,
            };
        }
        let known = match TokenGraph::known(&self.resource, context.current, node_limit) {
            Ok(graph) => graph,
            Err(error) => {
                return CandidateAnalysis::Unavailable {
                    reason: graph_error(error, node_limit),
                    known_node_count: graph_error_actual(error),
                    unknown_node_count: 0,
                    proof: None,
                };
            }
        };
        if known.has_complete_paths() {
            let (previous, next) = match (
                resolution::needs_copular_context(&known, patterns),
                context.previous,
                context.next,
            ) {
                (true, Some(previous), Some(next)) => {
                    let previous = match TokenGraph::known(&self.resource, previous, node_limit) {
                        Ok(graph) => graph,
                        Err(error) => {
                            return CandidateAnalysis::Unavailable {
                                reason: graph_error(error, node_limit),
                                known_node_count: known.node_count(),
                                unknown_node_count: 0,
                                proof: Some(known),
                            };
                        }
                    };
                    let next = match TokenGraph::known(&self.resource, next, node_limit) {
                        Ok(graph) => graph,
                        Err(error) => {
                            return CandidateAnalysis::Unavailable {
                                reason: graph_error(error, node_limit),
                                known_node_count: known.node_count(),
                                unknown_node_count: 0,
                                proof: Some(known),
                            };
                        }
                    };
                    (Some(previous), Some(next))
                }
                _ => (None, None),
            };
            let selection =
                resolution::select_context(context, &known, previous.as_ref(), next.as_ref());
            return CandidateAnalysis::Known {
                graph: known,
                context: selection,
            };
        }
        let unknown = match self.unknown() {
            Ok(unknown) => unknown,
            Err(_) => {
                return CandidateAnalysis::Unavailable {
                    reason: ConstraintUnavailable::InvalidUnknownModel,
                    known_node_count: known.node_count(),
                    unknown_node_count: 0,
                    proof: None,
                };
            }
        };
        let fallback =
            match TokenGraph::with_unknown(&self.resource, context.current, unknown, node_limit) {
                Ok(graph) => graph,
                Err(error) => {
                    return CandidateAnalysis::Unavailable {
                        reason: graph_error(error, node_limit),
                        known_node_count: known.node_count(),
                        unknown_node_count: graph_error_actual(error)
                            .saturating_sub(known.node_count()),
                        proof: None,
                    };
                }
            };
        let reason = if fallback.has_complete_paths() {
            ConstraintUnavailable::UnknownOnly
        } else {
            ConstraintUnavailable::NoCompletePath
        };
        CandidateAnalysis::Unavailable {
            reason,
            known_node_count: known.node_count(),
            unknown_node_count: fallback.unknown_node_count(),
            proof: Some(fallback),
        }
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
