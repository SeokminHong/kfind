mod shadow;

use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use kfind_data::{DataErrorKind, decode_morphology_resource, parse_sha256};
use kfind_matcher::MorphMatcher;
use kfind_morph::CoarsePos;
use kfind_query::{
    BoundaryPolicy, CompileOptionOverrides, CompileOptions, ContextRequirement,
    LexiconQueryAnalyzer, Lexicons, compile_query,
};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use morph_index_benchmark::component_artifact::decode_compact_component_resource;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use shadow::{
    ShadowBranchEvidence, ShadowResource, ShadowVerificationCounters, diagnose_component_candidate,
    diagnose_lattice_candidate,
};

const FULL_POS_LEXICON: &str = "/opt/morph-benchmark/full-pos/lexicon.bin";
const FULL_POS_LEXICON_ENV: &str = "KFIND_FULL_POS_LEXICON";
const MORPHOLOGY_RESOURCE: &str = "/opt/morph-benchmark/morphology/morphology.bin";
const MORPHOLOGY_RESOURCE_ENV: &str = "KFIND_MORPHOLOGY_RESOURCE";
const COMPONENT_RESOURCE: &str = "/opt/morph-benchmark/component/morphology-component-compact.kfc";
const COMPONENT_RESOURCE_ENV: &str = "KFIND_COMPONENT_RESOURCE";
const MORPHOLOGY_SOURCE_SHA256: &str =
    "fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330";

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
    morphology_artifact_sha256: Option<String>,
    component_artifact_sha256: Option<String>,
    initialization_seconds: f64,
    evaluation_seconds: f64,
    peak_rss_kib: Option<u64>,
    results: Vec<Value>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
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
    let morphology_path = std::env::var_os(MORPHOLOGY_RESOURCE_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| MORPHOLOGY_RESOURCE.into());
    let morphology_bytes = match fs::read(&morphology_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read morphology resource {}",
                    morphology_path.display()
                )
            });
        }
    };
    let morphology_artifact_sha256 = morphology_bytes
        .as_ref()
        .map(|artifact| format!("{:x}", Sha256::digest(artifact)));
    let morphology_source_digest = parse_sha256(MORPHOLOGY_SOURCE_SHA256)?;
    let morphology = morphology_bytes.as_deref().map(|artifact| {
        decode_morphology_resource(
            &morphology_path.display().to_string(),
            artifact,
            &morphology_source_digest,
        )
    });
    let shadow_resource = match &morphology {
        Some(Ok(resource)) => ShadowResource::Loaded(resource),
        Some(Err(error))
            if matches!(
                error.kind.as_ref(),
                DataErrorKind::MorphologyResourceSourceMismatch
            ) =>
        {
            ShadowResource::SourceMismatch
        }
        Some(Err(_)) => ShadowResource::Corrupt,
        None => ShadowResource::Missing,
    };
    let component_path = std::env::var_os(COMPONENT_RESOURCE_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| COMPONENT_RESOURCE.into());
    let component_bytes = match fs::read(&component_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read component resource {}",
                    component_path.display()
                )
            });
        }
    };
    let component_artifact_sha256 = component_bytes
        .as_ref()
        .map(|artifact| format!("{:x}", Sha256::digest(artifact)));
    let component = component_bytes
        .as_deref()
        .map(|artifact| decode_compact_component_resource(artifact, &morphology_source_digest));
    let component_shadow_resource = match &component {
        Some(Ok(resource)) => ShadowResource::Loaded(resource),
        Some(Err(_)) => ShadowResource::Corrupt,
        None => ShadowResource::Missing,
    };
    for (case, result) in cases.iter().zip(&mut results) {
        result["failure_diagnostic"] = serde_json::to_value(diagnose_failure(case, &analyzer)?)?;
        result["shadow_verification"] = serde_json::to_value(diagnose_verification(
            case,
            &analyzer,
            shadow_resource,
            component_shadow_resource,
        )?)?;
    }
    Ok(Summary {
        backend: "kfind".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        profile: Some(profile.name().to_owned()),
        lexicon_artifact_sha256,
        morphology_artifact_sha256,
        component_artifact_sha256,
        initialization_seconds,
        evaluation_seconds,
        peak_rss_kib,
        results,
    })
}

fn diagnose_verification(
    case: &Case,
    analyzer: &LexiconQueryAnalyzer,
    resource: ShadowResource<'_>,
    component_resource: ShadowResource<'_>,
) -> Result<ShadowVerificationCounters> {
    let options = CompileOptions::resolve(CompileOptionOverrides {
        pos: Some(parse_pos(&case.pos)?),
        ..CompileOptionOverrides::default()
    })?;
    let plan = compile_query(&case.query, &options, analyzer)
        .with_context(|| format!("failed to compile shadow diagnostic for case {}", case.id))?;
    let local_branches = plan
        .atoms
        .iter()
        .enumerate()
        .flat_map(|(atom_index, atom)| {
            atom.branches
                .iter()
                .filter(|branch| branch.context_requirement == ContextRequirement::EojeolLattice)
                .map(move |branch| ShadowBranchEvidence {
                    atom_index,
                    anchor: std::str::from_utf8(&branch.anchor)
                        .expect("compiled query anchors are valid UTF-8")
                        .to_owned(),
                    require_left: branch.boundary.require_left,
                    require_right: branch.boundary.require_right,
                })
        })
        .collect();
    let component_branches = plan
        .atoms
        .iter()
        .enumerate()
        .flat_map(|(atom_index, atom)| {
            atom.branches
                .iter()
                .filter(|branch| branch.context_requirement == ContextRequirement::NominalComponent)
                .map(move |branch| ShadowBranchEvidence {
                    atom_index,
                    anchor: std::str::from_utf8(&branch.anchor)
                        .expect("compiled query anchors are valid UTF-8")
                        .to_owned(),
                    require_left: branch.boundary.require_left,
                    require_right: branch.boundary.require_right,
                })
        })
        .collect();
    let matcher = MorphMatcher::new(Arc::new(plan))?;
    let candidates = matcher.local_analysis_candidates(case.text.as_bytes());
    let component_candidates = candidates
        .iter()
        .filter(|candidate| candidate.context_requirement == ContextRequirement::NominalComponent)
        .collect::<Vec<_>>();
    let required_resource_error = (!component_candidates.is_empty())
        .then(|| resource.unavailable_status())
        .flatten();
    if let Some(status) = required_resource_error {
        bail!(
            "nominal component shadow for case {} requires a valid morphology resource: {status}",
            case.id
        );
    }
    let required_component_error = (!component_candidates.is_empty())
        .then(|| component_resource.unavailable_status())
        .flatten();
    if let Some(status) = required_component_error {
        bail!(
            "nominal component shadow for case {} requires a valid compact resource: {status}",
            case.id
        );
    }
    let lattice = candidates
        .iter()
        .filter(|candidate| candidate.context_requirement == ContextRequirement::EojeolLattice)
        .map(|candidate| diagnose_lattice_candidate(candidate, resource))
        .collect();
    let mut component = Vec::with_capacity(component_candidates.len());
    for candidate in component_candidates {
        let full_evidence = diagnose_component_candidate(candidate, resource);
        let compact_evidence = diagnose_component_candidate(candidate, component_resource);
        if full_evidence != compact_evidence {
            bail!(
                "component projection differs for case {} atom {} analysis {}",
                case.id,
                candidate.atom_index,
                candidate.analysis_index
            );
        }
        component.push(full_evidence);
    }
    let component_projection_comparisons = component.len();
    Ok(ShadowVerificationCounters::new(
        matcher.verification_counters(case.text.as_bytes()),
        local_branches,
        component_branches,
        lattice,
        component,
        component_projection_comparisons,
    ))
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
        morphology_artifact_sha256: None,
        component_artifact_sha256: None,
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
    use std::io::Cursor;

    use kfind_data::{
        DataFinePos, MecabSourceMorphologyEntry, decode_morphology_resource,
        encode_morphology_resource, parse_mecab_connection_matrix,
    };

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

    #[test]
    fn shadow_diagnostic_counts_vcp_analysis_windows() {
        let counters = diagnose_verification(
            &positive_case("이다", "adjective", "매일 운동한다."),
            &analyzer(),
            ShadowResource::Missing,
            ShadowResource::Missing,
        )
        .unwrap();

        assert_eq!(counters.raw_anchor_hits, 1);
        assert_eq!(counters.verified_branch_hits, 1);
        assert_eq!(counters.local_lattice_candidate_hits, 1);
        assert_eq!(counters.unique_analysis_windows, 1);
        assert_eq!(counters.nominal_component_candidate_hits, 0);
        assert_eq!(counters.unique_component_windows, 0);
        assert!(counters.local_branches.iter().any(|branch| {
            branch.anchor == "일" && !branch.require_left && branch.require_right
        }));
        assert!(
            counters
                .lattice
                .iter()
                .all(|evidence| evidence.status == "resource-missing")
        );
    }

    #[test]
    fn nominal_component_shadow_requires_a_valid_resource() {
        let error = diagnose_verification(
            &positive_case("권한", "noun", "사용자권한"),
            &analyzer(),
            ShadowResource::Missing,
            ShadowResource::Missing,
        )
        .unwrap_err();

        assert!(error.to_string().contains("resource-missing"));
    }

    #[test]
    fn nominal_component_shadow_compares_projection_evidence() {
        let bytes = component_fixture_resource(20);
        let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
        let counters = diagnose_verification(
            &positive_case("권한", "noun", "사용자권한"),
            &analyzer(),
            ShadowResource::Loaded(&resource),
            ShadowResource::Loaded(&resource),
        )
        .unwrap();

        assert_eq!(
            counters.component_projection_comparisons,
            counters.nominal_component_candidate_hits
        );
        assert_eq!(counters.component_projection_mismatches, 0);
    }

    #[test]
    fn nominal_component_shadow_rejects_projection_differences() {
        let full_bytes = component_fixture_resource(20);
        let compact_bytes = component_fixture_resource(2_000);
        let full = decode_morphology_resource("full", &full_bytes, &[9; 32]).unwrap();
        let compact = decode_morphology_resource("compact", &compact_bytes, &[9; 32]).unwrap();
        let error = diagnose_verification(
            &positive_case("권한", "noun", "사용자권한"),
            &analyzer(),
            ShadowResource::Loaded(&full),
            ShadowResource::Loaded(&compact),
        )
        .unwrap_err();

        assert!(error.to_string().contains("component projection differs"));
    }

    fn component_fixture_resource(component_cost: i32) -> Vec<u8> {
        let entries = [
            source_entry("사용자", -5_000),
            source_entry("권한", component_cost),
            source_entry("사용자권한", 5_000),
        ];
        let matrix = parse_mecab_connection_matrix(
            "matrix.def",
            Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
        )
        .unwrap();
        encode_morphology_resource(
            [9; 32],
            &entries,
            &matrix,
            b"DEFAULT 0 1 0\nHANGUL 0 1 2\n0xAC00..0xD7A3 HANGUL\n",
            b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
        )
        .unwrap()
    }

    fn source_entry(surface: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
        MecabSourceMorphologyEntry {
            surface: surface.to_owned(),
            pos: DataFinePos::Nng.as_str().to_owned(),
            left_id: 1,
            right_id: 1,
            word_cost,
            analysis_type: "*".to_owned(),
            start_pos: "*".to_owned(),
            end_pos: "*".to_owned(),
            expression: "*".to_owned(),
        }
    }
}
