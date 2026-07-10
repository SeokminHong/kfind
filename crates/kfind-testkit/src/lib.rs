//! Fixture and reference helpers shared by kfind tests.

mod corpus;
mod gold;
mod reference;

pub use corpus::{
    CorpusConfig, CorpusConfigError, CorpusGenerateError, CorpusStats, generate_corpus_tree,
};
pub use gold::{
    GoldCaseError, GoldCaseOutcome, GoldHarness, fixture_coarse_pos, load_morphology_cases,
};
pub use reference::{ReferenceMatcher, ReferenceMatcherBuildError, ReferenceMatcherError};
