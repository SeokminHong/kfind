use std::ops::Range;
use std::sync::Arc;

use kfind_morph::{ContinuationState, ParticleTransition, PredicatePos, RuleId};

use crate::{Analysis, BoundaryPolicy, NormalizationMode, PhrasePolicy, PlanLimits};

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
            atom.branches.iter().any(|branch| {
                matches!(
                    branch.context_requirement,
                    ContextRequirement::PredicateLexical
                        | ContextRequirement::ExactComponent
                        | ContextRequirement::LexicalContext
                )
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomPlan {
    pub analyses: Vec<Analysis>,
    pub branches: Vec<SurfaceBranch>,
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
pub struct SurfaceBranch {
    pub anchor: Box<[u8]>,
    pub verifier: BranchVerifier,
    pub core_mapping: CoreMapping,
    pub origins: Vec<Origin>,
    pub boundary: BoundaryProof,
    pub context_requirement: ContextRequirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextRequirement {
    None,
    PredicateLexical,
    ExactComponent,
    LexicalContext,
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
