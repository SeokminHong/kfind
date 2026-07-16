use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, ComponentResource, DataError, DataWarning, ExpectedMatch,
    FixturePos, MorphologyCase, decode_component_resource, parse_morphology_cases_tsv,
};
use kfind_matcher::{MorphMatcher, MorphMatcherBuildError};
use kfind_morph::CoarsePos;
use kfind_query::{
    CompileError, CompileOptionOverrides, CompileOptions, LexiconQueryAnalyzer, Lexicons,
    compile_query,
};

const FIXTURE_SOURCE: &str = "data/fixtures/morphology_cases.tsv";
const MORPHOLOGY_CASES: &str = include_str!("../../../data/fixtures/morphology_cases.tsv");

pub fn load_morphology_cases() -> Result<(Vec<MorphologyCase>, Vec<DataWarning>), DataError> {
    parse_morphology_cases_tsv(FIXTURE_SOURCE, MORPHOLOGY_CASES)
}

#[must_use]
pub const fn fixture_coarse_pos(pos: FixturePos) -> CoarsePos {
    match pos {
        FixturePos::Noun => CoarsePos::Noun,
        FixturePos::Pronoun => CoarsePos::Pronoun,
        FixturePos::Numeral => CoarsePos::Numeral,
        FixturePos::Verb => CoarsePos::Verb,
        FixturePos::Adjective | FixturePos::Copula => CoarsePos::Adjective,
        FixturePos::Determiner => CoarsePos::Determiner,
        FixturePos::Adverb => CoarsePos::Adverb,
        FixturePos::Particle => CoarsePos::Particle,
        FixturePos::Interjection => CoarsePos::Interjection,
        FixturePos::Literal => CoarsePos::Literal,
    }
}

#[derive(Debug, Clone)]
pub struct GoldHarness {
    analyzer: LexiconQueryAnalyzer,
    component_resource: Option<Arc<ComponentResource>>,
}

impl GoldHarness {
    pub fn embedded() -> Result<Self, DataError> {
        Ok(Self::new(Lexicons::embedded()?, None))
    }

    pub fn with_full_pos(full_pos: &[u8]) -> Result<Self, DataError> {
        Ok(Self::new(
            Lexicons::embedded_with(Some(full_pos), None)?,
            None,
        ))
    }

    pub fn with_component(component_resource: Vec<u8>) -> Result<Self, DataError> {
        let component_resource = decode_component_resource(
            "component resource",
            component_resource,
            &COMPONENT_RESOURCE_SOURCE_DIGEST,
        )?;
        Ok(Self::new(
            Lexicons::embedded()?,
            Some(Arc::new(component_resource)),
        ))
    }

    pub fn with_full_pos_and_component(
        component_resource: Vec<u8>,
        full_pos: &[u8],
    ) -> Result<Self, DataError> {
        let component_resource = decode_component_resource(
            "component resource",
            component_resource,
            &COMPONENT_RESOURCE_SOURCE_DIGEST,
        )?;
        Ok(Self::new(
            Lexicons::embedded_with(Some(full_pos), None)?,
            Some(Arc::new(component_resource)),
        ))
    }

    fn new(lexicons: Lexicons, component_resource: Option<Arc<ComponentResource>>) -> Self {
        Self {
            analyzer: LexiconQueryAnalyzer::new(Arc::new(lexicons)),
            component_resource,
        }
    }

    pub fn evaluate(&self, case: &MorphologyCase) -> Result<GoldCaseOutcome, GoldCaseError> {
        self.evaluate_with_pos(
            case,
            (case.pos != FixturePos::Literal).then(|| fixture_coarse_pos(case.pos)),
        )
    }

    pub fn evaluate_auto(&self, case: &MorphologyCase) -> Result<GoldCaseOutcome, GoldCaseError> {
        self.evaluate_with_pos(case, None)
    }

    pub fn auto_includes_expected_pos(&self, case: &MorphologyCase) -> Result<bool, GoldCaseError> {
        if case.pos == FixturePos::Literal {
            return Ok(true);
        }
        let plan = compile_query(&case.query, &CompileOptions::default(), &self.analyzer)
            .map_err(GoldCaseError::Compile)?;
        let expected = fixture_coarse_pos(case.pos);
        Ok(plan.atoms.len() == 1
            && plan.atoms[0]
                .analyses
                .iter()
                .any(|analysis| analysis.coarse_pos == expected))
    }

    pub fn find_all(&self, query: &str, text: &str) -> Result<Vec<Range<usize>>, GoldCaseError> {
        let plan = compile_query(query, &CompileOptions::default(), &self.analyzer)
            .map_err(GoldCaseError::Compile)?;
        let matcher = if let Some(resource) = &self.component_resource {
            MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(resource))
        } else {
            MorphMatcher::new(Arc::new(plan))
        }
        .map_err(GoldCaseError::Matcher)?;
        Ok(matcher
            .find_all_with_meta(text.as_bytes())
            .into_iter()
            .map(|matched| matched.span)
            .collect())
    }

    fn evaluate_with_pos(
        &self,
        case: &MorphologyCase,
        pos: Option<CoarsePos>,
    ) -> Result<GoldCaseOutcome, GoldCaseError> {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos,
            ..CompileOptionOverrides::default()
        })
        .expect("gold POS overrides never conflict");
        let plan =
            compile_query(&case.query, &options, &self.analyzer).map_err(GoldCaseError::Compile)?;
        let matcher = if let Some(resource) = &self.component_resource {
            MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(resource))
        } else {
            MorphMatcher::new(Arc::new(plan))
        }
        .map_err(GoldCaseError::Matcher)?;
        let actual_match = matcher.find_at_with_meta(case.text.as_bytes(), 0).is_some();
        Ok(GoldCaseOutcome {
            expected_match: case.expected == ExpectedMatch::Match,
            actual_match,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoldCaseOutcome {
    pub expected_match: bool,
    pub actual_match: bool,
}

impl GoldCaseOutcome {
    #[must_use]
    pub const fn matches_expectation(self) -> bool {
        self.expected_match == self.actual_match
    }
}

#[derive(Debug)]
pub enum GoldCaseError {
    Compile(CompileError),
    Matcher(MorphMatcherBuildError),
}

impl Display for GoldCaseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(error) => write!(formatter, "query compilation failed: {error}"),
            Self::Matcher(error) => write!(formatter, "matcher construction failed: {error}"),
        }
    }
}

impl Error for GoldCaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Compile(error) => Some(error),
            Self::Matcher(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use kfind_data::{MecabSourceMorphologyEntry, encode_component_resource};

    use super::*;

    const EXPECTED_CASES: usize = 588;

    #[test]
    fn embedded_morphology_gold_matches_expected() {
        let (cases, warnings) = load_morphology_cases().expect("gold fixture must be valid");
        assert_eq!(cases.len(), EXPECTED_CASES);
        assert!(
            warnings.is_empty(),
            "unexpected fixture warnings: {warnings:#?}"
        );

        let harness = GoldHarness::with_component(nonmatching_component_fixture())
            .expect("nonmatching component fixture is valid");
        let component_harness =
            GoldHarness::with_component(component_fixture()).expect("component fixture is valid");
        let mut failures = Vec::new();
        for case in &cases {
            let selected = if matches!(
                case.feature.as_str(),
                "nominal-component" | "exact-component" | "lexical-context" | "copula-structure"
            ) {
                &component_harness
            } else {
                &harness
            };
            match selected.evaluate(case) {
                Ok(outcome) if outcome.matches_expectation() => {}
                Ok(outcome) => failures.push(format!(
                    "query={:?} pos={:?} feature={} text={:?}: expected_match={}, actual_match={}",
                    case.query,
                    case.pos,
                    case.feature,
                    case.text,
                    outcome.expected_match,
                    outcome.actual_match,
                )),
                Err(error) => failures.push(format!(
                    "query={:?} pos={:?} feature={} text={:?}: {error}",
                    case.query, case.pos, case.feature, case.text,
                )),
            }
        }

        assert!(
            failures.is_empty(),
            "{} of {} morphology gold cases failed:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n"),
        );
    }

    #[test]
    fn fixture_pos_maps_copula_to_the_predicate_coarse_class() {
        assert_eq!(fixture_coarse_pos(FixturePos::Copula), CoarsePos::Adjective);
        assert_eq!(fixture_coarse_pos(FixturePos::Verb), CoarsePos::Verb);
        assert_eq!(fixture_coarse_pos(FixturePos::Literal), CoarsePos::Literal);
    }

    #[test]
    fn explicit_particle_gold_is_separate_from_untagged_allomorphs() {
        let (cases, warnings) = load_morphology_cases().expect("gold fixture must be valid");
        assert!(warnings.is_empty());
        let case = cases
            .iter()
            .find(|case| case.query == "는" && case.text == "권한은 관리자에게 있다.")
            .expect("particle allomorph fixture must exist");
        let harness = GoldHarness::embedded().expect("embedded lexicons must be valid");

        assert!(harness.auto_includes_expected_pos(case).unwrap());
        assert!(!harness.evaluate_auto(case).unwrap().actual_match);
        assert!(harness.evaluate(case).unwrap().matches_expectation());
    }

    fn component_fixture() -> Vec<u8> {
        let entries = [
            entry("사용자", "NNG", -5_000),
            entry("권한", "NNG", -5_000),
            expression_entry("사용자권한", "NNG+NNG", "사용자/NNG/*+권한/NNG/*", 5_000),
            entry("관리", "NNG", -5_000),
            expression_entry("권한관리", "NNG+NNG", "권한/NNG/*+관리/NNG/*", 5_000),
            entry("산", "NNG", -5_000),
            entry("길", "NNG", -5_000),
            entry("산길", "NNG", 5_000),
            entry("을", "JKO", 0),
            entry("매", "NNG", 0),
            entry("매일", "MAG", 0),
            entry("매일", "NNG", 0),
            entry("아니", "VCN", 0),
            entry("라", "EC", 0),
            entry("일", "VCP+ETM", 0),
            entry("동안", "NNG", 0),
            entry("동안", "MAG", 0),
            entry("이", "VCP", 0),
            entry("이", "JKS", 0),
            entry("이", "EF", 0),
            entry("었", "EP", 0),
            entry("습니다", "EF", 0),
            entry("끝", "NNG", 0),
            entry("인", "VCP+ETM", 0),
            entry("가", "EF", 0),
            entry("곳", "NNB", 0),
            entry("공학", "NNG", 0),
            entry("입", "VCP", 0),
            entry("니다", "EF", 0),
            expression_entry("입니다", "VCP+EF", "이/VCP/*+ᆸ니다/EF/*", 0),
            entry("것", "NNB", 0),
            entry("수", "NNB", 0),
            entry("자기", "NP", -5_000),
            entry("견해", "NNG", -5_000),
            entry("전자기", "NNG", -10_000),
            entry("둘", "NR", -5_000),
            entry("다", "MAG", -5_000),
            entry("아들둘레", "NNG", -10_000),
            entry("두", "MM", -5_000),
            entry("사람", "NNG", -5_000),
            entry("모두", "MAG", -10_000),
        ];
        encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries).unwrap()
    }

    fn nonmatching_component_fixture() -> Vec<u8> {
        encode_component_resource(
            COMPONENT_RESOURCE_SOURCE_DIGEST,
            &[entry("미사용", "NNG", 0)],
        )
        .unwrap()
    }

    fn entry(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
        expression_entry(surface, pos, "*", word_cost)
    }

    fn expression_entry(
        surface: &str,
        pos: &str,
        expression: &str,
        word_cost: i32,
    ) -> MecabSourceMorphologyEntry {
        MecabSourceMorphologyEntry {
            surface: surface.to_owned(),
            pos: pos.to_owned(),
            left_id: 1,
            right_id: 1,
            word_cost,
            analysis_type: "*".to_owned(),
            start_pos: "*".to_owned(),
            end_pos: "*".to_owned(),
            expression: expression.to_owned(),
        }
    }
}
