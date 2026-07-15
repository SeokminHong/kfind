use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, decode_component_resource, decode_morphology_graph_resource,
};
use kfind_matcher::{
    AnalysisWindow, DEFAULT_ANALYSIS_WINDOW_LIMITS, MorphMatcher, is_token_character,
};
use kfind_morph::{
    BoundedTokenContext, CandidateSpans, ConstraintEvidenceKind, ConstraintResolver,
    DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT, DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT, MorphContinuation,
    PreparedQueryAnalysis, PreparedTokenAnalysis, ProductPolicy,
};
use kfind_query::{
    BoundaryPolicy, CompileOptionOverrides, CompileOptions, CoreMapping, LexiconQueryAnalyzer,
    SurfaceBranch, compile_query,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use super::graph_shadow::outcome_name;
use super::{
    COMPONENT_RESOURCE, COMPONENT_RESOURCE_ENV, Case, GRAPH_RESOURCE, GRAPH_RESOURCE_ENV,
    KfindProfile, Span, find_all_spans, load_full_profile_lexicons, parse_pos, peak_rss_kib,
};

const POLICIES: [(&str, ProductPolicy); 4] = [
    ("whole", ProductPolicy::Whole),
    ("explicit-component", ProductPolicy::ExplicitComponent),
    ("possible-analysis", ProductPolicy::PossibleAnalysis),
    ("unambiguous-analysis", ProductPolicy::UnambiguousAnalysis),
];

#[derive(Debug, Serialize)]
pub(super) struct ConstraintEvaluationSummary {
    backend: &'static str,
    version: &'static str,
    profile: &'static str,
    lexicon_artifact_sha256: Option<String>,
    enriched_artifact_sha256: Option<String>,
    component_artifact_sha256: String,
    graph_artifact_sha256: String,
    initialization_seconds: f64,
    evaluation_seconds: f64,
    compile_seconds: f64,
    product_seconds: f64,
    candidate_enumeration_seconds: f64,
    resolver_seconds: f64,
    graph_preparation_seconds: f64,
    decision_seconds: f64,
    policy_seconds: f64,
    diagnostic_seconds: f64,
    peak_rss_kib: Option<u64>,
    metrics: ConstraintEvaluationMetrics,
    results: Vec<ConstraintCaseResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ConstraintProductControl {
    profile: String,
    component_artifact_sha256: String,
    product_seconds: f64,
    spans_by_case: BTreeMap<String, Vec<Span>>,
}

#[derive(Debug, Serialize)]
struct ConstraintEvaluationMetrics {
    cases: usize,
    positive_cases: usize,
    candidate_covered_positive_cases: usize,
    candidate_coverage_percent: f64,
    candidates: usize,
    candidate_statuses_by_class: BTreeMap<&'static str, BTreeMap<&'static str, usize>>,
    outcomes_by_class: BTreeMap<&'static str, BTreeMap<String, usize>>,
    evidence_by_class: BTreeMap<&'static str, BTreeMap<&'static str, usize>>,
    product_quality: QualityCounts,
    policy_quality: BTreeMap<&'static str, QualityCounts>,
    disagreement_from_product: BTreeMap<&'static str, usize>,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
struct QualityCounts {
    tp: usize,
    fp: usize,
    tn: usize,
    #[serde(rename = "fn")]
    fn_count: usize,
    precision_percent: f64,
    recall_percent: f64,
}

impl QualityCounts {
    fn observe(&mut self, expected: bool, predicted: bool) {
        match (expected, predicted) {
            (true, true) => self.tp += 1,
            (false, true) => self.fp += 1,
            (false, false) => self.tn += 1,
            (true, false) => self.fn_count += 1,
        }
    }

    fn finish(mut self) -> Self {
        let predicted = self.tp + self.fp;
        let positive = self.tp + self.fn_count;
        self.precision_percent = if predicted == 0 {
            0.0
        } else {
            100.0 * self.tp as f64 / predicted as f64
        };
        self.recall_percent = if positive == 0 {
            0.0
        } else {
            100.0 * self.tp as f64 / positive as f64
        };
        self
    }
}

#[derive(Debug, Serialize)]
struct ConstraintCaseResult {
    id: String,
    expected: bool,
    gold: Option<Span>,
    candidate_covered: bool,
    product_spans: Vec<Span>,
    product_prediction: bool,
    policy_spans: BTreeMap<&'static str, Vec<Span>>,
    policy_predictions: BTreeMap<&'static str, bool>,
    latency_ms: f64,
    candidates: Vec<ConstraintCandidateEvidence>,
}

#[derive(Debug, Serialize)]
struct ConstraintCandidateEvidence {
    atom_index: usize,
    branch_index: usize,
    status: &'static str,
    core: Span,
    anchor: Span,
    consumed: Option<Span>,
    token: Option<Span>,
    previous: Option<String>,
    current: Option<String>,
    next: Option<String>,
    outcome: Option<String>,
    evidence: Vec<&'static str>,
    policies: BTreeMap<&'static str, bool>,
    error: Option<String>,
}

#[derive(Default)]
struct EvaluationTimings {
    compile_seconds: f64,
    product_seconds: f64,
    candidate_enumeration_seconds: f64,
    resolver_seconds: f64,
    graph_preparation_seconds: f64,
    decision_seconds: f64,
    policy_seconds: f64,
    diagnostic_seconds: f64,
}

#[derive(Default)]
struct EvaluationMetrics {
    positive_cases: usize,
    candidate_covered_positive_cases: usize,
    candidates: usize,
    candidate_statuses_by_class: BTreeMap<&'static str, BTreeMap<&'static str, usize>>,
    outcomes_by_class: BTreeMap<&'static str, BTreeMap<String, usize>>,
    evidence_by_class: BTreeMap<&'static str, BTreeMap<&'static str, usize>>,
    product_quality: QualityCounts,
    policy_quality: BTreeMap<&'static str, QualityCounts>,
    disagreement_from_product: BTreeMap<&'static str, usize>,
}

#[derive(Clone, Copy)]
struct CandidateEvaluationOptions {
    expected: bool,
    verify_diagnostic_parity: bool,
}

pub(super) fn run_constraint_evaluation(
    cases: &[Case],
    profile: KfindProfile,
    verify_diagnostic_parity: bool,
    mut product_control: ConstraintProductControl,
) -> Result<ConstraintEvaluationSummary> {
    if product_control.profile != profile.name() {
        bail!(
            "constraint product control profile {:?} does not match {:?}",
            product_control.profile,
            profile.name()
        );
    }
    if product_control.spans_by_case.len() != cases.len() {
        bail!(
            "constraint product control contains {} cases, expected {}",
            product_control.spans_by_case.len(),
            cases.len()
        );
    }
    let initialization_started = Instant::now();
    let (lexicons, lexicon_artifact_sha256, enriched_artifact_sha256) = match profile {
        KfindProfile::Embedded => (super::Lexicons::embedded()?, None, None),
        KfindProfile::FullPos => {
            let (lexicons, full_pos, enriched) = load_full_profile_lexicons()?;
            (
                lexicons,
                Some(format!("{:x}", Sha256::digest(full_pos))),
                Some(format!("{:x}", Sha256::digest(enriched))),
            )
        }
    };
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
    let analyzer_initialization_seconds = initialization_started.elapsed().as_secs_f64();
    let mut timings = EvaluationTimings::default();
    let mut prepared_cases = Vec::with_capacity(cases.len());
    for case in cases {
        let preparation_started = Instant::now();
        let compile_started = Instant::now();
        let options = CompileOptions::resolve(CompileOptionOverrides {
            boundary: Some(BoundaryPolicy::Smart),
            pos: Some(parse_pos(&case.pos)?),
            ..CompileOptionOverrides::default()
        })?;
        let plan = compile_query(&case.query, &options, &analyzer)
            .with_context(|| format!("failed to compile constraint case {}", case.id))?;
        timings.compile_seconds += compile_started.elapsed().as_secs_f64();
        let candidate_started = Instant::now();
        let candidates = enumerate_candidates(&case.text, &plan);
        timings.candidate_enumeration_seconds += candidate_started.elapsed().as_secs_f64();
        prepared_cases.push(PreparedCaseInput {
            candidates,
            preparation_seconds: preparation_started.elapsed().as_secs_f64(),
        });
    }
    let component_artifact_sha256 = product_control.component_artifact_sha256;
    timings.product_seconds = product_control.product_seconds;

    let graph_initialization_started = Instant::now();
    let graph_path = resource_path(GRAPH_RESOURCE_ENV, GRAPH_RESOURCE);
    let graph_bytes = fs::read(&graph_path).with_context(|| {
        format!(
            "constraint evaluation requires graph resource {}",
            graph_path.display()
        )
    })?;
    let graph_artifact_sha256 = format!("{:x}", Sha256::digest(&graph_bytes));
    let graph = decode_morphology_graph_resource(
        &graph_path.display().to_string(),
        graph_bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )?;
    let resolver = ConstraintResolver::new(Arc::new(graph));
    let initialization_seconds =
        analyzer_initialization_seconds + graph_initialization_started.elapsed().as_secs_f64();

    let mut metrics = EvaluationMetrics::default();
    let mut results = Vec::with_capacity(cases.len());
    for (case, prepared) in cases.iter().zip(prepared_cases) {
        let product_spans = product_control
            .spans_by_case
            .remove(&case.id)
            .with_context(|| format!("constraint product control omitted case {}", case.id))?;
        let evaluation_before = timings.candidate_enumeration_seconds
            + timings.resolver_seconds
            + timings.policy_seconds;
        let raw_candidates = prepared.candidates;
        metrics.candidates += raw_candidates.len();

        let gold = case
            .gold_byte_start
            .zip(case.gold_byte_end)
            .map(|(start, end)| start..end);
        let candidate_covered = case.expected
            && gold.as_ref().is_some_and(|gold| {
                raw_candidates
                    .iter()
                    .any(|candidate| overlaps(&candidate.core, gold))
            });
        if case.expected {
            metrics.positive_cases += 1;
            metrics.candidate_covered_positive_cases += usize::from(candidate_covered);
        }
        let product_prediction = case_prediction(case, &product_spans);
        metrics
            .product_quality
            .observe(case.expected, product_prediction);

        let mut policy_spans = POLICIES
            .iter()
            .map(|(name, _)| (*name, BTreeSet::new()))
            .collect::<BTreeMap<_, _>>();
        let total_started = Instant::now();
        let resolver_before = timings.resolver_seconds;
        let policy_before = timings.policy_seconds;
        let diagnostic_before = timings.diagnostic_seconds;
        let candidates = evaluate_candidates(
            &case.text,
            raw_candidates,
            &resolver,
            &mut policy_spans,
            &mut metrics,
            &mut timings,
            CandidateEvaluationOptions {
                expected: case.expected,
                verify_diagnostic_parity,
            },
        );
        let measured = (timings.resolver_seconds - resolver_before)
            + (timings.policy_seconds - policy_before)
            + (timings.diagnostic_seconds - diagnostic_before);
        timings.candidate_enumeration_seconds +=
            (total_started.elapsed().as_secs_f64() - measured).max(0.0);
        for evidence in &candidates {
            *metrics
                .candidate_statuses_by_class
                .entry(if case.expected {
                    "positive"
                } else {
                    "negative"
                })
                .or_default()
                .entry(evidence.status)
                .or_default() += 1;
        }
        let policy_spans = policy_spans
            .into_iter()
            .map(|(name, spans)| {
                (
                    name,
                    spans
                        .into_iter()
                        .map(|(byte_start, byte_end)| Span {
                            byte_start,
                            byte_end,
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let policy_predictions = POLICIES
            .iter()
            .map(|(name, _)| (*name, case_prediction(case, &policy_spans[name])))
            .collect::<BTreeMap<_, _>>();
        for (name, predicted) in &policy_predictions {
            metrics
                .policy_quality
                .entry(name)
                .or_default()
                .observe(case.expected, *predicted);
            if *predicted != product_prediction {
                *metrics.disagreement_from_product.entry(name).or_default() += 1;
            }
        }
        let evaluation_after = timings.candidate_enumeration_seconds
            + timings.resolver_seconds
            + timings.policy_seconds;
        results.push(ConstraintCaseResult {
            id: case.id.clone(),
            expected: case.expected,
            gold: gold.map(|gold| Span {
                byte_start: gold.start,
                byte_end: gold.end,
            }),
            candidate_covered,
            product_spans,
            product_prediction,
            policy_spans,
            policy_predictions,
            latency_ms: 1_000.0
                * (prepared.preparation_seconds + evaluation_after - evaluation_before),
            candidates,
        });
    }
    let positive_cases = metrics.positive_cases;
    let candidate_covered_positive_cases = metrics.candidate_covered_positive_cases;
    let policy_quality = metrics
        .policy_quality
        .into_iter()
        .map(|(name, quality)| (name, quality.finish()))
        .collect();
    let metrics = ConstraintEvaluationMetrics {
        cases: cases.len(),
        positive_cases,
        candidate_covered_positive_cases,
        candidate_coverage_percent: if positive_cases == 0 {
            0.0
        } else {
            100.0 * candidate_covered_positive_cases as f64 / positive_cases as f64
        },
        candidates: metrics.candidates,
        candidate_statuses_by_class: metrics.candidate_statuses_by_class,
        outcomes_by_class: metrics.outcomes_by_class,
        evidence_by_class: metrics.evidence_by_class,
        product_quality: metrics.product_quality.finish(),
        policy_quality,
        disagreement_from_product: metrics.disagreement_from_product,
    };
    Ok(ConstraintEvaluationSummary {
        backend: "kfind-constraint-resolver",
        version: env!("CARGO_PKG_VERSION"),
        profile: profile.name(),
        lexicon_artifact_sha256,
        enriched_artifact_sha256,
        component_artifact_sha256,
        graph_artifact_sha256,
        initialization_seconds,
        evaluation_seconds: timings.compile_seconds
            + timings.candidate_enumeration_seconds
            + timings.resolver_seconds
            + timings.policy_seconds,
        compile_seconds: timings.compile_seconds,
        product_seconds: timings.product_seconds,
        candidate_enumeration_seconds: timings.candidate_enumeration_seconds,
        resolver_seconds: timings.resolver_seconds,
        graph_preparation_seconds: timings.graph_preparation_seconds,
        decision_seconds: timings.decision_seconds,
        policy_seconds: timings.policy_seconds,
        diagnostic_seconds: timings.diagnostic_seconds,
        peak_rss_kib: peak_rss_kib(),
        metrics,
        results,
    })
}

pub(super) fn run_constraint_product_control(
    cases: &[Case],
    profile: KfindProfile,
) -> Result<ConstraintProductControl> {
    let lexicons = match profile {
        KfindProfile::Embedded => super::Lexicons::embedded()?,
        KfindProfile::FullPos => load_full_profile_lexicons()?.0,
    };
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
    let component_path = resource_path(COMPONENT_RESOURCE_ENV, COMPONENT_RESOURCE);
    let component_bytes = fs::read(&component_path).with_context(|| {
        format!(
            "constraint product control requires component resource {}",
            component_path.display()
        )
    })?;
    let component_artifact_sha256 = format!("{:x}", Sha256::digest(&component_bytes));
    let component = Arc::new(decode_component_resource(
        &component_path.display().to_string(),
        component_bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )?);
    let product_started = Instant::now();
    let mut spans_by_case = BTreeMap::new();
    for case in cases {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            boundary: Some(BoundaryPolicy::Smart),
            pos: Some(parse_pos(&case.pos)?),
            ..CompileOptionOverrides::default()
        })?;
        let plan = compile_query(&case.query, &options, &analyzer)
            .with_context(|| format!("failed to compile product control case {}", case.id))?;
        let matcher =
            MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(&component))?;
        spans_by_case.insert(case.id.clone(), find_all_spans(&matcher, &case.text));
    }
    Ok(ConstraintProductControl {
        profile: profile.name().to_owned(),
        component_artifact_sha256,
        product_seconds: product_started.elapsed().as_secs_f64(),
        spans_by_case,
    })
}

struct PreparedCaseInput {
    candidates: Vec<RawCandidate>,
    preparation_seconds: f64,
}

#[derive(Clone)]
struct RawCandidate {
    atom_index: usize,
    branch_index: usize,
    core: Range<usize>,
    anchor: Range<usize>,
    consume_token: bool,
    branch: SurfaceBranch,
}

fn enumerate_candidates(text: &str, plan: &kfind_query::QueryPlan) -> Vec<RawCandidate> {
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        for (branch_index, branch) in atom.branches.iter().enumerate() {
            if branch.morph_patterns.is_empty() {
                continue;
            }
            let Ok(anchor_text) = std::str::from_utf8(&branch.anchor) else {
                continue;
            };
            for anchor in anchor_occurrences(text, anchor_text) {
                let core_length = match branch.core_mapping {
                    CoreMapping::WholeAnchor => anchor.len(),
                    CoreMapping::PrefixBytes(length) => length,
                };
                let Some(core_end) = anchor.start.checked_add(core_length) else {
                    continue;
                };
                if core_length == 0 || core_end > anchor.end || !text.is_char_boundary(core_end) {
                    continue;
                }
                let core = anchor.start..core_end;
                let consumption_modes: &[bool] = if branch.morph_patterns.iter().any(|pattern| {
                    matches!(pattern.continuation, MorphContinuation::NominalParticles)
                }) {
                    &[false, true]
                } else if branch
                    .morph_patterns
                    .iter()
                    .all(|pattern| matches!(pattern.continuation, MorphContinuation::Exact))
                {
                    &[false]
                } else {
                    &[true]
                };
                for &consume_token in consumption_modes {
                    let key = (
                        atom_index,
                        branch_index,
                        core.start,
                        core.end,
                        anchor.end,
                        consume_token,
                    );
                    if seen.insert(key) {
                        candidates.push(RawCandidate {
                            atom_index,
                            branch_index,
                            core: core.clone(),
                            anchor: anchor.clone(),
                            consume_token,
                            branch: branch.clone(),
                        });
                    }
                }
            }
        }
    }
    candidates
}

struct CandidateEvaluationInput {
    candidate: RawCandidate,
    window: AnalysisWindow,
    spans: CandidateSpans,
    consumed_original: Range<usize>,
    context: AdjacentContext,
}

enum CandidateEvaluation {
    Ready(CandidateEvaluationInput),
    Unavailable(ConstraintCandidateEvidence),
}

struct PreparedCandidateAnalysis<'prepared, 'text> {
    token: &'prepared PreparedTokenAnalysis<'text>,
    query: &'prepared PreparedQueryAnalysis,
}

fn evaluate_candidates(
    text: &str,
    candidates: Vec<RawCandidate>,
    resolver: &ConstraintResolver,
    policy_spans: &mut BTreeMap<&'static str, BTreeSet<(usize, usize)>>,
    metrics: &mut EvaluationMetrics,
    timings: &mut EvaluationTimings,
    options: CandidateEvaluationOptions,
) -> Vec<ConstraintCandidateEvidence> {
    let inputs = candidates
        .into_iter()
        .map(|candidate| prepare_candidate(text, candidate))
        .collect::<Vec<_>>();
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, input) in inputs.iter().enumerate() {
        if let CandidateEvaluation::Ready(input) = input {
            groups
                .entry(input.window.normalized().to_owned())
                .or_default()
                .push(index);
        }
    }
    let mut evidence = std::iter::repeat_with(|| None)
        .take(inputs.len())
        .collect::<Vec<_>>();
    let mut prepared_queries = BTreeMap::<Vec<_>, PreparedQueryAnalysis>::new();
    for input in &inputs {
        let CandidateEvaluation::Ready(input) = input else {
            continue;
        };
        let patterns = &input.candidate.branch.morph_patterns;
        if prepared_queries.contains_key(patterns) {
            continue;
        }
        let resolver_started = Instant::now();
        prepared_queries.insert(
            patterns.clone(),
            resolver.prepare_query_analysis(
                patterns,
                DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
                DEFAULT_ANALYSIS_GRAPH_PATH_LIMIT,
            ),
        );
        let elapsed = resolver_started.elapsed().as_secs_f64();
        timings.resolver_seconds += elapsed;
        timings.graph_preparation_seconds += elapsed;
    }
    for indices in groups.values() {
        let CandidateEvaluation::Ready(first) = &inputs[indices[0]] else {
            unreachable!("candidate group contains evaluated input");
        };
        let resolver_started = Instant::now();
        let prepared =
            resolver.prepare_token(first.window.normalized(), DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT);
        let elapsed = resolver_started.elapsed().as_secs_f64();
        timings.resolver_seconds += elapsed;
        timings.graph_preparation_seconds += elapsed;
        for &index in indices {
            let CandidateEvaluation::Ready(input) = &inputs[index] else {
                unreachable!("candidate group contains evaluated input");
            };
            let query = prepared_queries
                .get(&input.candidate.branch.morph_patterns)
                .expect("ready candidate query was prepared");
            evidence[index] = Some(evaluate_prepared_candidate(
                input,
                PreparedCandidateAnalysis {
                    token: &prepared,
                    query,
                },
                resolver,
                policy_spans,
                metrics,
                timings,
                options,
            ));
        }
    }
    for (index, input) in inputs.into_iter().enumerate() {
        if let CandidateEvaluation::Unavailable(unavailable) = input {
            evidence[index] = Some(unavailable);
        }
    }
    evidence
        .into_iter()
        .map(|evidence| evidence.expect("every candidate produces evidence"))
        .collect()
}

fn prepare_candidate(text: &str, candidate: RawCandidate) -> CandidateEvaluation {
    let window = match AnalysisWindow::extract(
        text.as_bytes(),
        candidate.core.clone(),
        DEFAULT_ANALYSIS_WINDOW_LIMITS,
    ) {
        Ok(window) => window,
        Err(error) => {
            return CandidateEvaluation::Unavailable(candidate_base(
                &candidate,
                "window-unavailable",
                Some(error.to_string()),
            ));
        }
    };
    let Some(core) = window.normalized_span(candidate.core.clone()) else {
        return CandidateEvaluation::Unavailable(candidate_base(
            &candidate,
            "core-unavailable",
            Some("candidate core does not map to stable NFC boundaries".to_owned()),
        ));
    };
    let Some(anchor) = window.normalized_span(candidate.anchor.clone()) else {
        return CandidateEvaluation::Unavailable(candidate_base(
            &candidate,
            "anchor-unavailable",
            Some("candidate anchor does not map to stable NFC boundaries".to_owned()),
        ));
    };
    let token = 0..window.normalized().len();
    let consumed = if candidate.consume_token {
        anchor.start..token.end
    } else {
        anchor.clone()
    };
    let Some(consumed_original) = window.original_span(consumed.clone()) else {
        return CandidateEvaluation::Unavailable(candidate_base(
            &candidate,
            "consumed-unavailable",
            Some("candidate consumed span does not map to stable raw boundaries".to_owned()),
        ));
    };
    let context = match adjacent_context(text, window.raw_span()) {
        Ok(context) => context,
        Err(error) => {
            return CandidateEvaluation::Unavailable(candidate_base(
                &candidate,
                "context-unavailable",
                Some(error.to_owned()),
            ));
        }
    };
    CandidateEvaluation::Ready(CandidateEvaluationInput {
        candidate,
        window,
        spans: CandidateSpans {
            core,
            anchor,
            consumed,
            token,
        },
        consumed_original,
        context,
    })
}

fn candidate_base(
    candidate: &RawCandidate,
    status: &'static str,
    error: Option<String>,
) -> ConstraintCandidateEvidence {
    ConstraintCandidateEvidence {
        atom_index: candidate.atom_index,
        branch_index: candidate.branch_index,
        status,
        core: to_span(candidate.core.clone()),
        anchor: to_span(candidate.anchor.clone()),
        consumed: None,
        token: None,
        previous: None,
        current: None,
        next: None,
        outcome: None,
        evidence: Vec::new(),
        policies: BTreeMap::new(),
        error,
    }
}

fn evaluate_prepared_candidate(
    input: &CandidateEvaluationInput,
    prepared: PreparedCandidateAnalysis<'_, '_>,
    resolver: &ConstraintResolver,
    policy_spans: &mut BTreeMap<&'static str, BTreeSet<(usize, usize)>>,
    metrics: &mut EvaluationMetrics,
    timings: &mut EvaluationTimings,
    options: CandidateEvaluationOptions,
) -> ConstraintCandidateEvidence {
    let candidate = &input.candidate;
    let resolver_started = Instant::now();
    let resolver_context = BoundedTokenContext {
        previous: input.context.previous.as_deref(),
        current: input.window.normalized(),
        next: input.context.next.as_deref(),
    };
    let decision = resolver.decide_prepared_query_candidate(
        prepared.token,
        resolver_context,
        input.spans.clone(),
        prepared.query,
    );
    let elapsed = resolver_started.elapsed().as_secs_f64();
    timings.resolver_seconds += elapsed;
    timings.decision_seconds += elapsed;
    if options.verify_diagnostic_parity {
        let diagnostic_started = Instant::now();
        let resolution = resolver.resolve_prepared_query_candidate(
            prepared.token,
            BoundedTokenContext {
                previous: input.context.previous.as_deref(),
                current: input.window.normalized(),
                next: input.context.next.as_deref(),
            },
            input.spans.clone(),
            prepared.query,
        );
        assert_eq!(
            decision,
            resolution.decision(),
            "decision and proof diverged"
        );
        timings.diagnostic_seconds += diagnostic_started.elapsed().as_secs_f64();
    }
    let class = if options.expected {
        "positive"
    } else {
        "negative"
    };
    *metrics
        .outcomes_by_class
        .entry(class)
        .or_default()
        .entry(outcome_name(decision.outcome))
        .or_default() += 1;
    let mut evidence = decision
        .supported
        .iter()
        .map(|support| evidence_name(support.evidence))
        .collect::<Vec<_>>();
    evidence.sort_unstable();
    evidence.dedup();
    for kind in &evidence {
        *metrics
            .evidence_by_class
            .entry(class)
            .or_default()
            .entry(kind)
            .or_default() += 1;
    }
    let policy_started = Instant::now();
    let policies = POLICIES
        .iter()
        .map(|(name, policy)| {
            let accepted = policy.accepts_decision(&decision, &candidate.branch.morph_patterns);
            if accepted {
                policy_spans
                    .get_mut(name)
                    .expect("known product policy")
                    .insert((input.consumed_original.start, input.consumed_original.end));
            }
            (*name, accepted)
        })
        .collect::<BTreeMap<_, _>>();
    timings.policy_seconds += policy_started.elapsed().as_secs_f64();
    ConstraintCandidateEvidence {
        atom_index: candidate.atom_index,
        branch_index: candidate.branch_index,
        status: "evaluated",
        core: to_span(candidate.core.clone()),
        anchor: to_span(candidate.anchor.clone()),
        consumed: Some(to_span(input.consumed_original.clone())),
        token: Some(to_span(input.window.raw_span())),
        previous: input.context.previous.clone(),
        current: Some(input.window.normalized().to_owned()),
        next: input.context.next.clone(),
        outcome: Some(outcome_name(decision.outcome)),
        evidence,
        policies,
        error: None,
    }
}

struct AdjacentContext {
    previous: Option<String>,
    next: Option<String>,
}

fn adjacent_context(text: &str, current: Range<usize>) -> Result<AdjacentContext, &'static str> {
    let previous = adjacent_token(text, current.start, Direction::Previous);
    let next = adjacent_token(text, current.end, Direction::Next);
    let context_start = previous.as_ref().map_or(current.start, |span| span.start);
    let context_end = next.as_ref().map_or(current.end, |span| span.end);
    if context_end.saturating_sub(context_start) > DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes {
        return Err("bounded context exceeds raw byte limit");
    }
    let normalized_context = text[context_start..context_end].nfc().collect::<String>();
    if normalized_context.chars().count() > DEFAULT_ANALYSIS_WINDOW_LIMITS.max_normalized_scalars {
        return Err("bounded context exceeds normalized scalar limit");
    }
    Ok(AdjacentContext {
        previous: previous.map(|span| text[span].nfc().collect()),
        next: next.map(|span| text[span].nfc().collect()),
    })
}

#[derive(Clone, Copy)]
enum Direction {
    Previous,
    Next,
}

fn adjacent_token(text: &str, at: usize, direction: Direction) -> Option<Range<usize>> {
    match direction {
        Direction::Previous => {
            let mut cursor = at;
            let end = loop {
                let (start, character) = text[..cursor].char_indices().next_back()?;
                if matches!(character, '\r' | '\n') {
                    return None;
                }
                cursor = start;
                if is_token_character(character) {
                    break start + character.len_utf8();
                }
            };
            let mut start = cursor;
            while let Some((previous, character)) = text[..start].char_indices().next_back() {
                if !is_token_character(character) {
                    break;
                }
                start = previous;
            }
            Some(start..end)
        }
        Direction::Next => {
            let mut cursor = at;
            let start = loop {
                let character = text[cursor..].chars().next()?;
                if matches!(character, '\r' | '\n') {
                    return None;
                }
                if is_token_character(character) {
                    break cursor;
                }
                cursor += character.len_utf8();
            };
            let mut end = start;
            for character in text[start..].chars() {
                if !is_token_character(character) {
                    break;
                }
                end += character.len_utf8();
            }
            Some(start..end)
        }
    }
}

fn anchor_occurrences(text: &str, anchor: &str) -> Vec<Range<usize>> {
    if anchor.is_empty() || anchor.len() > text.len() {
        return Vec::new();
    }
    text.as_bytes()
        .windows(anchor.len())
        .enumerate()
        .filter_map(|(start, window)| {
            let end = start + anchor.len();
            (window == anchor.as_bytes()
                && text.is_char_boundary(start)
                && text.is_char_boundary(end))
            .then_some(start..end)
        })
        .collect()
}

fn case_prediction(case: &Case, spans: &[Span]) -> bool {
    if !case.expected {
        return !spans.is_empty();
    }
    case.gold_byte_start
        .zip(case.gold_byte_end)
        .is_some_and(|(start, end)| {
            spans
                .iter()
                .any(|span| span.byte_start < end && start < span.byte_end)
        })
}

fn overlaps(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn to_span(range: Range<usize>) -> Span {
    Span {
        byte_start: range.start,
        byte_end: range.end,
    }
}

fn resource_path(environment: &str, fallback: &str) -> PathBuf {
    std::env::var_os(environment)
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback.into())
}

const fn evidence_name(evidence: ConstraintEvidenceKind) -> &'static str {
    match evidence {
        ConstraintEvidenceKind::SourceWhole => "source-whole",
        ConstraintEvidenceKind::SourceComponent => "source-component",
        ConstraintEvidenceKind::RuntimeComposed => "runtime-composed",
        ConstraintEvidenceKind::OpaqueExpression => "opaque-expression",
        ConstraintEvidenceKind::Contradiction => "contradiction",
        ConstraintEvidenceKind::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_overlapping_anchors_without_verifying_boundaries() {
        assert_eq!(
            anchor_occurrences("가가가", "가가"),
            [0.."가가".len(), "가".len().."가가가".len()]
        );
    }

    #[test]
    fn extracts_only_same_line_adjacent_tokens() {
        let text = "앞 매일 매일\n뒤";
        let current_start = text.find("매일 매일").unwrap() + "매일 ".len();
        let current = current_start..current_start + "매일".len();
        let context = adjacent_context(text, current).unwrap();

        assert_eq!(context.previous.as_deref(), Some("매일"));
        assert_eq!(context.next, None);
    }
}
