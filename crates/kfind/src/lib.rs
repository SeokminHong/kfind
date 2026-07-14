//! In-memory Korean lemma and inflection matching.
//!
//! [`Engine`] owns reusable lexicon state. Compile a query once into a [`Matcher`],
//! then search any number of UTF-8 byte slices without filesystem or CLI dependencies.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use kfind_data::{COMPONENT_RESOURCE_SOURCE_DIGEST, decode_component_resource};
use kfind_matcher::{MorphMatcher, MorphMatcherBuildError};
use kfind_morph::LocalComponentEvaluator;
use kfind_query::{LexiconQueryAnalyzer, compile_query};

pub use kfind_data::DataError;
pub use kfind_morph::CoarsePos;
pub use kfind_query::{
    BoundaryPolicy, CompileError, CompileOptionError, CompileOptionOverrides, CompileOptions,
    ExpandMode, Lexicons, NormalizationMode, PhraseMatch, QueryPlan, VerifiedSpan,
};

/// Optional dictionary resources that define an engine's analysis profile.
#[derive(Clone, Debug, Default)]
pub struct ResourceBundle<'a> {
    /// Encoded full POS lexicon bytes.
    pub full_pos: Option<&'a [u8]>,
    /// UTF-8 enriched predicate metadata in the project TSV format.
    pub enriched_predicates: Option<&'a str>,
    /// Encoded compact component resource bytes.
    pub component: Option<Vec<u8>>,
}

/// Reusable lexicon and query-analysis state.
#[derive(Clone, Debug)]
pub struct Engine {
    analyzer: LexiconQueryAnalyzer,
    component_evaluator: Option<Arc<LocalComponentEvaluator>>,
}

impl Engine {
    /// Creates an engine with the embedded core lexicon.
    pub fn new() -> Result<Self, DataError> {
        Self::with_resources(ResourceBundle::default())
    }

    /// Creates an engine from one validated dictionary-resource profile.
    pub fn with_resources(resources: ResourceBundle<'_>) -> Result<Self, DataError> {
        let mut lexicons = Lexicons::embedded_with(resources.full_pos, None)?;
        if let Some(enriched_predicates) = resources.enriched_predicates {
            lexicons
                .load_enriched_predicates("enriched predicate resource", enriched_predicates)?;
        }
        match resources.component {
            Some(component) => Self::from_lexicons_with_component(lexicons, component),
            None => Ok(Self::from_lexicons(lexicons)),
        }
    }

    /// Creates an engine with embedded data and a component resource.
    pub fn with_component_resource(
        component_resource: impl Into<Vec<u8>>,
    ) -> Result<Self, DataError> {
        Self::with_resources(ResourceBundle {
            component: Some(component_resource.into()),
            ..ResourceBundle::default()
        })
    }

    /// Creates an engine with embedded data and a full POS lexicon.
    pub fn with_full_pos(full_pos: &[u8]) -> Result<Self, DataError> {
        Self::with_resources(ResourceBundle {
            full_pos: Some(full_pos),
            ..ResourceBundle::default()
        })
    }

    /// Creates an engine with embedded data, a full POS lexicon, and a component resource.
    pub fn with_full_pos_and_component(
        full_pos: &[u8],
        component_resource: impl Into<Vec<u8>>,
    ) -> Result<Self, DataError> {
        Self::with_resources(ResourceBundle {
            full_pos: Some(full_pos),
            component: Some(component_resource.into()),
            ..ResourceBundle::default()
        })
    }

    /// Creates an engine from caller-configured lexicons.
    #[must_use]
    pub fn from_lexicons(lexicons: Lexicons) -> Self {
        Self {
            analyzer: LexiconQueryAnalyzer::new(Arc::new(lexicons)),
            component_evaluator: None,
        }
    }

    /// Creates an engine from caller-configured lexicons and a component resource.
    pub fn from_lexicons_with_component(
        lexicons: Lexicons,
        component_resource: impl Into<Vec<u8>>,
    ) -> Result<Self, DataError> {
        Ok(Self {
            analyzer: LexiconQueryAnalyzer::new(Arc::new(lexicons)),
            component_evaluator: Some(decode_component(component_resource)?),
        })
    }

    /// Validates and installs a component resource for subsequent smart queries.
    pub fn load_component_resource(
        &mut self,
        component_resource: impl Into<Vec<u8>>,
    ) -> Result<(), DataError> {
        let component_evaluator = decode_component(component_resource)?;
        self.component_evaluator = Some(component_evaluator);
        Ok(())
    }

    /// Reports whether this engine includes the optional full POS lexicon.
    #[must_use]
    pub fn full_pos_loaded(&self) -> bool {
        self.analyzer.lexicons().full_pos_loaded()
    }

    /// Reports whether this engine includes enriched predicate metadata.
    #[must_use]
    pub fn enriched_predicates_loaded(&self) -> bool {
        self.analyzer.lexicons().enriched_predicates_loaded()
    }

    /// Reports whether this engine includes the optional component resource.
    #[must_use]
    pub fn component_resource_loaded(&self) -> bool {
        self.component_evaluator.is_some()
    }

    /// Compiles a query into a matcher that can be reused across inputs.
    pub fn compile(
        &self,
        query: &str,
        options: &CompileOptions,
    ) -> Result<Matcher, CompileMatcherError> {
        let plan = Arc::new(compile_query(query, options, &self.analyzer)?);
        let matcher = if plan.requires_component_resource() {
            let evaluator = self
                .component_evaluator
                .as_ref()
                .ok_or(CompileMatcherError::ComponentResourceRequired)?;
            MorphMatcher::with_component_evaluator(plan, Arc::clone(evaluator))
        } else {
            MorphMatcher::new(plan)
        }?;
        Ok(Matcher::from(matcher))
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
    ComponentResourceRequired,
    Matcher(MorphMatcherBuildError),
}

impl Display for CompileMatcherError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(error) => Display::fmt(error, formatter),
            Self::ComponentResourceRequired => {
                formatter.write_str("component resource is required for this smart query")
            }
            Self::Matcher(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CompileMatcherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Compile(error) => Some(error),
            Self::ComponentResourceRequired => None,
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

fn decode_component(
    component_resource: impl Into<Vec<u8>>,
) -> Result<Arc<LocalComponentEvaluator>, DataError> {
    decode_component_resource(
        "component resource",
        component_resource.into(),
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .map(Arc::new)
    .map(LocalComponentEvaluator::new)
    .map(Arc::new)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use kfind_data::{
        DataFinePos, LexiconData, MecabSourceMorphologyEntry, ModifierRecord, NominalRecord,
        collect_pos_entries, encode_component_resource, encode_pos_lexicon,
        parse_mecab_connection_matrix,
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
        let error = Engine::with_full_pos(b"not a lexicon").unwrap_err();

        assert!(error.to_string().contains("binary"));
    }

    #[test]
    fn invalid_component_data_fails_during_engine_creation() {
        let error =
            Engine::with_component_resource(b"not a component resource".as_slice()).unwrap_err();

        assert!(error.to_string().contains("component resource"));
    }

    #[test]
    fn resource_bundle_initializes_every_dictionary_layer() {
        let full_pos = encode_pos_lexicon(&collect_pos_entries(&LexiconData {
            nominals: vec![NominalRecord {
                lemma: "대규모사전".to_owned(),
                pos: DataFinePos::Nng,
                flags: Default::default(),
                overrides: Vec::new(),
            }],
            ..LexiconData::default()
        }))
        .unwrap();
        let enriched = concat!(
            "lemma\tpos\talternation\tflags\toverrides\n",
            "가깝다\tVA\tBToWo\t\t\n",
        );

        let engine = Engine::with_resources(ResourceBundle {
            full_pos: Some(&full_pos),
            enriched_predicates: Some(enriched),
            component: Some(component_resource()),
        })
        .unwrap();

        assert!(engine.full_pos_loaded());
        assert!(engine.enriched_predicates_loaded());
        assert!(engine.component_resource_loaded());

        let matcher = engine
            .compile(
                "가깝다",
                &CompileOptions {
                    boundary: BoundaryPolicy::Any,
                    ..CompileOptions::default()
                },
            )
            .unwrap();
        assert_eq!(matcher.find_all("더 가까워졌다".as_bytes()).len(), 1);
    }

    #[test]
    fn invalid_enriched_predicates_fail_during_engine_creation() {
        let error = Engine::with_resources(ResourceBundle {
            enriched_predicates: Some("lemma\tpos\naltered\tVV\n"),
            ..ResourceBundle::default()
        })
        .unwrap_err();

        assert!(error.to_string().contains("enriched predicate resource"));
    }

    #[test]
    fn invalid_query_preserves_the_compile_error() {
        let engine = test_engine();
        let error = engine.compile("", &CompileOptions::default()).unwrap_err();

        assert!(matches!(error, CompileMatcherError::Compile(_)));
    }

    #[test]
    fn component_smart_query_requires_explicit_initialization() {
        let mut engine = Engine::new().unwrap();
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos: Some(CoarsePos::Noun),
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        let error = engine.compile("권한", &options).unwrap_err();

        assert!(matches!(
            error,
            CompileMatcherError::ComponentResourceRequired
        ));

        engine
            .load_component_resource(component_resource())
            .unwrap();
        let matcher = engine.compile("권한", &options).unwrap();
        assert_eq!(matcher.find_all("권한".as_bytes()).len(), 1);
    }

    #[test]
    fn ordinary_smart_adverbs_do_not_require_component_initialization() {
        let engine = Engine::new().unwrap();

        for query in ["adv:빨리", "adv:매우"] {
            engine.compile(query, &CompileOptions::default()).unwrap();
        }
    }

    #[test]
    fn smart_copula_rejects_a_non_predicate_whole_token_without_changing_any() {
        let without_component = Engine::new().unwrap();
        let error = without_component
            .compile("이다", &CompileOptions::default())
            .unwrap_err();
        assert!(matches!(
            error,
            CompileMatcherError::ComponentResourceRequired
        ));

        let with_component = Engine::with_component_resource(component_resource()).unwrap();
        let smart = with_component
            .compile("이다", &CompileOptions::default())
            .unwrap();
        assert!(smart.find_all("매일".as_bytes()).is_empty());
        assert!(smart.find_all("그건 매일 수도 있어".as_bytes()).is_empty());
        assert_eq!(smart.find_all("학생일 수도 있어".as_bytes()).len(), 1);

        let any = without_component
            .compile(
                "이다",
                &CompileOptions {
                    boundary: BoundaryPolicy::Any,
                    ..CompileOptions::default()
                },
            )
            .unwrap();
        assert_eq!(any.find_all("매일".as_bytes()).len(), 1);
    }

    #[test]
    fn smart_context_selects_the_same_maeil_analysis_for_every_query() {
        let engine = contextual_engine();
        let copular = "독수리가 아니라 매일 것 같아";
        assert_eq!(matched_surfaces(&engine, "n:매", copular), ["매"]);
        for query in ["매일", "n:매일", "adv:매일"] {
            assert!(matched_surfaces(&engine, query, copular).is_empty());
        }
        assert_eq!(matched_surfaces(&engine, "이다", copular), ["일"]);

        let repeated = "매일 매일 보고 싶어";
        for query in ["n:매", "n:매일"] {
            assert!(matched_surfaces(&engine, query, repeated).is_empty());
        }
        assert_eq!(
            matched_surfaces(&engine, "매일", repeated),
            ["매일", "매일"]
        );
        assert_eq!(
            matched_surfaces(&engine, "adv:매일", repeated),
            ["매일", "매일"]
        );

        let object = "그는 집념으로 매일을 보내고 있었다.";
        assert_eq!(matched_surfaces(&engine, "매일", object), ["매일을"]);
        assert_eq!(matched_surfaces(&engine, "n:매일", object), ["매일을"]);
        for query in ["n:매", "adv:매일"] {
            assert!(matched_surfaces(&engine, query, object).is_empty());
        }
    }

    #[test]
    fn smart_context_does_not_cross_lines_or_analysis_limits() {
        let engine = contextual_engine();
        let line_break = "아니라\n매일 것";
        assert!(matched_surfaces(&engine, "n:매", line_break).is_empty());
        assert_eq!(matched_surfaces(&engine, "n:매일", line_break), ["매일"]);

        let distant_repeat = format!("매일{}매일", " ".repeat(257));
        assert_eq!(
            matched_surfaces(&engine, "n:매일", &distant_repeat),
            ["매일", "매일"]
        );
    }

    #[test]
    fn smart_context_ignores_invalid_utf8_outside_the_bounded_evidence() {
        let engine = contextual_engine();
        let matcher = engine
            .compile("n:매일", &CompileOptions::default())
            .unwrap();
        let mut text = vec![0xff];
        text.extend(std::iter::repeat_n(b' ', 300));
        text.extend_from_slice("매일 매일".as_bytes());

        assert!(matcher.find_all(&text).is_empty());
    }

    #[test]
    fn smart_context_falls_back_on_invalid_utf8_inside_the_bounded_evidence() {
        let engine = contextual_engine();
        let matcher = engine
            .compile("n:매일", &CompileOptions::default())
            .unwrap();
        let mut text = "매일 ".as_bytes().to_vec();
        text.push(0xff);
        text.extend_from_slice(" 매일".as_bytes());

        assert_eq!(matcher.find_all(&text).len(), 2);
    }

    #[test]
    fn explicit_component_initialization_is_observable() {
        let mut without_component = Engine::new().unwrap();
        let with_component = Engine::with_component_resource(component_resource()).unwrap();

        assert!(!without_component.component_resource_loaded());
        assert!(with_component.component_resource_loaded());

        without_component
            .load_component_resource(component_resource())
            .unwrap();
        assert!(without_component.component_resource_loaded());

        assert!(
            without_component
                .load_component_resource(b"not a component resource".as_slice())
                .is_err()
        );
        assert!(without_component.component_resource_loaded());
    }

    fn test_engine() -> Engine {
        Engine::new().unwrap()
    }

    fn contextual_engine() -> Engine {
        let full_pos = encode_pos_lexicon(&collect_pos_entries(&LexiconData {
            nominals: vec![
                NominalRecord {
                    lemma: "매".to_owned(),
                    pos: DataFinePos::Nng,
                    flags: Default::default(),
                    overrides: Vec::new(),
                },
                NominalRecord {
                    lemma: "매일".to_owned(),
                    pos: DataFinePos::Nng,
                    flags: Default::default(),
                    overrides: Vec::new(),
                },
            ],
            modifiers: vec![ModifierRecord {
                lemma: "매일".to_owned(),
                pos: DataFinePos::Mag,
                flags: Default::default(),
            }],
            ..LexiconData::default()
        }))
        .unwrap();
        Engine::with_full_pos_and_component(&full_pos, component_resource()).unwrap()
    }

    fn matched_surfaces(engine: &Engine, query: &str, text: &str) -> Vec<String> {
        engine
            .compile(query, &CompileOptions::default())
            .unwrap()
            .find_all(text.as_bytes())
            .into_iter()
            .map(|matched| text[matched.span].to_owned())
            .collect()
    }

    fn component_resource() -> Vec<u8> {
        let matrix = parse_mecab_connection_matrix(
            "matrix.def",
            Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
        )
        .unwrap();
        encode_component_resource(
            COMPONENT_RESOURCE_SOURCE_DIGEST,
            &[
                component_entry("걷다", "VV"),
                component_entry("매일", "MAG"),
                component_entry("매일", "NNG"),
                component_entry("매", "NNG"),
                component_entry("일", "VCP+ETM"),
                component_entry("아니", "VCN"),
                component_entry("라", "EC"),
                component_entry("것", "NNB"),
                component_entry("학생일", "NNG+VCP+ETM"),
            ],
            &matrix,
            b"DEFAULT 0 1 0\nHANGUL 0 1 2\n0xAC00..0xD7A3 HANGUL\n",
            b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
        )
        .unwrap()
    }

    fn component_entry(surface: &str, pos: &str) -> MecabSourceMorphologyEntry {
        MecabSourceMorphologyEntry {
            surface: surface.to_owned(),
            pos: pos.to_owned(),
            left_id: 1,
            right_id: 1,
            word_cost: 0,
            analysis_type: "*".to_owned(),
            start_pos: "*".to_owned(),
            end_pos: "*".to_owned(),
            expression: "*".to_owned(),
        }
    }
}
