use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use serde::{Deserialize, Serialize};

const LINDERA_VERSION: &str = "4.0.0";

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    backend: &'static str,
    version: &'static str,
    initialization_seconds: f64,
    evaluation_seconds: f64,
    peak_rss_kib: Option<u64>,
    results: Vec<CaseResult>,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    id: String,
    latency_ms: f64,
    tokens: Vec<RawToken>,
}

#[derive(Debug, Serialize)]
struct RawToken {
    surface: String,
    byte_start: usize,
    byte_end: usize,
    details: Vec<String>,
}

fn main() -> Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        bail!("usage: lindera-benchmark-runner CASES.jsonl OUTPUT.json");
    }
    let cases = load_cases(Path::new(&arguments[0]))?;
    let summary = run_lindera(&cases)?;
    serde_json::to_writer_pretty(BufWriter::new(File::create(&arguments[1])?), &summary)?;
    Ok(())
}

fn load_cases(path: &Path) -> Result<Vec<Case>> {
    BufReader::new(File::open(path).with_context(|| format!("failed to open {}", path.display()))?)
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(&line?).with_context(|| {
                format!(
                    "invalid case at {}:{}",
                    path.display(),
                    index.saturating_add(1)
                )
            })
        })
        .collect()
}

fn run_lindera(cases: &[Case]) -> Result<Summary> {
    let initialization_started = Instant::now();
    let dictionary = load_dictionary("embedded://ko-dic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);
    let initialization_seconds = initialization_started.elapsed().as_secs_f64();
    let evaluation_started = Instant::now();
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let case_started = Instant::now();
        let mut analyzed = tokenizer
            .tokenize(&case.text)
            .with_context(|| format!("failed to tokenize case {}", case.id))?;
        let tokens = analyzed
            .iter_mut()
            .map(|token| RawToken {
                surface: token.surface.to_string(),
                byte_start: token.byte_start,
                byte_end: token.byte_end,
                details: token.details().into_iter().map(str::to_owned).collect(),
            })
            .collect();
        results.push(CaseResult {
            id: case.id.clone(),
            latency_ms: case_started.elapsed().as_secs_f64() * 1_000.0,
            tokens,
        });
    }
    Ok(Summary {
        backend: "lindera",
        version: LINDERA_VERSION,
        initialization_seconds,
        evaluation_seconds: evaluation_started.elapsed().as_secs_f64(),
        peak_rss_kib: peak_rss_kib(),
        results,
    })
}

fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}
