use kfind_data::{
    DataFinePos, DecodedMorphologyResource, MorphologyExpressionAlignmentKind,
    align_morphology_expression,
};
use kfind_matcher::{AnalysisWindowError, LocalAnalysisCandidate, VerificationCounters};
use kfind_morph::{
    DEFAULT_LATTICE_NODE_LIMIT, FinePos, LocalLatticeDecision, LocalLatticeReport,
    LocalLatticeResource, evaluate_local_component_paths,
};
use serde::Serialize;

use super::Span;

#[derive(Debug, Serialize)]
pub(super) struct ShadowVerificationCounters {
    pub(super) raw_anchor_hits: usize,
    pub(super) verified_branch_hits: usize,
    pub(super) exact_component_candidate_hits: usize,
    pub(super) unique_component_windows: usize,
    pub(super) component_projection_comparisons: usize,
    pub(super) component_projection_mismatches: usize,
    component_branches: Vec<ShadowBranchEvidence>,
    component: Vec<ShadowLatticeEvidence>,
}

#[derive(Debug, Serialize)]
pub(super) struct ShadowBranchEvidence {
    pub(super) atom_index: usize,
    pub(super) anchor: String,
    pub(super) require_left: bool,
    pub(super) require_right: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct ShadowLatticeEvidence {
    pub(super) status: &'static str,
    atom_index: usize,
    analysis_index: u16,
    rule_path: Vec<String>,
    fine_pos: &'static str,
    query_source_pos: Option<&'static str>,
    target: Span,
    normalized_target: Option<Span>,
    window: Option<ShadowWindowEvidence>,
    decision: Option<&'static str>,
    include_cost: Option<i64>,
    exclude_cost: Option<i64>,
    cost_margin: Option<i64>,
    node_count: Option<usize>,
    paths: Vec<ShadowPathEvidence>,
    error: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ShadowWindowEvidence {
    raw: Span,
    normalized: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ShadowPathEvidence {
    cost: i64,
    includes_query: bool,
    nodes: Vec<ShadowNodeEvidence>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ShadowNodeEvidence {
    normalized: Span,
    original: Option<Span>,
    pos: Option<String>,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    unknown: bool,
    source: Option<ShadowSourceEvidence>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ShadowSourceEvidence {
    kind: &'static str,
    surface: String,
    analyses: Vec<ShadowSourceAnalysis>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ShadowSourceAnalysis {
    pos: String,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    analysis_type: String,
    start_pos: String,
    end_pos: String,
    expression: String,
    expression_alignment: Option<&'static str>,
    components: Vec<ShadowSourceComponent>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ShadowSourceComponent {
    surface: String,
    pos: String,
    surface_span: Option<Span>,
}

#[derive(Clone, Copy)]
pub(super) enum ShadowResource<'a> {
    Loaded(&'a dyn LocalLatticeResource),
    Missing,
    Corrupt,
    SourceMismatch,
}

impl ShadowVerificationCounters {
    pub(super) fn new(
        counters: VerificationCounters,
        component_branches: Vec<ShadowBranchEvidence>,
        component: Vec<ShadowLatticeEvidence>,
        component_projection_comparisons: usize,
    ) -> Self {
        Self {
            raw_anchor_hits: counters.raw_anchor_hits,
            verified_branch_hits: counters.verified_branch_hits,
            exact_component_candidate_hits: counters.exact_component_candidate_hits,
            unique_component_windows: counters.unique_component_windows,
            component_projection_comparisons,
            component_projection_mismatches: 0,
            component_branches,
            component,
        }
    }
}

impl ShadowResource<'_> {
    pub(super) const fn unavailable_status(self) -> Option<&'static str> {
        match self {
            Self::Loaded(_) => None,
            Self::Missing => Some("resource-missing"),
            Self::Corrupt => Some("resource-corrupt"),
            Self::SourceMismatch => Some("source-mismatch"),
        }
    }
}

pub(super) fn diagnose_component_candidate(
    candidate: &LocalAnalysisCandidate,
    resource: ShadowResource<'_>,
) -> ShadowLatticeEvidence {
    diagnose_candidate(candidate, resource)
}

pub(super) fn attach_source_provenance(
    evidence: &mut ShadowLatticeEvidence,
    resource: &DecodedMorphologyResource<'_>,
) {
    let Some(window) = evidence.window.as_ref() else {
        return;
    };
    for path in &mut evidence.paths {
        for node in &mut path.nodes {
            node.source = Some(source_evidence(&window.normalized, node, resource));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn diagnose_agent_candidate(
    atom_index: usize,
    analysis_index: u16,
    rule_path: Vec<kfind_morph::RuleId>,
    fine_pos: FinePos,
    target: std::ops::Range<usize>,
    window: Result<kfind_matcher::AnalysisWindow, AnalysisWindowError>,
    resource: &dyn LocalLatticeResource,
) -> ShadowLatticeEvidence {
    diagnose_candidate(
        &LocalAnalysisCandidate {
            atom_index,
            analysis_index,
            rule_path,
            fine_pos,
            target,
            window,
        },
        ShadowResource::Loaded(resource),
    )
}

fn diagnose_candidate(
    candidate: &LocalAnalysisCandidate,
    resource: ShadowResource<'_>,
) -> ShadowLatticeEvidence {
    let query_source_pos = data_fine_pos(candidate.fine_pos);
    let base = |status, error| ShadowLatticeEvidence {
        status,
        atom_index: candidate.atom_index,
        analysis_index: candidate.analysis_index,
        rule_path: candidate
            .rule_path
            .iter()
            .map(|rule| rule.as_str().to_owned())
            .collect(),
        fine_pos: fine_pos_name(candidate.fine_pos),
        query_source_pos: query_source_pos.map(|pos| pos.as_str()),
        target: span(candidate.target.clone()),
        normalized_target: None,
        window: candidate
            .window
            .as_ref()
            .ok()
            .map(|window| ShadowWindowEvidence {
                raw: span(window.raw_span()),
                normalized: window.normalized().to_owned(),
            }),
        decision: None,
        include_cost: None,
        exclude_cost: None,
        cost_margin: None,
        node_count: None,
        paths: Vec::new(),
        error,
    };
    let window = match &candidate.window {
        Ok(window) => window,
        Err(
            error @ (AnalysisWindowError::RawBytes { .. }
            | AnalysisWindowError::NormalizedScalars { .. }),
        ) => {
            return base("limit-exceeded", Some(error.to_string()));
        }
        Err(error) => return base("evaluation-error", Some(error.to_string())),
    };
    let resource = match resource {
        ShadowResource::Loaded(resource) => resource,
        ShadowResource::Missing => return base("resource-missing", None),
        ShadowResource::Corrupt => return base("resource-corrupt", None),
        ShadowResource::SourceMismatch => return base("source-mismatch", None),
    };
    let Some(query_pos) = query_source_pos else {
        return base(
            "evaluation-error",
            Some("query POS is not represented in the morphology resource".to_owned()),
        );
    };
    let Some(query_span) = window.normalized_span(candidate.target.clone()) else {
        return base(
            "evaluation-error",
            Some("query span does not map to stable NFC boundaries".to_owned()),
        );
    };
    let report = evaluate_local_component_paths(
        resource,
        window.normalized(),
        query_span.clone(),
        query_pos,
        DEFAULT_LATTICE_NODE_LIMIT,
    );
    match report {
        Ok(report) => lattice_evidence(base("evaluated", None), window, query_span, report),
        Err(error @ kfind_morph::LocalLatticeError::NodeLimit { .. }) => {
            base("limit-exceeded", Some(error.to_string()))
        }
        Err(error) => base("evaluation-error", Some(error.to_string())),
    }
}

fn lattice_evidence(
    mut evidence: ShadowLatticeEvidence,
    window: &kfind_matcher::AnalysisWindow,
    query_span: std::ops::Range<usize>,
    report: LocalLatticeReport,
) -> ShadowLatticeEvidence {
    evidence.normalized_target = Some(span(query_span));
    evidence.decision = Some(match report.decision {
        LocalLatticeDecision::Accept => "accept",
        LocalLatticeDecision::Reject => "reject",
        LocalLatticeDecision::Ambiguous => "ambiguous",
    });
    evidence.include_cost = report.include_cost;
    evidence.exclude_cost = report.exclude_cost;
    evidence.cost_margin = report.cost_margin;
    evidence.node_count = Some(report.node_count);
    evidence.paths = report
        .paths
        .into_iter()
        .map(|path| ShadowPathEvidence {
            cost: path.cost,
            includes_query: path.includes_query,
            nodes: path
                .nodes
                .into_iter()
                .map(|node| ShadowNodeEvidence {
                    original: window.original_span(node.span.clone()).map(span),
                    normalized: span(node.span),
                    pos: node.pos,
                    left_id: node.left_id,
                    right_id: node.right_id,
                    word_cost: node.word_cost,
                    unknown: node.unknown,
                    source: None,
                })
                .collect(),
        })
        .collect();
    evidence
}

fn source_evidence(
    normalized: &str,
    node: &ShadowNodeEvidence,
    resource: &DecodedMorphologyResource<'_>,
) -> ShadowSourceEvidence {
    let surface = normalized
        .get(node.normalized.byte_start..node.normalized.byte_end)
        .unwrap_or_default()
        .to_owned();
    if node.unknown {
        return ShadowSourceEvidence {
            kind: "unknown",
            surface,
            analyses: Vec::new(),
        };
    }
    let mut analyses = Vec::new();
    resource.common_prefixes(surface.as_bytes(), |length, candidates| {
        if length != surface.len() {
            return;
        }
        analyses.extend(
            candidates
                .iter()
                .filter(|analysis| {
                    Some(analysis.pos) == node.pos.as_deref()
                        && analysis.left_id == node.left_id
                        && analysis.right_id == node.right_id
                        && analysis.word_cost == node.word_cost
                })
                .map(|analysis| source_analysis(&surface, analysis)),
        );
    });
    let kind = match analyses.as_slice() {
        [analysis] if is_atomic_analysis(analysis) => "source-atomic",
        [analysis] if is_decomposition_analysis(analysis) => "source-decomposition",
        _ => "unresolved",
    };
    ShadowSourceEvidence {
        kind,
        surface,
        analyses,
    }
}

fn source_analysis(
    surface: &str,
    analysis: &kfind_data::MorphologyAnalysis<'_>,
) -> ShadowSourceAnalysis {
    let has_expression = !matches!(analysis.expression, "" | "*");
    let alignment = has_expression.then(|| align_morphology_expression(surface, analysis.expression));
    let components = alignment
        .as_ref()
        .map(|alignment| {
            alignment
                .components
                .iter()
                .map(|component| ShadowSourceComponent {
                    surface: component.surface.to_owned(),
                    pos: component.pos.to_owned(),
                    surface_span: component.span.clone().map(span),
                })
                .collect()
        })
        .unwrap_or_default();
    ShadowSourceAnalysis {
        pos: analysis.pos.to_owned(),
        left_id: analysis.left_id,
        right_id: analysis.right_id,
        word_cost: analysis.word_cost,
        analysis_type: analysis.analysis_type.to_owned(),
        start_pos: analysis.start_pos.to_owned(),
        end_pos: analysis.end_pos.to_owned(),
        expression: analysis.expression.to_owned(),
        expression_alignment: alignment.map(|alignment| match alignment.kind {
            MorphologyExpressionAlignmentKind::SpanAligned => "span-aligned",
            MorphologyExpressionAlignmentKind::Fused => "fused",
            MorphologyExpressionAlignmentKind::Unaligned => "unaligned",
            MorphologyExpressionAlignmentKind::Invalid => "invalid",
        }),
        components,
    }
}

fn is_atomic_analysis(analysis: &ShadowSourceAnalysis) -> bool {
    matches!(analysis.analysis_type.as_str(), "" | "*")
        && matches!(analysis.expression.as_str(), "" | "*")
}

fn is_decomposition_analysis(analysis: &ShadowSourceAnalysis) -> bool {
    !matches!(analysis.analysis_type.as_str(), "" | "*")
        && !matches!(analysis.expression.as_str(), "" | "*")
}

fn span(range: std::ops::Range<usize>) -> Span {
    Span {
        byte_start: range.start,
        byte_end: range.end,
    }
}

fn data_fine_pos(pos: FinePos) -> Option<DataFinePos> {
    Some(match pos {
        FinePos::CommonNoun => DataFinePos::Nng,
        FinePos::ProperNoun => DataFinePos::Nnp,
        FinePos::DependentNoun => DataFinePos::Nnb,
        FinePos::Pronoun => DataFinePos::Np,
        FinePos::Numeral => DataFinePos::Nr,
        FinePos::Verb => DataFinePos::Vv,
        FinePos::Adjective => DataFinePos::Va,
        FinePos::AuxiliaryVerb | FinePos::AuxiliaryAdjective => DataFinePos::Vx,
        FinePos::Copula => DataFinePos::Vcp,
        FinePos::Determiner => DataFinePos::Mm,
        FinePos::GeneralAdverb => DataFinePos::Mag,
        FinePos::ConjunctiveAdverb => DataFinePos::Maj,
        FinePos::Interjection => DataFinePos::Ic,
        FinePos::Particle
        | FinePos::Foreign
        | FinePos::Number
        | FinePos::Code
        | FinePos::Literal => return None,
    })
}

pub(super) fn fine_pos_name(pos: FinePos) -> &'static str {
    match pos {
        FinePos::CommonNoun => "common-noun",
        FinePos::ProperNoun => "proper-noun",
        FinePos::DependentNoun => "dependent-noun",
        FinePos::Pronoun => "pronoun",
        FinePos::Numeral => "numeral",
        FinePos::Verb => "verb",
        FinePos::Adjective => "adjective",
        FinePos::AuxiliaryVerb => "auxiliary-verb",
        FinePos::AuxiliaryAdjective => "auxiliary-adjective",
        FinePos::Copula => "VCP",
        FinePos::Determiner => "determiner",
        FinePos::GeneralAdverb => "general-adverb",
        FinePos::ConjunctiveAdverb => "conjunctive-adverb",
        FinePos::Particle => "particle",
        FinePos::Interjection => "interjection",
        FinePos::Foreign => "foreign",
        FinePos::Number => "number",
        FinePos::Code => "code",
        FinePos::Literal => "literal",
    }
}
