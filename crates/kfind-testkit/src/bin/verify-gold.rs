use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use kfind_morph::CoarsePos;
use kfind_query::BoundaryPolicy;
use kfind_testkit::{GoldHarness, load_morphology_cases};

const WALK_HANG_STRESS: &str = include_str!("../../../../data/fixtures/walk_hang_stress.txt");
const UD_MORPHOLOGY_CASES: &str = include_str!("../../../../data/fixtures/ud_morphology_cases.tsv");
const BOUND_SI_CASES: &str = include_str!("../../../../data/fixtures/bound_si_span_cases.tsv");
const INTERROGATIVE_CASES: &str =
    include_str!("../../../../data/fixtures/interrogative_span_cases.tsv");
const PROSPECTIVE_CASES: &str =
    include_str!("../../../../data/fixtures/prospective_span_cases.tsv");
const SEOYA_CASES: &str = include_str!("../../../../data/fixtures/seoya_span_cases.tsv");
const MEURO_CASES: &str = include_str!("../../../../data/fixtures/meuro_span_cases.tsv");
const MSE_CASES: &str = include_str!("../../../../data/fixtures/mse_span_cases.tsv");
const DAO_CASES: &str = include_str!("../../../../data/fixtures/dao_span_cases.tsv");
const JIYO_CASES: &str = include_str!("../../../../data/fixtures/jiyo_span_cases.tsv");
const HUMBLE_CASES: &str = include_str!("../../../../data/fixtures/humble_span_cases.tsv");
const CASUAL_PREFINAL_CASES: &str =
    include_str!("../../../../data/fixtures/casual_prefinal_span_cases.tsv");
const TOPIC_PREDICATE_CASES: &str =
    include_str!("../../../../data/fixtures/topic_predicate_span_cases.tsv");
const ENRICHED_PREDICATES_SOURCE: &str = "data/enriched/predicates.tsv";
const DEPENDENT_SUBJECT_CASES: &str =
    include_str!("../../../../data/fixtures/dependent_subject_cases.tsv");
const ELAPSED_MEASURE_CASES: &str =
    include_str!("../../../../data/fixtures/elapsed_measure_cases.tsv");
const EMPHATIC_PARTICLE_CASES: &str =
    include_str!("../../../../data/fixtures/emphatic_particle_cases.tsv");
const NOMINAL_ONLY_CASES: &str = include_str!("../../../../data/fixtures/nominal_only_cases.tsv");
const DICTIONARY_NUMERIC_UNIT_CASES: &str =
    include_str!("../../../../data/fixtures/dictionary_numeric_unit_cases.tsv");
const PREFIXED_NOMINAL_COPULA_CASES: &str =
    include_str!("../../../../data/fixtures/prefixed_nominal_copula_cases.tsv");
const ADNOMINAL_OBJECT_ATTACHMENT_CASES: &str =
    include_str!("../../../../data/fixtures/adnominal_object_attachment_cases.tsv");
const QUERY_MATRIX_POS_REVIEW_CASES: &str =
    include_str!("../../../../data/fixtures/query_matrix_pos_review_cases.tsv");
const QUANTIFYING_ADVERB_CASES: &str =
    include_str!("../../../../data/fixtures/quantifying_adverb_cases.tsv");
const BIEUP_ADVERBIAL_CASES: &str =
    include_str!("../../../../data/fixtures/bieup_adverbial_cases.tsv");
const ENRICHED_PREDICATES: &str = include_str!("../../../../data/enriched/predicates.tsv");

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let [full_pos_path, component_path] = arguments.as_slice() else {
        return Err(usage_error("full POS and component resource paths are required").into());
    };
    let full_pos = std::fs::read(full_pos_path)?;
    let component_resource = std::fs::read(component_path)?;
    let harness = GoldHarness::with_full_pos_and_component(component_resource, &full_pos)?;
    let (cases, warnings) = load_morphology_cases()?;
    if !warnings.is_empty() {
        return Err(format!("gold fixture warnings: {warnings:#?}").into());
    }

    let mut failures = Vec::new();
    for case in &cases {
        if !harness.auto_includes_expected_pos(case)? {
            failures.push(format!(
                "query={:?} pos={:?} feature={}: auto analysis omitted the expected POS",
                case.query, case.pos, case.feature,
            ));
            continue;
        }
        let outcome = harness.evaluate(case)?;
        if !outcome.matches_expectation() {
            failures.push(format!(
                "query={:?} pos={:?} feature={} text={:?}: expected_match={}, actual_match={}",
                case.query,
                case.pos,
                case.feature,
                case.text,
                outcome.expected_match,
                outcome.actual_match,
            ));
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "{} of {} morphology gold cases failed:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n"),
        )
        .into());
    }

    verify_walk_hang_stress(&harness)?;
    let enriched_harness =
        harness.with_enriched_predicates(ENRICHED_PREDICATES_SOURCE, ENRICHED_PREDICATES)?;
    let mut suite_errors = Vec::new();
    for (fixture, cases) in [
        ("UD morphology", UD_MORPHOLOGY_CASES),
        ("bound si suffix", BOUND_SI_CASES),
        ("dependent noun subject", DEPENDENT_SUBJECT_CASES),
        ("elapsed measure", ELAPSED_MEASURE_CASES),
        ("emphatic particle", EMPHATIC_PARTICLE_CASES),
        ("nominal only", NOMINAL_ONLY_CASES),
        ("dictionary numeric units", DICTIONARY_NUMERIC_UNIT_CASES),
        ("prefixed nominal copula", PREFIXED_NOMINAL_COPULA_CASES),
        (
            "adnominal object attachment",
            ADNOMINAL_OBJECT_ATTACHMENT_CASES,
        ),
        ("query matrix POS review", QUERY_MATRIX_POS_REVIEW_CASES),
        ("quantifying adverbs", QUANTIFYING_ADVERB_CASES),
        ("bieup adverbials", BIEUP_ADVERBIAL_CASES),
        ("interrogative endings", INTERROGATIVE_CASES),
        ("prospective endings", PROSPECTIVE_CASES),
        ("seoya endings", SEOYA_CASES),
        ("meuro endings", MEURO_CASES),
        ("mse endings", MSE_CASES),
        ("dao endings", DAO_CASES),
        ("jiyo endings", JIYO_CASES),
        ("humble endings", HUMBLE_CASES),
        ("casual prefinal endings", CASUAL_PREFINAL_CASES),
        ("topic before predicate", TOPIC_PREDICATE_CASES),
    ] {
        if let Err(error) = verify_span_cases(&enriched_harness, fixture, cases) {
            suite_errors.push(error.to_string());
        }
    }
    for verify in [verify_context_homographs, verify_copula_conditionals] {
        if let Err(error) = verify(&enriched_harness) {
            suite_errors.push(error.to_string());
        }
    }
    if !suite_errors.is_empty() {
        return Err(suite_errors.join("\n\n").into());
    }

    println!(
        "morphology gold with auto-POS coverage: {} cases and walk/hang stress passed",
        cases.len()
    );
    Ok(())
}

fn verify_context_homographs(harness: &GoldHarness) -> Result<(), Box<dyn Error>> {
    for (query, pos, text, surface) in [
        (
            "되다",
            CoarsePos::Verb,
            "신청만으로 자동 승인이 돼지는 건 아니에요.",
            "돼지는",
        ),
        (
            "되다",
            CoarsePos::Verb,
            "첫 번째 기능은 업데이트하면 자동으로 돼지고요, 두 번째는 따로 켜야 해요.",
            "돼지고요",
        ),
        (
            "가다",
            CoarsePos::Verb,
            "물살이 세서 앞으로 잘 가지지 않았다.",
            "가지지",
        ),
        (
            "가다",
            CoarsePos::Verb,
            "밀물이 시작되자 노를 젓지 않아도 배가 앞으로 가지는 게 신기했다.",
            "가지는",
        ),
        ("돼지", CoarsePos::Noun, "돼지는 잡식 동물이다.", "돼지는"),
        (
            "돼지",
            CoarsePos::Noun,
            "이쪽은 돼지고요, 저쪽은 소예요.",
            "돼지",
        ),
        (
            "가지다",
            CoarsePos::Verb,
            "그는 재산을 가지지 않았다.",
            "가지지",
        ),
        ("가지", CoarsePos::Noun, "가지는 보라색 채소다.", "가지는"),
        ("가다", CoarsePos::Verb, "그곳에 가지는 않았다.", "가지는"),
        ("가다", CoarsePos::Verb, "그곳에 가지는 못했다.", "가지는"),
        ("가다", CoarsePos::Verb, "그곳에 가지는 마라.", "가지는"),
        ("뭐", CoarsePos::Pronoun, "지금 뭐 하나?", "뭐"),
        ("뭐", CoarsePos::Pronoun, "지금 뭔가?", "뭔가"),
        ("무엇", CoarsePos::Pronoun, "뭔가를 찾고 있다.", "뭔가를"),
        (
            "어디",
            CoarsePos::Pronoun,
            "어딘가에서 전화가 왔다.",
            "어딘가에서",
        ),
        (
            "불과",
            CoarsePos::Noun,
            "그가 바라는 것은 불과, 곧 부처의 깨달음이다.",
            "불과",
        ),
        (
            "제",
            CoarsePos::Pronoun,
            "그건 제 규정이 아니라 위원회가 정한 규정입니다.",
            "제",
        ),
    ] {
        let start = text.find(surface).expect("target appears in the fixture");
        for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Any] {
            let matches = harness.find_lemma_matches(query, text, pos, boundary)?;
            if !matches
                .iter()
                .any(|matched| matched.span == (start..start + surface.len()))
            {
                return Err(
                    format!("{query} {boundary:?}: expected {surface:?} in {text:?}").into(),
                );
            }
        }
    }
    for (ask, text) in [
        (true, "면접에서는 무엇을 물어보더냐?"),
        (false, "새 장난감을 주니까 강아지가 물어보더냐?"),
    ] {
        let surface = "물어보더냐";
        let start = text.find(surface).expect("target appears in the fixture");
        for (query, contextual_match) in [("묻다", ask), ("물다", !ask)] {
            // Both forms are valid at the same span. Sentence meaning supplies the raw label.
            for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Any] {
                let matches = harness.find_lemma_matches(query, text, CoarsePos::Verb, boundary)?;
                if !matches
                    .iter()
                    .any(|matched| matched.span == (start..start + surface.len()))
                {
                    return Err(format!(
                        "{query} {boundary:?}: expected {surface:?} in {text:?}, contextual_match={contextual_match}"
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

fn verify_copula_conditionals(harness: &GoldHarness) -> Result<(), Box<dyn Error>> {
    // Omitted copulas require the structural analysis used by Smart.
    for (query, text, surface, omitted) in [
        (
            "이다",
            "가까운 가족이라도 지켜야 할 예절이 있다.",
            "이라도",
            false,
        ),
        (
            "이다",
            "설사 그 소문이 진짜라도 우리와는 상관없는 일이야.",
            "진짜라도",
            true,
        ),
        (
            "이다",
            "네 말이 전부 거짓말이라도 나는 너를 아직 믿고 싶어.",
            "이라도",
            false,
        ),
        (
            "이다",
            "내가 아무리 수험생이라도 하루 종일 공부만 할 수는 없다.",
            "이라도",
            false,
        ),
        (
            "이다",
            "내가 그 사람이라도 화가 났을 거야.",
            "이라도",
            false,
        ),
        (
            "이다",
            "너와 함께라면 난 어디든지 갈 수 있어.",
            "함께라면",
            true,
        ),
        ("이다", "내가 너라면 그런 일은 하지 않겠다.", "너라면", true),
        (
            "이다",
            "저에게라면 얼마든지 말씀하셔도 됩니다.",
            "저에게라면",
            true,
        ),
        (
            "새다",
            "길 안내원이 여기서 옆길로 새라면 그대로 가면 돼.",
            "새라면",
            false,
        ),
        (
            "아니다",
            "고의가 아니라도 사과는 해야지.",
            "아니라도",
            false,
        ),
        (
            "아니다",
            "범인이 아니라면 누가 한 짓이지?",
            "아니라면",
            false,
        ),
        (
            "이다",
            "그분이 선생님이시라도 규칙은 같아요.",
            "이시라도",
            false,
        ),
        (
            "이다",
            "그분이 선생님이시라면 설명해 주실 거예요.",
            "이시라면",
            false,
        ),
        (
            "이다",
            "운동이 산책이라도 준비는 필요하다.",
            "이라도",
            false,
        ),
        ("이다", "산책이라도 하자.", "이라도", false),
        ("이다", "이게 영화라도 결말이 너무 뻔해.", "영화라도", true),
        ("이다", "영화라도 한 편 보자.", "영화라도", true),
        (
            "이다",
            "내가 만약 새라면 저 하늘을 훨훨 날아다닐 텐데.",
            "새라면",
            true,
        ),
        (
            "이다",
            "길 안내원이 여기서 옆길로 새라면 그대로 가면 돼.",
            "새라면",
            true,
        ),
        ("이다", "그 글자가 가라면 첫 칸에 써.", "가라면", true),
        ("이다", "아이들이 가라면 먼저 보낼게요.", "가라면", true),
    ] {
        let start = text.find(surface).expect("target appears in the fixture");
        let pos = if query == "새다" {
            CoarsePos::Verb
        } else {
            CoarsePos::Adjective
        };
        for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Any] {
            let matches = harness.find_lemma_matches(query, text, pos, boundary)?;
            let exact = matches
                .iter()
                .any(|matched| matched.span == (start..start + surface.len()));
            if exact != (!omitted || boundary == BoundaryPolicy::Smart) {
                return Err(format!(
                    "{query} {boundary:?}: unexpected range for {surface:?} in {text:?}"
                )
                .into());
            }
        }
    }
    Ok(())
}

fn verify_span_cases(
    harness: &GoldHarness,
    fixture: &str,
    cases: &str,
) -> Result<(), Box<dyn Error>> {
    let mut failures = Vec::new();
    for (index, line) in cases.lines().enumerate().skip(1) {
        let invalid_row = || format!("invalid {fixture} fixture row {}", index + 1);
        let fields = line.split('\t').collect::<Vec<_>>();
        let [
            id,
            query,
            pos,
            expectation,
            start,
            end,
            text,
            contract_review @ ..,
        ] = fields.as_slice()
        else {
            return Err(invalid_row().into());
        };
        let pos = match *pos {
            "noun" => CoarsePos::Noun,
            "pronoun" => CoarsePos::Pronoun,
            "numeral" => CoarsePos::Numeral,
            "verb" => CoarsePos::Verb,
            "adjective" => CoarsePos::Adjective,
            "determiner" => CoarsePos::Determiner,
            "adverb" => CoarsePos::Adverb,
            _ => return Err(invalid_row().into()),
        };
        let contract_positive = match (*expectation, contract_review) {
            (_, []) => false,
            ("no-match", ["structurally-indistinguishable-homograph"]) => true,
            _ => return Err(invalid_row().into()),
        };
        let expectation = if contract_positive {
            "smart-exact"
        } else {
            *expectation
        };
        let expected = match expectation {
            "match" | "exact" | "smart-exact" => true,
            "no-match" => false,
            _ => return Err(invalid_row().into()),
        };
        let start: usize = start.parse()?;
        let end: usize = end.parse()?;
        let target = text.get(start..end).ok_or_else(invalid_row)?;
        if target.is_empty() {
            return Err(invalid_row().into());
        }
        for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Any] {
            if boundary == BoundaryPolicy::Any && (!expected || expectation == "smart-exact") {
                continue;
            }
            let matches = harness.find_lemma_matches(query, text, pos, boundary)?;
            let exact = matches!(expectation, "exact" | "smart-exact");
            let spans = if exact {
                matches
                    .into_iter()
                    .map(|matched| matched.span)
                    .collect::<Vec<_>>()
            } else {
                matches
                    .into_iter()
                    .flat_map(|matched| matched.atoms.into_iter().map(|atom| atom.core))
                    .collect::<Vec<_>>()
            };
            let found = spans.iter().any(|span| {
                if exact {
                    *span == (start..end)
                } else {
                    span.start < end && start < span.end
                }
            });
            if found != expected {
                failures.push(format!(
                    "{id} {boundary:?}: target={target:?}, expected={expectation}, spans={spans:?}"
                ));
            } else if contract_positive {
                println!(
                    "{id} {boundary:?}: raw=no-match, contract=smart-exact, target={target:?}, span={start}..{end}"
                );
            }
        }
    }
    if !failures.is_empty() {
        return Err(format!("{fixture} mismatches:\n{}", failures.join("\n")).into());
    }
    Ok(())
}

fn verify_walk_hang_stress(harness: &GoldHarness) -> Result<(), Box<dyn Error>> {
    let expectations = [
        (
            "v:걷다",
            97,
            &[
                "걸었다며",
                "걸었잖소",
                "걸으시면",
                "걷더니",
                "걸읍시다",
                "걷도록",
            ][..],
        ),
        (
            "v:걸다",
            21,
            &["걸었다며", "걸었잖소", "걸어온", "걸어가십니까", "걸고"][..],
        ),
    ];
    let forbidden = [
        "걸머지고",
        "걸터앉았다",
        "걷잡을",
        "걷히자",
        "걸려",
        "걸신들린",
    ];

    for (query, expected_count, required) in expectations {
        let ranges = harness.find_all(query, WALK_HANG_STRESS)?;
        if ranges.len() != expected_count {
            return Err(format!(
                "walk/hang stress query={query:?}: expected {expected_count} spans, got {}",
                ranges.len()
            )
            .into());
        }
        for required_prefix in required {
            let required_starts = WALK_HANG_STRESS
                .match_indices(required_prefix)
                .map(|(start, _)| start)
                .collect::<Vec<_>>();
            if required_starts.is_empty()
                || !ranges
                    .iter()
                    .any(|range| required_starts.contains(&range.start))
            {
                return Err(format!(
                    "walk/hang stress query={query:?}: required surface {required_prefix:?} was omitted"
                )
                .into());
            }
        }
        for forbidden_prefix in forbidden {
            let forbidden_starts = WALK_HANG_STRESS
                .match_indices(forbidden_prefix)
                .map(|(start, _)| start)
                .collect::<Vec<_>>();
            if ranges
                .iter()
                .any(|range| forbidden_starts.contains(&range.start))
            {
                return Err(format!(
                    "walk/hang stress query={query:?}: derived lemma {forbidden_prefix:?} was matched"
                )
                .into());
            }
        }
    }
    Ok(())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{message}\nusage: verify-gold <lexicon.bin> <morphology-component-compact.kfc>"),
    )
}
