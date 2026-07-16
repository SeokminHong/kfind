use std::ops::Range;
use std::sync::Arc;

use kfind_morph::{
    CandidateTokenRelation, ComponentCapability, ContinuationState, MorphContinuation,
    ParticleTransition, PredicateFlags, PredicatePos, PredicateStemClass, QueryMorphPattern,
    RuleId,
};

use crate::{Analysis, BoundaryPolicy, Morphology, NormalizationMode, PhrasePolicy, PlanLimits};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPlan {
    pub raw_query: Box<str>,
    pub atoms: Vec<AtomPlan>,
    pub phrase_policy: PhrasePolicy,
    pub normalization: NormalizationMode,
    pub limits: PlanLimits,
    pub diagnostics: Vec<QueryDiagnostic>,
    pub particle_transitions: Arc<[ParticleTransition]>,
    pub estimated_matcher_bytes: usize,
}

impl QueryPlan {
    #[must_use]
    pub fn requires_component_resource(&self) -> bool {
        self.atoms.iter().any(|atom| {
            atom.programs
                .iter()
                .any(|program| program.decision.is_structural())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomPlan {
    pub analyses: Vec<Analysis>,
    pub programs: Vec<CandidateProgram>,
    pub boundary: BoundaryPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateConsumption {
    Anchor,
    PredicateContinuation {
        continuation: ContinuationState,
        pos: PredicatePos,
        allowed_rule_ids: Arc<[RuleId]>,
        nominal_particle_transition: bool,
        left_context: CandidateLeftContext,
    },
    StructuralPredicateEnding {
        pos: PredicatePos,
        flags: PredicateFlags,
        base_state: ContinuationState,
        validate_anchor: bool,
        stem_class: PredicateStemClass,
        allowed_suffixes: Arc<[Box<str>]>,
    },
    NominalParticleChain {
        allowed_rule_ids: Arc<[RuleId]>,
        blocked_rule_ids: Arc<[RuleId]>,
    },
    DirectParticleHost {
        rule_id: RuleId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateLeftContext {
    Any,
    ContractedAfterVowel { uncontracted_prefix: Box<str> },
}

impl CandidateConsumption {
    #[must_use]
    pub fn allows_rule_path(&self, rules: &[RuleId]) -> bool {
        match self {
            Self::Anchor | Self::DirectParticleHost { .. } => rules.is_empty(),
            Self::PredicateContinuation {
                allowed_rule_ids, ..
            } => rules.iter().all(|rule| {
                allowed_rule_ids
                    .binary_search_by_key(&rule.as_str(), |known| known.as_str())
                    .is_ok()
            }),
            Self::StructuralPredicateEnding { .. } => rules
                .iter()
                .all(|rule| rule.as_str() == "structural.ending-path"),
            Self::NominalParticleChain {
                allowed_rule_ids,
                blocked_rule_ids,
            } => rules.iter().all(|rule| {
                allowed_rule_ids
                    .binary_search_by_key(&rule.as_str(), |known| known.as_str())
                    .is_ok()
                    && blocked_rule_ids
                        .binary_search_by_key(&rule.as_str(), |blocked| blocked.as_str())
                        .is_err()
            }),
        }
    }

    #[must_use]
    pub fn allows_structural_suffix(&self, suffix: &str) -> bool {
        let Self::StructuralPredicateEnding {
            allowed_suffixes, ..
        } = self
        else {
            return false;
        };
        allowed_suffixes
            .binary_search_by_key(&suffix, |known| known.as_ref())
            .is_ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreMapping {
    WholeAnchor,
    PrefixBytes(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateProgram {
    pub anchor: Box<[u8]>,
    pub core_mapping: CoreMapping,
    pub consumption: CandidateConsumption,
    pub origins: Vec<Origin>,
    pub decision: CandidateDecision,
}

impl CandidateProgram {
    #[must_use]
    pub fn structural_patterns(&self) -> &[QueryMorphPattern] {
        match &self.decision {
            CandidateDecision::Boundary(_) => &[],
            CandidateDecision::Structural(constraint) => &constraint.patterns,
        }
    }

    #[must_use]
    pub fn boundary(&self) -> BoundaryProof {
        self.decision.boundary()
    }

    pub fn set_boundary(&mut self, boundary: BoundaryProof) {
        match &mut self.decision {
            CandidateDecision::Boundary(current) => *current = boundary,
            CandidateDecision::Structural(constraint) => constraint.boundary = boundary,
        }
    }

    pub fn apply_structural_constraint(
        &mut self,
        analyses: &[Analysis],
        component_capability: ComponentCapability,
    ) {
        self.decision = CandidateDecision::Structural(StructuralConstraint {
            patterns: self.query_morph_patterns(analyses, component_capability),
            boundary: self.boundary(),
        });
    }

    fn query_morph_patterns(
        &self,
        analyses: &[Analysis],
        component_capability: ComponentCapability,
    ) -> Vec<QueryMorphPattern> {
        let Some((token_relation, continuation)) = self.morph_contract() else {
            return Vec::new();
        };
        let mut patterns = Vec::new();
        for origin in &self.origins {
            let Some(analysis) = analyses.get(usize::from(origin.analysis_index)) else {
                continue;
            };
            let lexical_form = match &analysis.morphology {
                Morphology::Predicate(predicate) => predicate
                    .lemma
                    .strip_suffix('다')
                    .unwrap_or(&predicate.lemma),
                Morphology::Nominal(_) | Morphology::Particle(_) | Morphology::Exact => {
                    &analysis.lemma
                }
            };
            for pattern in QueryMorphPattern::from_fine_pos(analysis.fine_pos, lexical_form) {
                let pattern = pattern.with_candidate_contract(
                    token_relation,
                    continuation,
                    component_capability,
                );
                if !patterns.contains(&pattern) {
                    patterns.push(pattern);
                }
            }
        }
        patterns
    }

    fn morph_contract(&self) -> Option<(CandidateTokenRelation, MorphContinuation)> {
        Some(match &self.consumption {
            CandidateConsumption::Anchor => {
                (CandidateTokenRelation::Whole, MorphContinuation::Exact)
            }
            CandidateConsumption::PredicateContinuation {
                continuation,
                nominal_particle_transition,
                ..
            } => (
                CandidateTokenRelation::PrefixWithContinuation,
                MorphContinuation::Predicate {
                    state: *continuation,
                    nominal_particles: *nominal_particle_transition,
                },
            ),
            CandidateConsumption::StructuralPredicateEnding { .. } => (
                CandidateTokenRelation::PrefixWithContinuation,
                MorphContinuation::Predicate {
                    state: ContinuationState::Terminal,
                    nominal_particles: false,
                },
            ),
            CandidateConsumption::NominalParticleChain { .. } => (
                CandidateTokenRelation::PrefixWithContinuation,
                MorphContinuation::NominalParticles,
            ),
            CandidateConsumption::DirectParticleHost { .. } => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateDecision {
    Boundary(BoundaryProof),
    Structural(StructuralConstraint),
}

impl CandidateDecision {
    #[must_use]
    pub const fn boundary(&self) -> BoundaryProof {
        match self {
            Self::Boundary(boundary) => *boundary,
            Self::Structural(constraint) => constraint.boundary,
        }
    }

    #[must_use]
    pub const fn is_structural(&self) -> bool {
        matches!(self, Self::Structural(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructuralConstraint {
    pub patterns: Vec<QueryMorphPattern>,
    pub boundary: BoundaryProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoundaryProof {
    pub require_left: bool,
    pub require_right: bool,
    pub one_scalar_anchor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Origin {
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSpan {
    pub core: Range<usize>,
    pub token: Range<usize>,
    pub origins: Vec<Origin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryDiagnostic {
    FullPosLexiconUnavailable,
    UnregisteredDaLiteralOnly { atom_index: usize, lemma: Box<str> },
    RuleVocabularyRestricted { excluded_rule_ids: Box<[RuleId]> },
}
