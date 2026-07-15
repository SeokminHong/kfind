use std::ops::Range;
use std::sync::{Arc, OnceLock};

use kfind_data::{DataFinePos, MorphologyGraphExpressionKind, MorphologyGraphResource};

use crate::FinePos;
use crate::lattice::LocalLatticeError;
use crate::lattice::unknown::UnknownDictionary;

mod paths;

use paths::TokenGraph;

pub const DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompoundExposureProfile {
    Opaque,
    Transparent,
    Explicit,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QueryMorphPattern {
    pub fine_pos: DataFinePos,
    pub lexical_form: Arc<str>,
    pub expose_source_components: bool,
}

impl QueryMorphPattern {
    #[must_use]
    pub fn from_fine_pos(fine_pos: FinePos, lexical_form: &str) -> Vec<Self> {
        let fine_pos = match fine_pos {
            FinePos::CommonNoun => DataFinePos::Nng,
            FinePos::ProperNoun => DataFinePos::Nnp,
            FinePos::DependentNoun => DataFinePos::Nnb,
            FinePos::Pronoun => DataFinePos::Np,
            FinePos::Numeral => DataFinePos::Nr,
            FinePos::Verb => DataFinePos::Vv,
            FinePos::Adjective => {
                return vec![
                    Self::new(DataFinePos::Va, lexical_form),
                    Self::new(DataFinePos::Vcn, lexical_form),
                ];
            }
            FinePos::AuxiliaryVerb | FinePos::AuxiliaryAdjective => DataFinePos::Vx,
            FinePos::Copula => DataFinePos::Vcp,
            FinePos::Determiner => DataFinePos::Mm,
            FinePos::GeneralAdverb => DataFinePos::Mag,
            FinePos::ConjunctiveAdverb => DataFinePos::Maj,
            FinePos::Interjection => DataFinePos::Ic,
            FinePos::Particle
            | FinePos::Foreign
            | FinePos::Number
            | FinePos::Code
            | FinePos::Literal => return Vec::new(),
        };
        vec![Self::new(fine_pos, lexical_form)]
    }

    fn new(fine_pos: DataFinePos, lexical_form: &str) -> Self {
        Self {
            fine_pos,
            lexical_form: Arc::from(lexical_form),
            expose_source_components: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintAmbiguity {
    CompoundExposure,
    CompetingAnalyses,
    OpaqueExpression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintUnavailable {
    InvalidPattern,
    InvalidUnknownModel,
    NodeLimit { actual: usize, limit: usize },
    NoCompletePath,
    UnknownOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintVerdict {
    Proven,
    Contradicted,
    Ambiguous(ConstraintAmbiguity),
    Unavailable(ConstraintUnavailable),
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
    pub span: Range<usize>,
    pub pos: String,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
    pub source: ConstraintNodeSource,
    pub analysis_type: Option<String>,
    pub expression_kind: Option<MorphologyGraphExpressionKind>,
    pub matches_query_node: bool,
    pub matches_source_component: bool,
    pub has_opaque_expression: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintPathProof {
    pub evidence: ConstraintEvidenceKind,
    pub nodes: Vec<ConstraintNodeProof>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintProof {
    pub known_node_count: usize,
    pub unknown_node_count: usize,
    pub paths: Vec<ConstraintPathProof>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintResolution {
    pub verdict: ConstraintVerdict,
    pub proof: ConstraintProof,
}

impl ConstraintResolution {
    #[must_use]
    pub fn verdict_for(
        &self,
        profile: CompoundExposureProfile,
        patterns: &[QueryMorphPattern],
    ) -> ConstraintVerdict {
        if self.verdict != ConstraintVerdict::Ambiguous(ConstraintAmbiguity::CompoundExposure) {
            return self.verdict;
        }
        match profile {
            CompoundExposureProfile::Opaque => ConstraintVerdict::Contradicted,
            CompoundExposureProfile::Transparent => ConstraintVerdict::Proven,
            CompoundExposureProfile::Explicit
                if patterns
                    .iter()
                    .any(|pattern| pattern.expose_source_components) =>
            {
                ConstraintVerdict::Proven
            }
            CompoundExposureProfile::Explicit => ConstraintVerdict::Contradicted,
        }
    }
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
        if patterns.is_empty()
            || !valid_span(text, &target)
            || !valid_span(text, &candidate)
            || candidate.start > target.start
            || target.end > candidate.end
        {
            return unavailable(ConstraintUnavailable::InvalidPattern, 0, 0, Vec::new());
        }
        let known = match TokenGraph::known(&self.resource, text, &target, patterns, node_limit) {
            Ok(graph) => graph,
            Err(actual) => {
                return unavailable(
                    ConstraintUnavailable::NodeLimit {
                        actual,
                        limit: node_limit,
                    },
                    actual,
                    0,
                    Vec::new(),
                );
            }
        };
        if known.has_complete_paths() {
            return resolve_known(known, text, &candidate);
        }
        let unknown = match self.unknown() {
            Ok(unknown) => unknown,
            Err(_) => {
                return unavailable(
                    ConstraintUnavailable::InvalidUnknownModel,
                    known.node_count(),
                    0,
                    Vec::new(),
                );
            }
        };
        let fallback = match TokenGraph::with_unknown(
            &self.resource,
            text,
            &target,
            patterns,
            unknown,
            node_limit,
        ) {
            Ok(graph) => graph,
            Err(actual) => {
                return unavailable(
                    ConstraintUnavailable::NodeLimit {
                        actual,
                        limit: node_limit,
                    },
                    known.node_count(),
                    actual.saturating_sub(known.node_count()),
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
            fallback.proof_paths(text.len()),
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

fn valid_span(text: &str, span: &Range<usize>) -> bool {
    span.start < span.end
        && span.end <= text.len()
        && text.is_char_boundary(span.start)
        && text.is_char_boundary(span.end)
}

fn resolve_known(graph: TokenGraph, text: &str, candidate: &Range<usize>) -> ConstraintResolution {
    let paths = graph.proof_paths(text.len());
    let source_whole_paths = paths
        .iter()
        .filter(|path| is_source_whole_path(path, text.len()))
        .collect::<Vec<_>>();
    let selected = if source_whole_paths.is_empty() {
        paths.iter().collect::<Vec<_>>()
    } else {
        source_whole_paths
    };
    let has_component = selected
        .iter()
        .any(|path| path.nodes.iter().any(|node| node.matches_source_component));
    let has_exact = selected
        .iter()
        .any(|path| path.nodes.iter().any(|node| node.matches_query_node));
    let has_opaque = selected
        .iter()
        .any(|path| path.nodes.iter().any(|node| node.has_opaque_expression));
    let strict_candidate = *candidate != (0..text.len());
    let opaque_support = !strict_candidate && has_opaque;
    let has_support = has_component || has_exact || opaque_support;
    let verdict = if strict_candidate && (has_component || has_exact) {
        ConstraintVerdict::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    } else if has_support {
        ConstraintVerdict::Proven
    } else if has_opaque {
        ConstraintVerdict::Ambiguous(ConstraintAmbiguity::OpaqueExpression)
    } else {
        ConstraintVerdict::Contradicted
    };
    ConstraintResolution {
        verdict,
        proof: ConstraintProof {
            known_node_count: graph.node_count(),
            unknown_node_count: 0,
            paths,
        },
    }
}

fn is_source_whole_path(path: &ConstraintPathProof, text_len: usize) -> bool {
    path.nodes.len() == 1
        && path.nodes[0].source == ConstraintNodeSource::Source
        && path.nodes[0].span == (0..text_len)
}

fn unavailable(
    reason: ConstraintUnavailable,
    known_node_count: usize,
    unknown_node_count: usize,
    paths: Vec<ConstraintPathProof>,
) -> ConstraintResolution {
    ConstraintResolution {
        verdict: ConstraintVerdict::Unavailable(reason),
        proof: ConstraintProof {
            known_node_count,
            unknown_node_count,
            paths,
        },
    }
}

#[cfg(test)]
mod tests;
