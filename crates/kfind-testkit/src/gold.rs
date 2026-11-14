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
    BoundaryPolicy, CompileError, CompileOptionOverrides, CompileOptions, LexiconQueryAnalyzer,
    Lexicons, PhraseMatch, compile_query,
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

    pub fn with_enriched_predicates(
        mut self,
        source: &str,
        input: &str,
    ) -> Result<Self, DataError> {
        let mut lexicons = self.analyzer.lexicons().as_ref().clone();
        lexicons.load_enriched_predicates(source, input)?;
        self.analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
        Ok(self)
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
        let matcher = self.matcher(query, &CompileOptions::default())?;
        Ok(matcher
            .find_all_with_meta(text.as_bytes())
            .into_iter()
            .map(|matched| matched.span)
            .collect())
    }

    pub fn find_lemma_matches(
        &self,
        query: &str,
        text: &str,
        pos: CoarsePos,
        boundary: BoundaryPolicy,
    ) -> Result<Vec<PhraseMatch>, GoldCaseError> {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos: Some(pos),
            boundary: Some(boundary),
            ..CompileOptionOverrides::default()
        })
        .expect("gold fixture uses compatible options");
        Ok(self
            .matcher(query, &options)?
            .find_all_with_meta(text.as_bytes()))
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
        let matcher = self.matcher(&case.query, &options)?;
        let actual_match = matcher.find_at_with_meta(case.text.as_bytes(), 0).is_some();
        Ok(GoldCaseOutcome {
            expected_match: case.expected == ExpectedMatch::Match,
            actual_match,
        })
    }

    fn matcher(
        &self,
        query: &str,
        options: &CompileOptions,
    ) -> Result<MorphMatcher, GoldCaseError> {
        let plan = compile_query(query, options, &self.analyzer).map_err(GoldCaseError::Compile)?;
        if let Some(resource) = &self.component_resource {
            MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(resource))
        } else {
            MorphMatcher::new(Arc::new(plan))
        }
        .map_err(GoldCaseError::Matcher)
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

    const EXPECTED_CASES: usize = 925;

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
                "nominal-component"
                    | "exact-component"
                    | "lexical-context"
                    | "copula-structure"
                    | "fused-copula-ending"
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
    fn generated_endings_match_the_complete_eojeol() {
        let harness = GoldHarness::embedded().expect("embedded lexicons must be valid");
        for (query, text, target) in [
            ("v:가다", "내일 같이 갈까요?", "갈까요"),
            ("adj:바쁘다", "요즘도 많이 바쁜가요?", "바쁜가요"),
            ("v:하다", "어떤 일 하시는지요?", "하시는지요"),
            ("adj:좋다", "날씨가 좋을수록 사람이 많다.", "좋을수록"),
            ("v:먹다", "먼저 먹으라면서 재촉했다.", "먹으라면서"),
            ("v:들다", "먼저 드시라고 했다.", "드시라고"),
            ("adj:좋다", "듣기 좋으라고 칭찬을 보탰다.", "좋으라고"),
            ("v:다루다", "이 주제는 다음 장에서 다뤄진다.", "다뤄진다"),
            ("v:이루다", "합의는 단계적으로 이뤄진다.", "이뤄진다"),
            ("adj:흐리다", "하늘이 흐려지더니 비가 왔다.", "흐려지더니"),
            ("adj:춥다", "날씨가 추워지는 때입니다.", "추워지는"),
            ("adj:춥다", "갑자기 추워져서 외투를 꺼냈다.", "추워져서"),
            ("adj:춥다", "밤이 되어도 추워지지 않았다.", "추워지지"),
            ("adj:춥다", "해가 지면 더 추워집니다.", "추워집니다"),
            ("adj:넓다", "주민들은 길이 넓어지기를 바랐다.", "넓어지기를"),
            ("v:묻다", "면접에서는 무엇을 물어보더냐?", "물어보더냐"),
            (
                "v:묻다",
                "궁금한 것이 있으면 선생님한테 물어보렴.",
                "물어보렴",
            ),
            ("v:먹다", "이것도 같이 먹어볼까요?", "먹어볼까요"),
            ("v:먹다", "이것도 같이 먹어봅시다.", "먹어봅시다"),
            ("v:먹다", "아이도 먹어보기를 바랐다.", "먹어보기를"),
            ("v:먹다", "이것도 한번 먹어 보세요.", "먹어"),
            (
                "adj:넓다",
                "이 기록에서 도로가 넓어짐을 확인했다.",
                "넓어짐을",
            ),
            ("v:하다", "선생님이 직접 설명도 하시더구나.", "하시더구나"),
            (
                "adj:심하다",
                "어제는 통증이 꽤 심했겠더구나.",
                "심했겠더구나",
            ),
            ("v:오다", "먼 데서 친척들도 오셨더구먼.", "오셨더구먼"),
            (
                "adj:아름답다",
                "그날 할머니는 얼마나 아름다우시더뇨?",
                "아름다우시더뇨",
            ),
            (
                "v:지르다",
                "아버지는 한참 소리를 지르시더니마는 이내 조용해지셨다.",
                "지르시더니마는",
            ),
            (
                "adj:좋다",
                "오늘은 몸이 안 좋다더니 먼저 집에 갔구나.",
                "좋다더니",
            ),
            (
                "v:공부하다",
                "요새 열심히 공부한다더니만 시험도 잘 봤구나.",
                "공부한다더니만",
            ),
        ] {
            let start = text.find(target).expect("target appears in the fixture");
            assert_eq!(
                harness
                    .find_all(query, text)
                    .expect("fixture query compiles"),
                vec![start..start + target.len()],
                "{query} in {text:?}",
            );
        }
    }

    #[test]
    fn exclamative_nuna_does_not_consume_the_separate_noun() {
        let harness = GoldHarness::embedded().expect("embedded lexicons must be valid");
        for text in [
            "오니 누나가 웃었다.",
            "오니 누나를 만났다.",
            "오니 누나도 웃었다.",
        ] {
            for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Any] {
                let spans: Vec<_> = harness
                    .find_lemma_matches("오다", text, CoarsePos::Verb, boundary)
                    .expect("fixture query compiles")
                    .into_iter()
                    .map(|matched| matched.span)
                    .collect();
                assert_eq!(spans, [0.."오니".len()], "{text:?}, {boundary:?}");
            }
        }
    }

    #[test]
    fn imperative_ryeom_preserves_regular_and_irregular_stems() {
        let harness = GoldHarness::embedded()
            .expect("embedded lexicons are valid")
            .with_enriched_predicates(
                "data/enriched/predicates.tsv",
                include_str!("../../../data/enriched/predicates.tsv"),
            )
            .expect("enriched predicates are valid");
        for (query, text, target) in [
            ("오다", "얘들아, 이리 오렴.", "오렴"),
            ("살다", "앞으로 건강하게 살려무나.", "살려무나"),
            ("먹다", "이것 좀 먹으려무나.", "먹으려무나"),
            ("듣다", "선생님 말씀을 잘 들으렴.", "들으렴"),
            ("낫다", "유민아, 빨리 나으렴.", "나으렴"),
        ] {
            let start = text.find(target).expect("target appears in the fixture");
            for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Any] {
                let matches = harness
                    .find_lemma_matches(query, text, CoarsePos::Verb, boundary)
                    .expect("fixture query compiles");
                assert!(
                    matches
                        .iter()
                        .any(|matched| matched.span == (start..start + target.len())),
                    "{query} {boundary:?} in {text:?}: {matches:?}",
                );
            }
        }
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
            entry("그런데", "MAJ", 0),
            entry("누굴까", "NP+VCP+EF", 0),
            entry("얼만데", "NNG+VCP+EC", 0),
            entry("날세", "NP+VCP+EF", 0),
            entry("텐데", "NNB+VCP+EC", 0),
            entry("겁니까", "NNB+VCP+EF", 0),
            entry("갈까", "VV+EF", 0),
            entry("게", "NNB+VCP+EC", 0),
            entry("언젠가", "NP+VCP+EC", 0),
            entry("언젠가", "MAG", 0),
            entry("동생", "NNG", 0),
            entry("사기꾼", "NNG", 0),
            entry("수재", "NNG", 0),
            entry("가게", "NNG", 0),
            entry("주소", "NNG", 0),
            entry("새댁", "NNG", 0),
            entry("나오", "VV", 0),
            entry("면", "EC", 0),
            expression_entry("나오면", "VV+EC", "나오/VV/*+면/EC/*", 0),
            entry("친구", "NNG", 0),
            entry("어머니", "NNG", 0),
            entry("가", "VV", 0),
            entry("산책", "NNG", 0),
            entry("이라도", "JX", 0),
            entry("뭐", "NP", 0),
            entry("뭐", "MAG", 0),
            entry("하노", "NNP", 0),
            entry("하", "VV", 0),
            entry("노라", "EF", 0),
            entry("누구", "NP", 0),
            entry("의사", "NNG", 0),
            entry("가", "NNG", 0),
            entry("갑니다", "NNG+VCP+EF", 0),
            entry("갑니다", "VV+EF", 0),
            entry("아", "NNG", 0),
            entry("알", "VV", 0),
            entry("모양", "NNG", 0),
            entry("컴퓨터", "NNG", 0),
            entry("작가", "NNG", 0),
            entry("가지", "NNG", 0),
            entry("가지", "VV", 0),
            entry("넣", "VV", 0),
            entry("을", "ETM", 0),
            entry("잘", "VV+ETM", 0),
            entry("잘", "MAG", 0),
            entry("자", "NNG", 0),
            entry("먹", "VV", 0),
            entry("라고", "EC", 0),
            entry("봄", "NNG", 0),
            entry("이야기", "NNG", 0),
            entry("이래", "VA+EF", 0),
            entry("미", "NNG", 0),
            entry("미래", "NNG", 0),
            entry("학생", "NNG", 0),
            entry("어디", "NP", 0),
            entry("모자", "NNG", 0),
            entry("영숙", "NNP", 0),
            entry("뭐야", "NP+VCP+EC", 0),
            entry("차례", "NNG", 0),
            entry("집", "NNG", 0),
            entry("철수", "NNP", 0),
            entry("하다", "NNG", 0),
            entry("다니", "EC", 0),
            entry("이자", "NNG", 0),
            entry("하시", "NNG", 0),
            entry("이게", "NP+JKS", 0),
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
