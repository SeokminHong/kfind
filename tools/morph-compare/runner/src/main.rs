use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use kfind_matcher::MorphMatcher;
use kfind_morph::CoarsePos;
use kfind_query::{
    BoundaryPolicy, CompileOptionOverrides, CompileOptions, LexiconQueryAnalyzer, Lexicons,
    compile_query,
};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FULL_POS_LEXICON: &str = "/opt/morph-benchmark/full-pos/lexicon.bin";
const FULL_POS_LEXICON_ENV: &str = "KFIND_FULL_POS_LEXICON";

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    query: String,
    pos: String,
    text: String,
    expected: bool,
    gold_byte_start: Option<usize>,
    gold_byte_end: Option<usize>,
}

#[derive(Debug, Serialize)]
struct Summary {
    backend: String,
    version: String,
    profile: Option<String>,
    lexicon_artifact_sha256: Option<String>,
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

#[derive(Debug, Serialize)]
struct FailureDiagnostic {
    auto_has_expected_pos_analysis: bool,
    gold_anchor_overlap: bool,
    any_boundary_gold_overlap: bool,
}

#[derive(Clone, Copy)]
enum KfindProfile {
    Embedded,
    FullPos,
}

impl KfindProfile {
    const fn name(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::FullPos => "full-pos",
        }
    }
}

fn main() -> Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 {
        bail!("usage: morph-benchmark-runner BACKEND CASES.jsonl OUTPUT.json");
    }
    let cases = load_cases(Path::new(&arguments[1]))?;
    let summary = match arguments[0].as_str() {
        "kfind" | "kfind-embedded" => run_kfind(&cases, KfindProfile::Embedded)?,
        "kfind-full-pos" => run_kfind(&cases, KfindProfile::FullPos)?,
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

fn run_kfind(cases: &[Case], profile: KfindProfile) -> Result<Summary> {
    let initialization_started = Instant::now();
    let (lexicons, lexicon_artifact_sha256) = match profile {
        KfindProfile::Embedded => (Lexicons::embedded()?, None),
        KfindProfile::FullPos => {
            let configured_path = std::env::var_os(FULL_POS_LEXICON_ENV)
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| FULL_POS_LEXICON.into());
            let artifact = fs::read(&configured_path).with_context(|| {
                format!(
                    "full-pos profile requires lexicon artifact {}",
                    configured_path.display()
                )
            })?;
            let digest = format!("{:x}", Sha256::digest(&artifact));
            (
                Lexicons::embedded_with(Some(&artifact), None)?,
                Some(digest),
            )
        }
    };
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
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
    let evaluation_seconds = evaluation_started.elapsed().as_secs_f64();
    let peak_rss_kib = peak_rss_kib();
    for (case, result) in cases.iter().zip(&mut results) {
        result["failure_diagnostic"] = serde_json::to_value(diagnose_failure(case, &analyzer)?)?;
    }
    Ok(Summary {
        backend: "kfind".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        profile: Some(profile.name().to_owned()),
        lexicon_artifact_sha256,
        initialization_seconds,
        evaluation_seconds,
        peak_rss_kib,
        results,
    })
}

fn diagnose_failure(
    case: &Case,
    analyzer: &LexiconQueryAnalyzer,
) -> Result<Option<FailureDiagnostic>> {
    if !case.expected {
        return Ok(None);
    }
    let expected_pos = parse_pos(&case.pos)?;
    let gold = case
        .gold_byte_start
        .zip(case.gold_byte_end)
        .with_context(|| format!("positive case {} has no gold span", case.id))?;
    let gold_range = gold.0..gold.1;
    let auto_plan = compile_query(&case.query, &CompileOptions::default(), analyzer)
        .with_context(|| format!("failed to compile auto diagnostic for case {}", case.id))?;
    let auto_has_expected_pos_analysis = auto_plan.atoms[0]
        .analyses
        .iter()
        .any(|analysis| analysis.coarse_pos == expected_pos);

    let mut any_options = CompileOptions::resolve(CompileOptionOverrides {
        pos: Some(expected_pos),
        ..CompileOptionOverrides::default()
    })?;
    any_options.boundary = BoundaryPolicy::Any;
    let any_plan = compile_query(&case.query, &any_options, analyzer)
        .with_context(|| format!("failed to compile boundary diagnostic for case {}", case.id))?;
    let gold_anchor_overlap = any_plan.atoms[0].branches.iter().any(|branch| {
        case.text
            .as_bytes()
            .get(gold_range.clone())
            .is_some_and(|gold_text| contains_bytes(gold_text, &branch.anchor))
    });
    let any_matcher = MorphMatcher::new(Arc::new(any_plan))?;
    let any_boundary_gold_overlap = find_all_spans(&any_matcher, &case.text)
        .iter()
        .any(|span| ranges_overlap(span.byte_start..span.byte_end, gold_range.clone()));
    Ok(Some(FailureDiagnostic {
        auto_has_expected_pos_analysis,
        gold_anchor_overlap,
        any_boundary_gold_overlap,
    }))
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn ranges_overlap(left: std::ops::Range<usize>, right: std::ops::Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
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
        profile: None,
        lexicon_artifact_sha256: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn analyzer() -> LexiconQueryAnalyzer {
        LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()))
    }

    fn positive_case(query: &str, pos: &str, text: &str) -> Case {
        Case {
            id: "test".to_owned(),
            query: query.to_owned(),
            pos: pos.to_owned(),
            text: text.to_owned(),
            expected: true,
            gold_byte_start: Some(0),
            gold_byte_end: Some(text.len()),
        }
    }

    #[test]
    fn diagnostic_observes_missing_auto_pos_analysis() {
        let diagnostic =
            diagnose_failure(&positive_case("미등록다", "verb", "미등록다"), &analyzer())
                .unwrap()
                .unwrap();

        assert!(!diagnostic.auto_has_expected_pos_analysis);
        assert!(diagnostic.gold_anchor_overlap);
    }

    #[test]
    fn diagnostic_compares_smart_and_any_boundaries() {
        let diagnostic =
            diagnose_failure(&positive_case("권한", "noun", "사용자권한"), &analyzer())
                .unwrap()
                .unwrap();

        assert!(diagnostic.auto_has_expected_pos_analysis);
        assert!(diagnostic.gold_anchor_overlap);
        assert!(diagnostic.any_boundary_gold_overlap);
    }
}
