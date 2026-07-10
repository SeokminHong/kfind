//! Fixture and reference helpers shared by kfind tests.

mod corpus;

pub use corpus::{
    CorpusConfig, CorpusConfigError, CorpusGenerateError, CorpusStats, generate_corpus_tree,
};
