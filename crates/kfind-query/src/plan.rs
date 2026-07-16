use std::ops::Range;
use std::sync::Arc;

use kfind_morph::{
    CandidateExtentPolicy, CandidateTokenRelation, ComponentCapability, ContinuationState,
    MorphContinuation, ParticleTransition, PredicatePos, QueryMorphPattern, RuleId,
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
                .any(|program| matches!(program.decision, CandidateDecision::Structural(_)))
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
pub enum BranchVerifier {
    Exact,
    Predicate {
        continuation: ContinuationState,
        pos: PredicatePos,
        allowed_rule_ids: Arc<[RuleId]>,
        nominal_particle_transition: bool,
        environment: BranchEnvironment,
    },
    NominalParticles {
        allowed_rule_ids: Arc<[RuleId]>,
        blocked_rule_ids: Arc<[RuleId]>,
    },
    DirectParticle {
        rule_id: RuleId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BranchEnvironment {
    Unrestricted,
    ContractedAfterVowel { uncontracted_prefix: Box<str> },
}

impl BranchVerifier {
    #[must_use]
    pub fn accepts_rule_path(&self, rules: &[RuleId]) -> bool {
        match self {
            Self::Exact => rules.is_empty(),
            Self::DirectParticle { .. } => rules.is_empty(),
            Self::Predicate {
                allowed_rule_ids, ..
            } => rules.iter().all(|rule| {
                allowed_rule_ids
                    .binary_search_by_key(&rule.as_str(), |known| known.as_str())
                    .is_ok()
            }),
            Self::NominalParticles {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreMapping {
    WholeAnchor,
    PrefixBytes(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateProgram {
    pub anchor: Box<[u8]>,
    pub verifier: BranchVerifier,
    pub core_mapping: CoreMapping,
    pub extent: CandidateExtentPolicy,
    pub origins: Vec<Origin>,
    pub boundary: BoundaryProof,
    pub decision: CandidateDecision,
}

impl CandidateProgram {
    #[must_use]
    pub fn structural_patterns(&self, atom: &AtomPlan) -> Vec<QueryMorphPattern> {
        if atom.boundary != BoundaryPolicy::Smart {
            return Vec::new();
        }
        let Some((token_relation, continuation)) = self.pattern_continuation() else {
            return Vec::new();
        };
        let CandidateDecision::Structural(component_capability) = self.decision else {
            return Vec::new();
        };
        let mut patterns = Vec::new();
        for origin in &self.origins {
            let Some(analysis) = atom.analyses.get(usize::from(origin.analysis_index)) else {
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

    fn pattern_continuation(&self) -> Option<(CandidateTokenRelation, MorphContinuation)> {
        Some(match &self.verifier {
            BranchVerifier::Exact => (CandidateTokenRelation::Whole, MorphContinuation::Exact),
            BranchVerifier::Predicate {
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
            BranchVerifier::NominalParticles { .. } => (
                CandidateTokenRelation::PrefixWithContinuation,
                MorphContinuation::NominalParticles,
            ),
            BranchVerifier::DirectParticle { .. } => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateDecision {
    Boundary,
    Structural(ComponentCapability),
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
    VerifierVocabularyRestricted { excluded_rule_ids: Box<[RuleId]> },
}
