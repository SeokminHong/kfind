use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use kfind_testkit::{GoldHarness, load_morphology_cases};

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

    println!(
        "morphology gold with auto-POS coverage: {} cases passed",
        cases.len()
    );
    Ok(())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{message}\nusage: verify-gold <lexicon.bin> <morphology-component-compact.kfc>"),
    )
}
