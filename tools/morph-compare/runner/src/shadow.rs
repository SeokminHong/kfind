use kfind_data::DataFinePos;
use kfind_matcher::{AnalysisWindowError, LocalAnalysisCandidate};
use kfind_morph::{
    DEFAULT_LATTICE_NODE_LIMIT, FinePos, LocalLatticeDecision, LocalLatticeReport,
    evaluate_local_component_paths, evaluate_local_lattice,
};

use super::{
    ShadowLatticeEvidence, ShadowNodeEvidence, ShadowPathEvidence, ShadowResource,
    ShadowWindowEvidence, Span,
};

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
