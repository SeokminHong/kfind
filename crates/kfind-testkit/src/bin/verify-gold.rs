use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use kfind_testkit::{GoldHarness, load_morphology_cases};

const WALK_HANG_STRESS: &str = include_str!("../../../../data/fixtures/walk_hang_stress.txt");

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

    println!(
        "morphology gold with auto-POS coverage: {} cases and walk/hang stress passed",
        cases.len()
    );
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
