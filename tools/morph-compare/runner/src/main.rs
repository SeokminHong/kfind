use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use kfind_matcher::MorphMatcher;
use kfind_morph::CoarsePos;
use kfind_query::{
    CompileOptionOverrides, CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query,
};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    query: String,
    pos: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    backend: String,
    version: String,
    initialization_seconds: f64,
    evaluation_seconds: f64,
    peak_rss_kib: Option<u64>,
    results: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct Span {
    byte_start: usize,
    byte_end: usize,
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
    if arguments.len() != 3 {
        bail!("usage: morph-benchmark-runner BACKEND CASES.jsonl OUTPUT.json");
    }
    let cases = load_cases(Path::new(&arguments[1]))?;
    let summary = match arguments[0].as_str() {
        "kfind" => run_kfind(&cases)?,
        "lindera" => run_lindera(&cases)?,
        backend => bail!("unknown backend {backend:?}"),
    };
    serde_json::to_writer_pretty(BufWriter::new(File::create(&arguments[2])?), &summary)?;
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

fn run_kfind(cases: &[Case]) -> Result<Summary> {
    let initialization_started = Instant::now();
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded()?));
    let initialization_seconds = initialization_started.elapsed().as_secs_f64();
    let evaluation_started = Instant::now();
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let case_started = Instant::now();
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos: Some(parse_pos(&case.pos)?),
            ..CompileOptionOverrides::default()
        })?;
        let plan = compile_query(&case.query, &options, &analyzer)
            .with_context(|| format!("failed to compile case {}", case.id))?;
        let matcher = MorphMatcher::new(Arc::new(plan))?;
        let spans = find_all_spans(&matcher, &case.text);
        let latency_ms = case_started.elapsed().as_secs_f64() * 1_000.0;
        results.push(json!({"id": case.id, "latency_ms": latency_ms, "spans": spans}));
    }
    Ok(Summary {
        backend: "kfind".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        initialization_seconds,
        evaluation_seconds: evaluation_started.elapsed().as_secs_f64(),
        peak_rss_kib: peak_rss_kib(),
        results,
    })
}

fn find_all_spans(matcher: &MorphMatcher, text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let Some(found) = matcher.find_at_with_meta(text.as_bytes(), at) else {
            break;
        };
        spans.extend(found.atoms.iter().map(|atom| Span {
            byte_start: atom.token.start,
            byte_end: atom.token.end,
        }));
        if found.span.end > at {
            at = found.span.end;
        } else {
            at += 1;
            while at < text.len() && !text.is_char_boundary(at) {
                at += 1;
            }
        }
    }
    spans.sort_by_key(|span| (span.byte_start, span.byte_end));
    spans.dedup_by_key(|span| (span.byte_start, span.byte_end));
    spans
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
            .collect::<Vec<_>>();
        let latency_ms = case_started.elapsed().as_secs_f64() * 1_000.0;
        results.push(json!({"id": case.id, "latency_ms": latency_ms, "tokens": tokens}));
    }
    Ok(Summary {
        backend: "lindera".to_owned(),
        version: "4.0.0".to_owned(),
        initialization_seconds,
        evaluation_seconds: evaluation_started.elapsed().as_secs_f64(),
        peak_rss_kib: peak_rss_kib(),
        results,
    })
}

fn parse_pos(value: &str) -> Result<CoarsePos> {
    Ok(match value {
        "noun" => CoarsePos::Noun,
        "pronoun" => CoarsePos::Pronoun,
        "numeral" => CoarsePos::Numeral,
        "verb" => CoarsePos::Verb,
        "adjective" => CoarsePos::Adjective,
        "determiner" => CoarsePos::Determiner,
        "adverb" => CoarsePos::Adverb,
        other => bail!("unsupported POS {other:?}"),
    })
}

fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}
