mod agent_shadow;
mod shadow;

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use kfind::Engine;
use kfind::expert::EngineExt as _;
use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, ComponentResource, DataErrorKind, decode_component_resource,
    decode_morphology_resource, parse_sha256,
};
use kfind_matcher::MorphMatcher;
use kfind_morph::CoarsePos;
use kfind_query::{
    BoundaryPolicy, CompileOptionOverrides, CompileOptions, ContextRequirement,
    LexiconQueryAnalyzer, Lexicons, compile_query,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use agent_shadow::diagnose_agent_shadow;
use shadow::{
    ShadowBranchEvidence, ShadowResource, ShadowVerificationCounters, diagnose_component_candidate,
};

const FULL_POS_LEXICON: &str = "/opt/morph-benchmark/full-pos/lexicon.bin";
const FULL_POS_LEXICON_ENV: &str = "KFIND_FULL_POS_LEXICON";
const ENRICHED_PREDICATES: &str = "/opt/morph-benchmark/enriched/predicates.tsv";
const ENRICHED_PREDICATES_ENV: &str = "KFIND_ENRICHED_PREDICATES";
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
    boundary: Option<String>,
    lexicon_artifact_sha256: Option<String>,
    enriched_artifact_sha256: Option<String>,
    morphology_artifact_sha256: Option<String>,
    component_artifact_sha256: Option<String>,
    initialization_seconds: f64,
    evaluation_seconds: f64,
    peak_rss_kib: Option<u64>,
    results: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct StartupSummary {
    backend: String,
    version: String,
    profile: String,
    base_initialization_seconds: f64,
    component_initialization_seconds: Option<f64>,
    initialization_seconds: f64,
    base_peak_rss_kib: Option<u64>,
    peak_rss_kib: Option<u64>,
    full_pos_loaded: bool,
    enriched_predicates_loaded: bool,
    component_resource_loaded: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct Span {
    byte_start: usize,
    byte_end: usize,
}

#[derive(Debug, Serialize)]
struct FailureDiagnostic {
    auto_has_expected_pos_analysis: bool,
    gold_anchor_overlap: bool,
    any_boundary_gold_overlap: bool,
    any_boundary_gold_matches: Vec<DiagnosticMatch>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct DiagnosticMatch {
    core: DiagnosticSpan,
    token: DiagnosticSpan,
    origins: Vec<DiagnosticOrigin>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct DiagnosticSpan {
    byte_start: usize,
    byte_end: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct DiagnosticOrigin {
    analysis_index: u16,
    rule_path: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct UntaggedPlanDiagnostic {
    expected_pos_present: bool,
    coarse_pos: Vec<&'static str>,
    multi_coarse_pos: bool,
    literal_fallback: bool,
}

#[derive(Clone, Copy)]
enum KfindProfile {
    Embedded,
    FullPos,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KfindBoundary {
    Smart,
    Token,
    Any,
}

#[derive(Clone, Copy)]
enum KfindQueryMode {
    ExplicitPos,
    Untagged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupProfile {
    Embedded,
    EmbeddedComponent,
    FullPos,
    FullPosComponent,
}

impl StartupProfile {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "embedded" => Ok(Self::Embedded),
            "embedded-component" => Ok(Self::EmbeddedComponent),
            "full-pos" => Ok(Self::FullPos),
            "full-pos-component" => Ok(Self::FullPosComponent),
            _ => bail!("unknown startup profile {value:?}"),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::EmbeddedComponent => "embedded-component",
            Self::FullPos => "full-pos",
            Self::FullPosComponent => "full-pos-component",
        }
    }

    const fn full_pos(self) -> bool {
        matches!(self, Self::FullPos | Self::FullPosComponent)
    }

    const fn component(self) -> bool {
        matches!(self, Self::EmbeddedComponent | Self::FullPosComponent)
    }
}

impl KfindProfile {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "embedded" | "kfind-embedded" => Ok(Self::Embedded),
            "full-pos" | "kfind-full-pos" => Ok(Self::FullPos),
            _ => bail!("unknown kfind profile {value:?}"),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::FullPos => "full-pos",
        }
    }
}

impl KfindBoundary {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "smart" => Ok(Self::Smart),
            "token" => Ok(Self::Token),
            "any" => Ok(Self::Any),
            _ => bail!("unknown boundary policy {value:?}"),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Smart => "smart",
            Self::Token => "token",
            Self::Any => "any",
        }
    }

    const fn policy(self) -> BoundaryPolicy {
        match self {
            Self::Smart => BoundaryPolicy::Smart,
            Self::Token => BoundaryPolicy::Token,
            Self::Any => BoundaryPolicy::Any,
        }
    }

    const fn requires_component(self) -> bool {
        matches!(self, Self::Smart)
    }
}

fn main() -> Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == "agent-shadow")
    {
        if arguments.len() != 3 {
            bail!("usage: morph-benchmark-runner agent-shadow CASES.jsonl OUTPUT.json");
        }
        let cases = load_cases(Path::new(&arguments[1]))?;
        let lexicons = Lexicons::embedded()?;
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
        let morphology_path = std::env::var_os(MORPHOLOGY_RESOURCE_ENV)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| MORPHOLOGY_RESOURCE.into());
        let morphology_bytes = fs::read(&morphology_path).with_context(|| {
            format!(
                "Agent shadow requires morphology resource {}",
                morphology_path.display()
            )
        })?;
        let morphology_artifact_sha256 = format!("{:x}", Sha256::digest(&morphology_bytes));
        let source_digest = parse_sha256(MORPHOLOGY_SOURCE_SHA256)?;
        let morphology = decode_morphology_resource(
            &morphology_path.display().to_string(),
            &morphology_bytes,
            &source_digest,
        )?;
        let summary =
            diagnose_agent_shadow(&cases, &analyzer, &morphology, morphology_artifact_sha256)?;
        serde_json::to_writer_pretty(BufWriter::new(File::create(&arguments[2])?), &summary)?;
        return Ok(());
    }
    if arguments
        .first()
        .is_some_and(|argument| matches!(argument.as_str(), "boundary" | "untagged"))
    {
        if arguments.len() != 5 {
            bail!(
                "usage: morph-benchmark-runner {{boundary|untagged}} PROFILE BOUNDARY CASES.jsonl OUTPUT.json"
            );
        }
        let cases = load_cases(Path::new(&arguments[3]))?;
        let summary = run_kfind(
            &cases,
            KfindProfile::parse(&arguments[1])?,
            KfindBoundary::parse(&arguments[2])?,
            false,
            if arguments[0] == "untagged" {
                KfindQueryMode::Untagged
            } else {
                KfindQueryMode::ExplicitPos
            },
        )?;
        serde_json::to_writer_pretty(BufWriter::new(File::create(&arguments[4])?), &summary)?;
        return Ok(());
    }
    if arguments.len() != 3 {
        bail!(
            "usage: morph-benchmark-runner BACKEND CASES.jsonl OUTPUT.json\n\
             or: morph-benchmark-runner startup PROFILE OUTPUT.json\n\
             or: morph-benchmark-runner agent-shadow CASES.jsonl OUTPUT.json\n\
             or: morph-benchmark-runner {{boundary|untagged}} PROFILE BOUNDARY CASES.jsonl OUTPUT.json"
        );
    }
    if arguments[0] == "startup" {
        let summary = run_kfind_startup(StartupProfile::parse(&arguments[1])?)?;
        serde_json::to_writer_pretty(BufWriter::new(File::create(&arguments[2])?), &summary)?;
        return Ok(());
    }
    let cases = load_cases(Path::new(&arguments[1]))?;
    let summary = match arguments[0].as_str() {
        "kfind" | "kfind-embedded" => run_kfind(
            &cases,
            KfindProfile::Embedded,
            KfindBoundary::Smart,
            true,
            KfindQueryMode::ExplicitPos,
        )?,
        "kfind-full-pos" => run_kfind(
            &cases,
            KfindProfile::FullPos,
            KfindBoundary::Smart,
            true,
            KfindQueryMode::ExplicitPos,
        )?,
        backend => bail!("unknown backend {backend:?}"),
    };
    serde_json::to_writer_pretty(BufWriter::new(File::create(&arguments[2])?), &summary)?;
    Ok(())
}

fn run_kfind_startup(profile: StartupProfile) -> Result<StartupSummary> {
    let base_started = Instant::now();
    let (mut engine, enriched_predicates_loaded) = if profile.full_pos() {
        let (lexicons, _, _) = load_full_profile_lexicons()?;
        (Engine::from_lexicons(lexicons), true)
    } else {
        (Engine::new()?, false)
    };
    let base_initialization_seconds = base_started.elapsed().as_secs_f64();
    let base_peak_rss_kib = peak_rss_kib();

    let component_initialization_seconds = if profile.component() {
        let component_started = Instant::now();
        let component_path = std::env::var_os(COMPONENT_RESOURCE_ENV)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| COMPONENT_RESOURCE.into());
        let component_bytes = fs::read(&component_path).with_context(|| {
            format!(
                "component startup profile requires resource {}",
                component_path.display()
            )
        })?;
        engine.load_component_resource(component_bytes)?;
        Some(component_started.elapsed().as_secs_f64())
    } else {
        None
    };
    let initialization_seconds =
        base_initialization_seconds + component_initialization_seconds.unwrap_or_default();

    Ok(StartupSummary {
        backend: "kfind".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        profile: profile.name().to_owned(),
        base_initialization_seconds,
        component_initialization_seconds,
        initialization_seconds,
        base_peak_rss_kib,
        peak_rss_kib: peak_rss_kib(),
        full_pos_loaded: engine.full_pos_loaded(),
        enriched_predicates_loaded,
        component_resource_loaded: engine.component_resource_loaded(),
    })
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

fn run_kfind(
    cases: &[Case],
    profile: KfindProfile,
    boundary: KfindBoundary,
    include_diagnostics: bool,
    query_mode: KfindQueryMode,
) -> Result<Summary> {
    let initialization_started = Instant::now();
    let (component_resource, component_artifact_sha256) = if boundary.requires_component() {
        let component_path = std::env::var_os(COMPONENT_RESOURCE_ENV)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| COMPONENT_RESOURCE.into());
        let component_bytes = fs::read(&component_path).with_context(|| {
            format!(
                "smart boundary requires component resource {}",
                component_path.display()
            )
        })?;
        let digest = format!("{:x}", Sha256::digest(&component_bytes));
        let resource = decode_component_resource(
            &component_path.display().to_string(),
            component_bytes,
            &COMPONENT_RESOURCE_SOURCE_DIGEST,
        )?;
        (Some(Arc::new(resource)), Some(digest))
    } else {
        (None, None)
    };
    let (lexicons, lexicon_artifact_sha256, enriched_artifact_sha256) = match profile {
        KfindProfile::Embedded => (Lexicons::embedded()?, None, None),
        KfindProfile::FullPos => {
            let (lexicons, full_pos_artifact, enriched_artifact) = load_full_profile_lexicons()?;
            (
                lexicons,
                Some(format!("{:x}", Sha256::digest(&full_pos_artifact))),
                Some(format!("{:x}", Sha256::digest(&enriched_artifact))),
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
            boundary: Some(boundary.policy()),
            pos: match query_mode {
                KfindQueryMode::ExplicitPos => Some(parse_pos(&case.pos)?),
                KfindQueryMode::Untagged => None,
            },
            ..CompileOptionOverrides::default()
        })?;
        let plan = compile_query(&case.query, &options, &analyzer)
            .with_context(|| format!("failed to compile case {}", case.id))?;
        let matcher = match &component_resource {
            Some(resource) => {
                MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(resource))?
            }
            None => MorphMatcher::new(Arc::new(plan))?,
        };
        let spans = find_all_spans(&matcher, &case.text);
        let latency_ms = case_started.elapsed().as_secs_f64() * 1_000.0;
        results.push(json!({
            "id": case.id,
            "latency_ms": latency_ms,
            "spans": spans,
            "failure_diagnostic": null,
            "plan_diagnostic": null,
            "shadow_verification": {},
        }));
    }
    let evaluation_seconds = evaluation_started.elapsed().as_secs_f64();
    let peak_rss_kib = peak_rss_kib();
    if matches!(query_mode, KfindQueryMode::Untagged) {
        append_untagged_plan_diagnostics(cases, &mut results, &analyzer)?;
    }
    let morphology_artifact_sha256 = if include_diagnostics {
        append_kfind_diagnostics(cases, &mut results, &analyzer, &component_resource)?
    } else {
        None
    };
    Ok(Summary {
        backend: "kfind".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        profile: Some(profile.name().to_owned()),
        boundary: Some(boundary.name().to_owned()),
        lexicon_artifact_sha256,
        enriched_artifact_sha256,
        morphology_artifact_sha256,
        component_artifact_sha256,
        initialization_seconds,
        evaluation_seconds,
        peak_rss_kib,
        results,
    })
}

fn load_full_profile_lexicons() -> Result<(Lexicons, Vec<u8>, Vec<u8>)> {
    let full_pos_path = std::env::var_os(FULL_POS_LEXICON_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| FULL_POS_LEXICON.into());
    let full_pos_artifact = fs::read(&full_pos_path).with_context(|| {
        format!(
            "full-pos profile requires lexicon artifact {}",
            full_pos_path.display()
        )
    })?;
    let enriched_path = std::env::var_os(ENRICHED_PREDICATES_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ENRICHED_PREDICATES.into());
    let enriched_artifact = fs::read(&enriched_path).with_context(|| {
        format!(
            "full-pos profile requires enriched predicates {}",
            enriched_path.display()
        )
    })?;
    let enriched_source = std::str::from_utf8(&enriched_artifact).with_context(|| {
        format!(
            "enriched predicate artifact is not UTF-8: {}",
            enriched_path.display()
        )
    })?;
    let mut lexicons = Lexicons::embedded_with(Some(&full_pos_artifact), None)?;
    lexicons.load_enriched_predicates(&enriched_path.to_string_lossy(), enriched_source)?;
    Ok((lexicons, full_pos_artifact, enriched_artifact))
}

fn append_untagged_plan_diagnostics(
    cases: &[Case],
    results: &mut [Value],
    analyzer: &LexiconQueryAnalyzer,
) -> Result<()> {
    for (case, result) in cases.iter().zip(results.iter_mut()) {
        result["plan_diagnostic"] = serde_json::to_value(diagnose_untagged_plan(case, analyzer)?)?;
    }
    Ok(())
}

fn diagnose_untagged_plan(
    case: &Case,
    analyzer: &LexiconQueryAnalyzer,
) -> Result<UntaggedPlanDiagnostic> {
    let plan = compile_query(&case.query, &CompileOptions::default(), analyzer)
        .with_context(|| format!("failed to compile untagged diagnostic for case {}", case.id))?;
    let coarse_pos = plan.atoms[0]
        .analyses
        .iter()
        .map(|analysis| analysis.coarse_pos)
        .collect::<BTreeSet<_>>();
    let expected_pos_present = coarse_pos.contains(&parse_pos(&case.pos)?);
    Ok(UntaggedPlanDiagnostic {
        expected_pos_present,
        coarse_pos: coarse_pos.iter().copied().map(coarse_pos_name).collect(),
        multi_coarse_pos: coarse_pos.len() > 1,
        literal_fallback: coarse_pos.len() == 1 && coarse_pos.contains(&CoarsePos::Literal),
    })
}

const fn coarse_pos_name(pos: CoarsePos) -> &'static str {
    match pos {
        CoarsePos::Noun => "noun",
        CoarsePos::Pronoun => "pronoun",
        CoarsePos::Numeral => "numeral",
        CoarsePos::Verb => "verb",
        CoarsePos::Adjective => "adjective",
        CoarsePos::Determiner => "determiner",
        CoarsePos::Adverb => "adverb",
        CoarsePos::Particle => "particle",
        CoarsePos::Interjection => "interjection",
        CoarsePos::Literal => "literal",
    }
}

fn append_kfind_diagnostics(
    cases: &[Case],
    results: &mut [Value],
    analyzer: &LexiconQueryAnalyzer,
    component_resource: &Option<Arc<kfind_data::ComponentResource>>,
) -> Result<Option<String>> {
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
    let component_shadow_resource = component_resource
        .as_deref()
        .map_or(ShadowResource::Missing, |resource| {
            ShadowResource::Loaded(resource)
        });
    for (case, result) in cases.iter().zip(results.iter_mut()) {
        result["failure_diagnostic"] = serde_json::to_value(diagnose_failure(case, analyzer)?)?;
        let shadow_verification = diagnose_verification(
            case,
            analyzer,
            shadow_resource,
            component_shadow_resource,
            component_resource.as_ref(),
        )?;
        result["shadow_verification"] = serde_json::to_value(shadow_verification)?;
    }
    Ok(morphology_artifact_sha256)
}

fn diagnose_verification(
    case: &Case,
    analyzer: &LexiconQueryAnalyzer,
    resource: ShadowResource<'_>,
    component_resource: ShadowResource<'_>,
    matcher_component_resource: Option<&Arc<ComponentResource>>,
) -> Result<ShadowVerificationCounters> {
    let options = CompileOptions::resolve(CompileOptionOverrides {
        pos: Some(parse_pos(&case.pos)?),
        ..CompileOptionOverrides::default()
    })?;
    let plan = compile_query(&case.query, &options, analyzer)
        .with_context(|| format!("failed to compile shadow diagnostic for case {}", case.id))?;
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
    if plan.requires_component_resource() {
        if let Some(status) = resource.unavailable_status() {
            bail!(
                "nominal component shadow for case {} requires a valid morphology resource: {status}",
                case.id
            );
        }
        if let Some(status) = component_resource.unavailable_status() {
            bail!(
                "nominal component shadow for case {} requires a valid compact resource: {status}",
                case.id
            );
        }
    }
    let matcher = match matcher_component_resource {
        Some(resource) => {
            MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(resource))?
        }
        None => MorphMatcher::new(Arc::new(plan))?,
    };
    let component_candidates = matcher.local_analysis_candidates(case.text.as_bytes());
    let mut component = Vec::with_capacity(component_candidates.len());
    for candidate in &component_candidates {
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
        component_branches,
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
    let any_boundary_gold_matches = any_matcher
        .find_all_with_meta(case.text.as_bytes())
        .into_iter()
        .flat_map(|matched| matched.atoms)
        .filter(|atom| ranges_overlap(atom.token.clone(), gold_range.clone()))
        .map(|atom| DiagnosticMatch {
            core: DiagnosticSpan {
                byte_start: atom.core.start,
                byte_end: atom.core.end,
            },
            token: DiagnosticSpan {
                byte_start: atom.token.start,
                byte_end: atom.token.end,
            },
            origins: atom
                .origins
                .into_iter()
                .map(|origin| DiagnosticOrigin {
                    analysis_index: origin.analysis_index,
                    rule_path: origin
                        .rule_path
                        .into_iter()
                        .map(|rule| rule.as_str().to_owned())
                        .collect(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let any_boundary_gold_overlap = !any_boundary_gold_matches.is_empty();
    Ok(Some(FailureDiagnostic {
        auto_has_expected_pos_analysis,
        gold_anchor_overlap,
        any_boundary_gold_overlap,
        any_boundary_gold_matches,
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
        DataFinePos, LexiconData, MecabSourceMorphologyEntry, NominalRecord, collect_pos_entries,
        decode_morphology_resource, encode_component_resource, encode_morphology_resource,
        encode_pos_lexicon, parse_mecab_connection_matrix,
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
    fn only_smart_boundary_requires_the_component_resource() {
        assert!(KfindBoundary::parse("smart").unwrap().requires_component());
        assert!(!KfindBoundary::parse("token").unwrap().requires_component());
        assert!(!KfindBoundary::parse("any").unwrap().requires_component());
        assert!(KfindBoundary::parse("non-smart").is_err());
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
        assert_eq!(diagnostic.any_boundary_gold_matches.len(), 1);
        assert_eq!(
            diagnostic.any_boundary_gold_matches[0].token,
            DiagnosticSpan {
                byte_start: "사용자".len(),
                byte_end: "사용자권한".len(),
            }
        );
    }

    #[test]
    fn diagnostic_preserves_any_boundary_rule_paths() {
        let diagnostic = diagnose_failure(&positive_case("먹다", "verb", "먹었다"), &analyzer())
            .unwrap()
            .unwrap();

        assert!(
            diagnostic.any_boundary_gold_matches[0]
                .origins
                .iter()
                .any(|origin| !origin.rule_path.is_empty())
        );
    }

    #[test]
    fn untagged_plan_reports_expected_and_ambiguous_pos() {
        let full_data = LexiconData {
            nominals: vec![NominalRecord {
                lemma: "새".to_owned(),
                pos: DataFinePos::Nng,
                flags: BTreeSet::new(),
                overrides: Vec::new(),
            }],
            ..LexiconData::default()
        };
        let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(
            Lexicons::embedded_with(Some(&binary), None).unwrap(),
        ));

        let diagnostic =
            diagnose_untagged_plan(&positive_case("새", "noun", "새"), &analyzer).unwrap();

        assert!(diagnostic.expected_pos_present);
        assert_eq!(diagnostic.coarse_pos, ["noun", "determiner"]);
        assert!(diagnostic.multi_coarse_pos);
        assert!(!diagnostic.literal_fallback);
    }

    #[test]
    fn untagged_plan_reports_literal_fallback() {
        let diagnostic =
            diagnose_untagged_plan(&positive_case("미등록다", "verb", "미등록다"), &analyzer())
                .unwrap();

        assert!(!diagnostic.expected_pos_present);
        assert_eq!(diagnostic.coarse_pos, ["literal"]);
        assert!(!diagnostic.multi_coarse_pos);
        assert!(diagnostic.literal_fallback);
    }

    #[test]
    fn nominal_component_shadow_requires_a_valid_resource() {
        let error = diagnose_verification(
            &positive_case("권한", "noun", "사용자권한"),
            &analyzer(),
            ShadowResource::Missing,
            ShadowResource::Missing,
            None,
        )
        .unwrap_err();

        assert!(error.to_string().contains("resource-missing"));
    }

    #[test]
    fn nominal_component_shadow_compares_projection_evidence() {
        let bytes = component_fixture_resource(20);
        let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
        let compact = component_fixture_compact_resource(20);
        let counters = diagnose_verification(
            &positive_case("권한", "noun", "사용자권한"),
            &analyzer(),
            ShadowResource::Loaded(&resource),
            ShadowResource::Loaded(compact.as_ref()),
            Some(&compact),
        )
        .unwrap();

        assert_eq!(counters.component_projection_comparisons, 1);
        assert_eq!(counters.nominal_component_candidate_hits, 0);
        assert_eq!(counters.component_projection_mismatches, 0);
    }

    #[test]
    fn nominal_component_shadow_rejects_projection_differences() {
        let full_bytes = component_fixture_resource(20);
        let full = decode_morphology_resource("full", &full_bytes, &[9; 32]).unwrap();
        let compact = component_fixture_compact_resource(2_000);
        let error = diagnose_verification(
            &positive_case("권한", "noun", "사용자권한"),
            &analyzer(),
            ShadowResource::Loaded(&full),
            ShadowResource::Loaded(compact.as_ref()),
            Some(&compact),
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

    fn component_fixture_compact_resource(component_cost: i32) -> Arc<ComponentResource> {
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
        let bytes = encode_component_resource(
            [9; 32],
            &entries,
            &matrix,
            b"DEFAULT 0 1 0\nHANGUL 0 1 2\n0xAC00..0xD7A3 HANGUL\n",
            b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
        )
        .unwrap();
        Arc::new(decode_component_resource("compact", bytes, &[9; 32]).unwrap())
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
