use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use kfind_testkit::{CorpusConfig, generate_corpus_tree};

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("generate-corpus: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), String> {
    if arguments.len() != 4 && arguments.len() != 5 {
        return Err(
            "usage: generate-corpus OUTPUT TOTAL_BYTES FILES KOREAN_PERCENT [SEED]".to_owned(),
        );
    }
    let output = PathBuf::from(&arguments[0]);
    let total_bytes = parse(&arguments[1], "TOTAL_BYTES")?;
    let file_count = parse(&arguments[2], "FILES")?;
    let korean_percent = parse(&arguments[3], "KOREAN_PERCENT")?;
    let seed = arguments
        .get(4)
        .map_or(Ok(0x004b_4649_4e44), |value| parse(value, "SEED"))?;
    let stats = generate_corpus_tree(
        &output,
        CorpusConfig {
            total_bytes,
            file_count,
            korean_percent,
            seed,
        },
    )
    .map_err(|error| error.to_string())?;
    println!(
        "generated {} bytes across {} files at {} (seed {})",
        stats.bytes_written,
        stats.files_written,
        stats.root.display(),
        stats.seed
    );
    Ok(())
}

fn parse<T>(value: &std::ffi::OsStr, name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .to_str()
        .ok_or_else(|| format!("{name} must be valid UTF-8"))?
        .parse()
        .map_err(|error| format!("invalid {name}: {error}"))
}
