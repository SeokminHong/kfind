use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use kfind_data::ExpectedMatch;
use kfind_testkit::{GoldHarness, load_morphology_cases};

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| usage_error("full POS lexicon path is required"))?;
    if env::args_os().nth(2).is_some() {
        return Err(usage_error("unexpected extra argument").into());
    }

    let full_pos = std::fs::read(&path)?;
    let harness = GoldHarness::with_full_pos(&full_pos)?;
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
        let outcome = if case.expected == ExpectedMatch::Match {
            harness.evaluate_auto(case)?
        } else {
            harness.evaluate(case)?
        };
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
            "{} of {} auto-POS gold cases failed:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n"),
        )
        .into());
    }

    println!("auto-POS gold: {} cases passed", cases.len());
    Ok(())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{message}\nusage: verify-gold <lexicon.bin>"),
    )
}
