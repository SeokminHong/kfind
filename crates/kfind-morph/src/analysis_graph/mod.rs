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
    BoundedTokenContext, ConstraintContextProof, ConstraintContinuationProof,
    ConstraintMorphUnitProof, ConstraintOutcome, ConstraintSpanRelation, ProductPolicy,
    SupportedAnalysis, SupportedAnalysisSet,
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
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
    pub source: ConstraintNodeSource,
    pub analysis_type: Option<String>,
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
        if patterns.is_empty()
            || !patterns.iter().all(QueryMorphPattern::is_well_formed)
            || !spans.is_valid_for(context.current)
            || spans.token != (0..context.current.len())
            || node_limit == 0
            || path_limit == 0
        {
            return unavailable(
                ConstraintUnavailable::InvalidPattern,
                0,
                0,
                Vec::new(),
                Vec::new(),
            );
        }
        let known = match TokenGraph::known(&self.resource, context.current, node_limit) {
            Ok(graph) => graph,
            Err(error) => {
                return unavailable(
                    graph_error(error, node_limit),
                    graph_error_actual(error),
                    0,
                    Vec::new(),
                    Vec::new(),
                );
            }
        };
        if known.has_complete_paths() {
            let (previous, next) = match (context.previous, context.next) {
                (Some(previous), Some(next)) => {
                    let previous = match TokenGraph::known(&self.resource, previous, node_limit) {
                        Ok(graph) => graph,
                        Err(error) => {
                            return unavailable(
                                graph_error(error, node_limit),
                                known.node_count(),
                                0,
                                known.proof_nodes(),
                                known.proof_paths(),
                            );
                        }
                    };
                    let next = match TokenGraph::known(&self.resource, next, node_limit) {
                        Ok(graph) => graph,
                        Err(error) => {
                            return unavailable(
                                graph_error(error, node_limit),
                                known.node_count(),
                                0,
                                known.proof_nodes(),
                                known.proof_paths(),
                            );
                        }
                    };
                    (Some(previous), Some(next))
                }
                _ => (None, None),
            };
            let selection =
                resolution::select_context(context, &known, previous.as_ref(), next.as_ref());
            return resolution::resolve_known(&known, &spans, patterns, &selection, path_limit);
        }
        let unknown = match self.unknown() {
            Ok(unknown) => unknown,
            Err(_) => {
                return unavailable(
                    ConstraintUnavailable::InvalidUnknownModel,
                    known.node_count(),
                    0,
                    Vec::new(),
                    Vec::new(),
                );
            }
        };
        let fallback =
            match TokenGraph::with_unknown(&self.resource, context.current, unknown, node_limit) {
                Ok(graph) => graph,
                Err(error) => {
                    return unavailable(
                        graph_error(error, node_limit),
                        known.node_count(),
                        graph_error_actual(error).saturating_sub(known.node_count()),
                        Vec::new(),
                        Vec::new(),
                    );
                }
            };
        let reason = if fallback.has_complete_paths() {
            ConstraintUnavailable::UnknownOnly
        } else {
            ConstraintUnavailable::NoCompletePath
        };
        unavailable(
            reason,
            known.node_count(),
            fallback.unknown_node_count(),
            fallback.proof_nodes(),
            fallback.proof_paths(),
        )
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
