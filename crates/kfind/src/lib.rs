//! In-memory Korean lemma and inflection matching.
//!
//! [`Engine`] owns reusable lexicon state. Compile a query once into a [`Matcher`],
//! then search any number of UTF-8 byte slices without filesystem or CLI dependencies.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use kfind_matcher::{MorphMatcher, MorphMatcherBuildError};
use kfind_query::{LexiconQueryAnalyzer, compile_query};

pub use kfind_data::DataError;
pub use kfind_query::{CompileError, CompileOptions, Lexicons, PhraseMatch, QueryPlan};

/// Reusable lexicon and query-analysis state.
#[derive(Clone, Debug)]
pub struct Engine {
    analyzer: LexiconQueryAnalyzer,
}

impl Engine {
    /// Creates an engine from the embedded core lexicon and morphology rules.
    pub fn embedded() -> Result<Self, DataError> {
        Lexicons::embedded().map(Self::from_lexicons)
    }

    /// Creates an engine with the embedded data and a decoded full POS lexicon.
    pub fn with_full_pos(full_pos: &[u8]) -> Result<Self, DataError> {
        Lexicons::embedded_with(Some(full_pos), None).map(Self::from_lexicons)
    }

    /// Creates an engine from caller-configured lexicons.
    #[must_use]
    pub fn from_lexicons(lexicons: Lexicons) -> Self {
        Self {
            analyzer: LexiconQueryAnalyzer::new(Arc::new(lexicons)),
        }
    }

    /// Reports whether this engine includes the optional full POS lexicon.
    #[must_use]
    pub fn full_pos_loaded(&self) -> bool {
        self.analyzer.lexicons().full_pos_loaded()
    }

    /// Compiles a query into a matcher that can be reused across inputs.
    pub fn compile(
        &self,
        query: &str,
        options: &CompileOptions,
    ) -> Result<Matcher, CompileMatcherError> {
        let plan = compile_query(query, options, &self.analyzer)?;
        MorphMatcher::new(Arc::new(plan))
            .map(Matcher::from)
            .map_err(CompileMatcherError::from)
    }
}

/// A compiled query that searches UTF-8 byte slices.
#[derive(Debug)]
pub struct Matcher {
    inner: MorphMatcher,
}

impl Matcher {
    /// Returns the compiled query plan used by this matcher.
    #[must_use]
    pub fn plan(&self) -> &QueryPlan {
        self.inner.plan()
    }

    /// Finds the next match at or after an absolute byte offset.
    #[must_use]
    pub fn find_at(&self, input: &[u8], at: usize) -> Option<PhraseMatch> {
        self.inner.find_at_with_meta(input, at)
    }

    /// Finds all non-overlapping matches with morphology provenance.
    #[must_use]
    pub fn find_all(&self, input: &[u8]) -> Vec<PhraseMatch> {
        self.inner.find_all_with_meta(input)
    }
}

impl From<MorphMatcher> for Matcher {
    fn from(inner: MorphMatcher) -> Self {
        Self { inner }
    }
}

/// Failure to compile a query plan or construct its anchor matcher.
#[derive(Debug)]
pub enum CompileMatcherError {
    Compile(CompileError),
    Matcher(MorphMatcherBuildError),
}

impl Display for CompileMatcherError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(error) => Display::fmt(error, formatter),
            Self::Matcher(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CompileMatcherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Compile(error) => Some(error),
            Self::Matcher(error) => Some(error),
        }
    }
}

impl From<CompileError> for CompileMatcherError {
    fn from(error: CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<MorphMatcherBuildError> for CompileMatcherError {
    fn from(error: MorphMatcherBuildError) -> Self {
        Self::Matcher(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_engine_compiles_and_reuses_a_matcher() {
        let engine = Engine::embedded().unwrap();
        let matcher = engine.compile("걷다", &CompileOptions::default()).unwrap();
        let text = "길을 걸어 갔다. 다시 걸었다.";

        let matches = matcher.find_all(text.as_bytes());

        assert_eq!(matches.len(), 2);
        assert_eq!(&text[matches[0].span.clone()], "걸어");
        assert_eq!(&text[matches[1].span.clone()], "걸었다");
        assert!(
            matches
                .iter()
                .all(|matched| { matched.atoms.iter().all(|atom| !atom.origins.is_empty()) })
        );
    }

    #[test]
    fn find_at_uses_an_absolute_byte_offset() {
        let engine = Engine::embedded().unwrap();
        let matcher = engine.compile("걷다", &CompileOptions::default()).unwrap();
        let text = "걸었다. 걸었다.";
        let second_start = "걸었다. ".len();

        let matched = matcher.find_at(text.as_bytes(), second_start).unwrap();

        assert_eq!(matched.span.start, second_start);
        assert_eq!(&text[matched.span], "걸었다");
    }

    #[test]
    fn invalid_full_pos_data_fails_during_engine_creation() {
        let error = Engine::with_full_pos(b"not a lexicon").unwrap_err();

        assert!(error.to_string().contains("binary"));
    }

    #[test]
    fn invalid_query_preserves_the_compile_error() {
        let engine = Engine::embedded().unwrap();
        let error = engine.compile("", &CompileOptions::default()).unwrap_err();

        assert!(matches!(error, CompileMatcherError::Compile(_)));
    }
}
