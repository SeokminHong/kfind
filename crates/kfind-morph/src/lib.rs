//! Korean syllable operations, lexicons, and morphology rules.

mod constraint;
mod domain;
mod lattice;
mod structure;

pub mod hangul;
pub mod particle;
pub mod predicate;

pub use constraint::{
    AdjacentSide, AdjacentTokenConstraint, CandidateSpans, CandidateTokenRelation,
    ComponentCapability, CopularFrameRole, MorphContinuation, QueryMorphPattern,
    StructuralSignature,
};
pub use domain::{
    CoarsePos, ContinuationState, EndingCategory, EndingInitial, EndingSpec, FinePos,
    LexicalAlternation, MorphFeatureMask, Origin, PredicateDerivation, PredicateEntry,
    PredicateFlags, PredicatePos, PredicatePosSet, PredicateStemClass, RuleId, SurfaceBranchSpec,
    SurfaceOverride,
};
pub use hangul::{
    Syllable, add_final, compose_syllable, decompose_syllable, drop_final, drop_last_final,
    has_final, has_rieul_final, replace_final, replace_last_final, replace_last_vowel,
};
pub use lattice::{
    DEFAULT_LATTICE_NODE_LIMIT, LocalLatticeAnalysis, LocalLatticeDecision, LocalLatticeError,
    LocalLatticeNode, LocalLatticePath, LocalLatticeReport, LocalLatticeResource,
    evaluate_local_component_decision, evaluate_local_component_paths,
};
pub use particle::{
    FinalCondition, ParticleAllomorph, ParticleChainModel, ParticleKind, ParticleMatch,
    ParticleRole, ParticleTransition, ParticleVerifier,
};
pub use predicate::{
    GenerateError, PredicateContinuationMatch, generate_predicate_branches,
    generate_predicate_fallback_stems, verify_copula_surface_after_nominal,
    verify_predicate_continuation,
};
pub use structure::{
    BoundedTokenContext, ConstraintDecision, ConstraintOutcome, ConstraintResolver,
    ConstraintUnavailable, PreparedStructuralContext, PreparedTokenGraph, ProductPolicy,
    StructuralEvidence,
};
