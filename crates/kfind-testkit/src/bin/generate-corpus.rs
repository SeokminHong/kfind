use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use kfind_testkit::{CorpusConfig, generate_corpus_tree};

const DEFAULT_SEED: u64 = 0x004b_4649_4e44;
const USAGE: &str = "usage: generate-corpus OUTPUT \
    --total-bytes BYTES --files COUNT --korean-percent PERCENT \
    [--nfd-percent PERCENT] [--small-files COUNT] \
    [--small-file-bytes BYTES] [--seed INTEGER]";

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("generate-corpus: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<(), String> {
    let parsed = Arguments::parse(arguments)?;
    let stats = generate_corpus_tree(
        &parsed.output,
        CorpusConfig {
            total_bytes: parsed.required_u64("--total-bytes", parsed.total_bytes)?,
            file_count: parsed.required_usize("--files", parsed.file_count)?,
            small_file_count: parsed.small_file_count,
            small_file_bytes: parsed.small_file_bytes,
            korean_percent: parsed.required_u8("--korean-percent", parsed.korean_percent)?,
            nfd_percent: parsed.nfd_percent,
            seed: parsed.seed,
        },
    )
    .map_err(|error| error.to_string())?;
    println!(
        "generated {} bytes across {} files ({} small, {} large) at {}",
        stats.bytes_written,
        stats.files_written,
        stats.small_files_written,
        stats.large_files_written,
        stats.root.display()
    );
    println!(
        "lines: {} Korean ({} NFC, {} NFD), {} ASCII; seed {}",
        stats.korean_lines,
        stats.nfc_korean_lines,
        stats.nfd_korean_lines,
        stats.ascii_lines,
        stats.seed
    );
    Ok(())
}

#[derive(Debug)]
struct Arguments {
    output: PathBuf,
    total_bytes: Option<u64>,
    file_count: Option<usize>,
    small_file_count: usize,
    small_file_bytes: u64,
    korean_percent: Option<u8>,
    nfd_percent: u8,
    seed: u64,
}

impl Arguments {
    fn parse(arguments: Vec<OsString>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let output = arguments
            .next()
            .filter(|value| value != "--help" && value != "-h")
            .map(PathBuf::from)
            .ok_or_else(|| USAGE.to_owned())?;
        let mut parsed = Self {
            output,
            total_bytes: None,
            file_count: None,
            small_file_count: 0,
            small_file_bytes: 0,
            korean_percent: None,
            nfd_percent: 0,
            seed: DEFAULT_SEED,
        };

        while let Some(flag) = arguments.next() {
            let value = arguments
                .next()
                .ok_or_else(|| format!("{} requires a value\n{USAGE}", flag.to_string_lossy()))?;
            match flag.to_str() {
                Some("--total-bytes") => parsed.total_bytes = Some(parse(&value, "BYTES")?),
                Some("--files") => parsed.file_count = Some(parse(&value, "COUNT")?),
                Some("--small-files") => parsed.small_file_count = parse(&value, "COUNT")?,
                Some("--small-file-bytes") => parsed.small_file_bytes = parse(&value, "BYTES")?,
                Some("--korean-percent") => parsed.korean_percent = Some(parse(&value, "PERCENT")?),
                Some("--nfd-percent") => parsed.nfd_percent = parse(&value, "PERCENT")?,
                Some("--seed") => parsed.seed = parse(&value, "INTEGER")?,
                _ => {
                    return Err(format!(
                        "unknown option {}\n{USAGE}",
                        flag.to_string_lossy()
                    ));
                }
            }
        }
        Ok(parsed)
    }

    fn required_u64(&self, name: &str, value: Option<u64>) -> Result<u64, String> {
        value.ok_or_else(|| format!("missing {name}\n{USAGE}"))
    }

    fn required_usize(&self, name: &str, value: Option<usize>) -> Result<usize, String> {
        value.ok_or_else(|| format!("missing {name}\n{USAGE}"))
    }

    fn required_u8(&self, name: &str, value: Option<u8>) -> Result<u8, String> {
        value.ok_or_else(|| format!("missing {name}\n{USAGE}"))
    }
}

fn parse<T>(value: &OsStr, name: &str) -> Result<T, String>
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
