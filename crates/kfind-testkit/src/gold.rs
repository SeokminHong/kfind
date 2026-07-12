use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use kfind_data::{
    DataError, DataWarning, ExpectedMatch, FixturePos, MorphologyCase, parse_morphology_cases_tsv,
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
}

impl GoldHarness {
    pub fn embedded() -> Result<Self, DataError> {
        Self::new(Lexicons::embedded()?)
    }

    pub fn with_full_pos(full_pos: &[u8]) -> Result<Self, DataError> {
        Self::new(Lexicons::embedded_with(Some(full_pos), None)?)
    }

    fn new(lexicons: Lexicons) -> Result<Self, DataError> {
        Ok(Self {
            analyzer: LexiconQueryAnalyzer::new(Arc::new(lexicons)),
        })
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
        let matcher = MorphMatcher::new(Arc::new(plan)).map_err(GoldCaseError::Matcher)?;
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
    use super::*;

    const EXPECTED_CASES: usize = 423;

    #[test]
    fn embedded_morphology_gold_matches_expected() {
        let (cases, warnings) = load_morphology_cases().expect("gold fixture must be valid");
        assert_eq!(cases.len(), EXPECTED_CASES);
        assert!(
            warnings.is_empty(),
            "unexpected fixture warnings: {warnings:#?}"
        );

        let harness = GoldHarness::embedded().expect("embedded lexicons must be valid");
        let mut failures = Vec::new();
        for case in &cases {
            match harness.evaluate(case) {
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
}
