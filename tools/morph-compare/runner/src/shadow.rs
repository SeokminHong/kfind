use kfind_data::DataFinePos;
use kfind_matcher::{AnalysisWindowError, LocalAnalysisCandidate, VerificationCounters};
use kfind_morph::{
    DEFAULT_LATTICE_NODE_LIMIT, FinePos, LocalLatticeDecision, LocalLatticeReport,
    LocalLatticeResource, evaluate_local_component_paths, evaluate_local_lattice,
};
use serde::Serialize;

use super::Span;

#[derive(Debug, Serialize)]
pub(super) struct ShadowVerificationCounters {
    pub(super) raw_anchor_hits: usize,
    pub(super) verified_branch_hits: usize,
    pub(super) local_lattice_candidate_hits: usize,
    pub(super) unique_analysis_windows: usize,
    pub(super) nominal_component_candidate_hits: usize,
    pub(super) unique_component_windows: usize,
    pub(super) component_projection_comparisons: usize,
    pub(super) component_projection_mismatches: usize,
    pub(super) local_branches: Vec<ShadowBranchEvidence>,
    component_branches: Vec<ShadowBranchEvidence>,
    pub(super) lattice: Vec<ShadowLatticeEvidence>,
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
    fine_pos: &'static str,
    target: Span,
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
    word_cost: i32,
    unknown: bool,
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
        local_branches: Vec<ShadowBranchEvidence>,
        component_branches: Vec<ShadowBranchEvidence>,
        lattice: Vec<ShadowLatticeEvidence>,
        component: Vec<ShadowLatticeEvidence>,
        component_projection_comparisons: usize,
    ) -> Self {
        Self {
            raw_anchor_hits: counters.raw_anchor_hits,
            verified_branch_hits: counters.verified_branch_hits,
            local_lattice_candidate_hits: counters.local_lattice_candidate_hits,
            unique_analysis_windows: counters.unique_analysis_windows,
            nominal_component_candidate_hits: counters.nominal_component_candidate_hits,
            unique_component_windows: counters.unique_component_windows,
            component_projection_comparisons,
            component_projection_mismatches: 0,
            local_branches,
            component_branches,
            lattice,
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

pub(super) fn diagnose_lattice_candidate(
    candidate: &LocalAnalysisCandidate,
    resource: ShadowResource<'_>,
) -> ShadowLatticeEvidence {
    diagnose_candidate(candidate, resource, ShadowEvaluation::Lattice)
}

pub(super) fn diagnose_component_candidate(
    candidate: &LocalAnalysisCandidate,
    resource: ShadowResource<'_>,
) -> ShadowLatticeEvidence {
    diagnose_candidate(candidate, resource, ShadowEvaluation::Component)
}

#[derive(Clone, Copy)]
enum ShadowEvaluation {
    Lattice,
    Component,
}

fn diagnose_candidate(
    candidate: &LocalAnalysisCandidate,
    resource: ShadowResource<'_>,
    evaluation: ShadowEvaluation,
) -> ShadowLatticeEvidence {
    let base = |status, error| ShadowLatticeEvidence {
        status,
        atom_index: candidate.atom_index,
        analysis_index: candidate.analysis_index,
        fine_pos: fine_pos_name(candidate.fine_pos),
        target: span(candidate.target.clone()),
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
    let Some(query_pos) = data_fine_pos(candidate.fine_pos) else {
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
    let report = match evaluation {
        ShadowEvaluation::Lattice => evaluate_local_lattice(
            resource,
            window.normalized(),
            query_span,
            query_pos,
            DEFAULT_LATTICE_NODE_LIMIT,
        ),
        ShadowEvaluation::Component => evaluate_local_component_paths(
            resource,
            window.normalized(),
            query_span,
            query_pos,
            DEFAULT_LATTICE_NODE_LIMIT,
        ),
    };
    match report {
        Ok(report) => lattice_evidence(base("evaluated", None), window, report),
        Err(error @ kfind_morph::LocalLatticeError::NodeLimit { .. }) => {
            base("limit-exceeded", Some(error.to_string()))
        }
        Err(error) => base("evaluation-error", Some(error.to_string())),
    }
}

fn lattice_evidence(
    mut evidence: ShadowLatticeEvidence,
    window: &kfind_matcher::AnalysisWindow,
    report: LocalLatticeReport,
) -> ShadowLatticeEvidence {
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
                    word_cost: node.word_cost,
                    unknown: node.unknown,
                })
                .collect(),
        })
        .collect();
    evidence
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

fn fine_pos_name(pos: FinePos) -> &'static str {
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
