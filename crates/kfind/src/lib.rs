//! In-memory Korean lemma and inflection matching.
//!
//! [`Engine`] owns reusable lexicon state. Compile a query once into a [`Matcher`],
//! then search any number of UTF-8 byte slices without filesystem or CLI dependencies.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use kfind_data::{COMPONENT_RESOURCE_SOURCE_DIGEST, ComponentResource, decode_component_resource};
use kfind_matcher::{MorphMatcher, MorphMatcherBuildError};
use kfind_query::{LexiconQueryAnalyzer, compile_query};

pub use kfind_data::DataError;
pub use kfind_morph::CoarsePos;
pub use kfind_query::{
    BoundaryPolicy, CompileError, CompileOptionError, CompileOptionOverrides, CompileOptions,
    ExpandMode, Lexicons, NormalizationMode, PhraseMatch, QueryPlan, VerifiedSpan,
};

/// Reusable lexicon and query-analysis state.
#[derive(Clone, Debug)]
pub struct Engine {
    analyzer: LexiconQueryAnalyzer,
    component_resource: Arc<ComponentResource>,
}

impl Engine {
    /// Creates an engine with the embedded core lexicon and a component resource.
    pub fn new(component_resource: impl Into<Vec<u8>>) -> Result<Self, DataError> {
        Self::from_lexicons(component_resource, Lexicons::embedded()?)
    }

    /// Creates an engine with embedded data, a component resource, and a full POS lexicon.
    pub fn with_full_pos(
        component_resource: impl Into<Vec<u8>>,
        full_pos: &[u8],
    ) -> Result<Self, DataError> {
        Self::from_lexicons(
            component_resource,
            Lexicons::embedded_with(Some(full_pos), None)?,
        )
    }

    /// Creates an engine from a component resource and caller-configured lexicons.
    pub fn from_lexicons(
        component_resource: impl Into<Vec<u8>>,
        lexicons: Lexicons,
    ) -> Result<Self, DataError> {
        let component_resource = decode_component_resource(
            "component resource",
            component_resource.into(),
            &COMPONENT_RESOURCE_SOURCE_DIGEST,
        )?;
        Ok(Self {
            analyzer: LexiconQueryAnalyzer::new(Arc::new(lexicons)),
            component_resource: Arc::new(component_resource),
        })
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
        MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(&self.component_resource))
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
    use std::io::Cursor;

    use kfind_data::{
        MecabSourceMorphologyEntry, encode_component_resource, parse_mecab_connection_matrix,
    };

    use super::*;

    #[test]
    fn embedded_engine_compiles_and_reuses_a_matcher() {
        let engine = test_engine();
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
        let engine = test_engine();
        let matcher = engine.compile("걷다", &CompileOptions::default()).unwrap();
        let text = "걸었다. 걸었다.";
        let second_start = "걸었다. ".len();

        let matched = matcher.find_at(text.as_bytes(), second_start).unwrap();

        assert_eq!(matched.span.start, second_start);
        assert_eq!(&text[matched.span], "걸었다");
    }

    #[test]
    fn invalid_full_pos_data_fails_during_engine_creation() {
        let error = Engine::with_full_pos(component_resource(), b"not a lexicon").unwrap_err();

        assert!(error.to_string().contains("binary"));
    }

    #[test]
    fn invalid_component_data_fails_during_engine_creation() {
        let error = Engine::new(b"not a component resource".as_slice()).unwrap_err();

        assert!(error.to_string().contains("component resource"));
    }

    #[test]
    fn invalid_query_preserves_the_compile_error() {
        let engine = test_engine();
        let error = engine.compile("", &CompileOptions::default()).unwrap_err();

        assert!(matches!(error, CompileMatcherError::Compile(_)));
    }

    fn test_engine() -> Engine {
        Engine::new(component_resource()).unwrap()
    }

    fn component_resource() -> Vec<u8> {
        let matrix = parse_mecab_connection_matrix(
            "matrix.def",
            Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
        )
        .unwrap();
        encode_component_resource(
            COMPONENT_RESOURCE_SOURCE_DIGEST,
            &[MecabSourceMorphologyEntry {
                surface: "걷다".to_owned(),
                pos: "VV".to_owned(),
                left_id: 1,
                right_id: 1,
                word_cost: 0,
                analysis_type: "*".to_owned(),
                start_pos: "*".to_owned(),
                end_pos: "*".to_owned(),
                expression: "*".to_owned(),
            }],
            &matrix,
            b"DEFAULT 0 1 0\nHANGUL 0 1 2\n0xAC00..0xD7A3 HANGUL\n",
            b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
        )
        .unwrap()
    }
}
