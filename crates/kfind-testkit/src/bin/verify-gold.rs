use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use kfind_morph::CoarsePos;
use kfind_query::BoundaryPolicy;
use kfind_testkit::{GoldHarness, load_morphology_cases};

const WALK_HANG_STRESS: &str = include_str!("../../../../data/fixtures/walk_hang_stress.txt");
const UD_MORPHOLOGY_CASES: &str = include_str!("../../../../data/fixtures/ud_morphology_cases.tsv");
const ENRICHED_PREDICATES_SOURCE: &str = "data/enriched/predicates.tsv";
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
    let ud_harness =
        harness.with_enriched_predicates(ENRICHED_PREDICATES_SOURCE, ENRICHED_PREDICATES)?;
    verify_ud_morphology(&ud_harness)?;
    verify_context_homographs(&ud_harness)?;

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
        (
            "만",
            CoarsePos::Numeral,
            "그 값은 천이 아니라 만에 가깝습니다.",
            "만에",
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
    Ok(())
}

fn verify_ud_morphology(harness: &GoldHarness) -> Result<(), Box<dyn Error>> {
    let mut failures = Vec::new();
    for (index, line) in UD_MORPHOLOGY_CASES.lines().enumerate().skip(1) {
        let invalid_row = || format!("invalid UD morphology fixture row {}", index + 1);
        let fields = line.split('\t').collect::<Vec<_>>();
        let [id, query, pos, expected, start, end, text] = fields.as_slice() else {
            return Err(invalid_row().into());
        };
        let pos = match *pos {
            "noun" => CoarsePos::Noun,
            "pronoun" => CoarsePos::Pronoun,
            "verb" => CoarsePos::Verb,
            "adjective" => CoarsePos::Adjective,
            "determiner" => CoarsePos::Determiner,
            "adverb" => CoarsePos::Adverb,
            _ => return Err(invalid_row().into()),
        };
        let expectation = *expected;
        let expected = match expectation {
            "match" | "exact" => true,
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
            if boundary == BoundaryPolicy::Any && !expected {
                continue;
            }
            let matches = harness.find_lemma_matches(query, text, pos, boundary)?;
            let spans = if expectation == "exact" {
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
                if expectation == "exact" {
                    *span == (start..end)
                } else {
                    span.start < end && start < span.end
                }
            });
            if found != expected {
                failures.push(format!(
                    "{id} {boundary:?}: target={target:?}, expected={expectation}, spans={spans:?}"
                ));
            }
        }
    }
    if !failures.is_empty() {
        return Err(format!("UD morphology mismatches:\n{}", failures.join("\n")).into());
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
