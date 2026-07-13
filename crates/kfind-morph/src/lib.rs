//! Korean syllable operations, lexicons, and morphology rules.

mod domain;
mod lattice;

pub mod hangul;
pub mod particle;
pub mod predicate;

pub use domain::{
    CoarsePos, ContinuationState, EndingCategory, EndingInitial, EndingSpec, FinePos,
    LexicalAlternation, MorphFeatureMask, Origin, PredicateEntry, PredicateFlags, PredicatePos,
    RuleId, SurfaceBranchSpec, SurfaceOverride,
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
    verify_predicate_continuation,
};
