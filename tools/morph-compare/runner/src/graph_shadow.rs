use kfind_data::MorphologyGraphExpressionKind;
use kfind_matcher::MorphMatcher;
use kfind_morph::{
    CompoundExposureProfile, ConstraintEvidenceKind, ConstraintNodeSource, ConstraintProof,
    ConstraintResolver, ConstraintVerdict, DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT, QueryMorphPattern,
};
use serde::Serialize;

use super::Span;

#[derive(Debug, Serialize)]
pub(super) struct GraphShadowEvidence {
    atom_index: usize,
    branch_index: usize,
    target: Span,
    token: Span,
    product_accepted: bool,
    boundary_accepted: bool,
    status: &'static str,
    window: Option<GraphWindowEvidence>,
    resolution: Option<GraphResolutionEvidence>,
    patterns: Vec<GraphPatternEvidence>,
    opaque: GraphProfileEvidence,
    transparent: GraphProfileEvidence,
    explicit: GraphProfileEvidence,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphWindowEvidence {
    raw: Span,
    normalized: String,
    target: Span,
    candidate: Span,
}

#[derive(Debug, Serialize)]
struct GraphResolutionEvidence {
    verdict: String,
    proof: GraphProofEvidence,
}

#[derive(Debug, Serialize)]
struct GraphPatternEvidence {
    fine_pos: &'static str,
    lexical_form: String,
    token_relation: String,
    continuation: String,
    component_capability: String,
    adjacent: Vec<String>,
    verdict: String,
    opaque_verdict: String,
    transparent_verdict: String,
    explicit_verdict: String,
    proof: GraphProofEvidence,
}

#[derive(Debug, Serialize)]
struct GraphProfileEvidence {
    accepted: bool,
    verdicts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GraphProofEvidence {
    known_node_count: usize,
    unknown_node_count: usize,
    paths: Vec<GraphPathEvidence>,
}

#[derive(Debug, Serialize)]
struct GraphPathEvidence {
    evidence: &'static str,
    nodes: Vec<GraphNodeEvidence>,
}

#[derive(Debug, Serialize)]
struct GraphNodeEvidence {
    span: Span,
    pos: String,
    start_pos: String,
    end_pos: String,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    source: &'static str,
    analysis_type: Option<String>,
    expression_kind: Option<&'static str>,
    matches_query_node: bool,
    matches_source_component: bool,
    has_opaque_expression: bool,
}

pub(super) fn diagnose_graph_shadow(
    matcher: &MorphMatcher,
    text: &[u8],
    resolver: Option<&ConstraintResolver>,
    unavailable_status: Option<&'static str>,
) -> Vec<GraphShadowEvidence> {
    matcher
        .analysis_graph_candidates(text)
        .into_iter()
        .map(|candidate| {
            let base = |status, error| GraphShadowEvidence {
                atom_index: candidate.atom_index,
                branch_index: candidate.branch_index,
                target: span(candidate.target.clone()),
                token: span(candidate.token.clone()),
                product_accepted: candidate.product_accepted,
                boundary_accepted: candidate.boundary_accepted,
                status,
                window: None,
                resolution: None,
                patterns: Vec::new(),
                opaque: unavailable_profile(),
                transparent: unavailable_profile(),
                explicit: unavailable_profile(),
                error,
            };
            let Some(resolver) = resolver else {
                return base(unavailable_status.unwrap_or("resource-unavailable"), None);
            };
            let window = match &candidate.window {
                Ok(window) => window,
                Err(error) => return base("window-unavailable", Some(error.to_string())),
            };
            let Some(target) = window.normalized_span(candidate.target.clone()) else {
                return base(
                    "target-unavailable",
                    Some("candidate span does not map to stable NFC boundaries".to_owned()),
                );
            };
            let Some(candidate_token) = window.normalized_span(candidate.token.clone()) else {
                return base(
                    "candidate-unavailable",
                    Some("candidate token span does not map to stable NFC boundaries".to_owned()),
                );
            };
            let patterns = candidate
                .patterns
                .iter()
                .map(|pattern| {
                    resolve_pattern(
                        resolver,
                        window.normalized(),
                        target.clone(),
                        candidate_token.clone(),
                        pattern,
                    )
                })
                .collect::<Vec<_>>();
            let resolution = resolver.resolve_patterns(
                window.normalized(),
                target.clone(),
                candidate_token.clone(),
                &candidate.patterns,
                DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
            );
            let opaque = profile_evidence(
                resolution.verdict_for(CompoundExposureProfile::Opaque, &candidate.patterns),
            );
            let transparent = profile_evidence(
                resolution.verdict_for(CompoundExposureProfile::Transparent, &candidate.patterns),
            );
            let explicit = profile_evidence(
                resolution.verdict_for(CompoundExposureProfile::Explicit, &candidate.patterns),
            );
            GraphShadowEvidence {
                atom_index: candidate.atom_index,
                branch_index: candidate.branch_index,
                target: span(candidate.target),
                token: span(candidate.token),
                product_accepted: candidate.product_accepted,
                boundary_accepted: candidate.boundary_accepted,
                status: "evaluated",
                window: Some(GraphWindowEvidence {
                    raw: span(window.raw_span()),
                    normalized: window.normalized().to_owned(),
                    target: span(target),
                    candidate: span(candidate_token),
                }),
                resolution: Some(GraphResolutionEvidence {
                    verdict: verdict_name(resolution.verdict),
                    proof: proof_evidence(resolution.proof),
                }),
                opaque,
                transparent,
                explicit,
                patterns,
                error: None,
            }
        })
        .collect()
}

fn resolve_pattern(
    resolver: &ConstraintResolver,
    text: &str,
    target: std::ops::Range<usize>,
    candidate: std::ops::Range<usize>,
    pattern: &QueryMorphPattern,
) -> GraphPatternEvidence {
    let resolution = resolver.resolve(
        text,
        target,
        candidate,
        pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    GraphPatternEvidence {
        fine_pos: pattern.fine_pos.as_str(),
        lexical_form: pattern.lexical_form.to_string(),
        token_relation: format!("{:?}", pattern.token_relation),
        continuation: format!("{:?}", pattern.continuation),
        component_capability: format!("{:?}", pattern.component_capability),
        adjacent: pattern
            .adjacent
            .iter()
            .map(|constraint| format!("{constraint:?}"))
            .collect(),
        verdict: verdict_name(resolution.verdict),
        opaque_verdict: verdict_name(resolution.verdict_for(
            CompoundExposureProfile::Opaque,
            std::slice::from_ref(pattern),
        )),
        transparent_verdict: verdict_name(resolution.verdict_for(
            CompoundExposureProfile::Transparent,
            std::slice::from_ref(pattern),
        )),
        explicit_verdict: verdict_name(resolution.verdict_for(
            CompoundExposureProfile::Explicit,
            std::slice::from_ref(pattern),
        )),
        proof: proof_evidence(resolution.proof),
    }
}

fn profile_evidence(verdict: ConstraintVerdict) -> GraphProfileEvidence {
    let verdict = verdict_name(verdict);
    GraphProfileEvidence {
        accepted: verdict == "proven",
        verdicts: vec![verdict],
    }
}

fn unavailable_profile() -> GraphProfileEvidence {
    GraphProfileEvidence {
        accepted: false,
        verdicts: Vec::new(),
    }
}

fn proof_evidence(proof: ConstraintProof) -> GraphProofEvidence {
    GraphProofEvidence {
        known_node_count: proof.known_node_count,
        unknown_node_count: proof.unknown_node_count,
        paths: proof
            .paths
            .into_iter()
            .map(|path| GraphPathEvidence {
                evidence: evidence_name(path.evidence),
                nodes: path
                    .nodes
                    .into_iter()
                    .map(|node| GraphNodeEvidence {
                        span: span(node.span),
                        pos: node.pos,
                        start_pos: node.start_pos,
                        end_pos: node.end_pos,
                        left_id: node.left_id,
                        right_id: node.right_id,
                        word_cost: node.word_cost,
                        source: match node.source {
                            ConstraintNodeSource::Source => "source",
                            ConstraintNodeSource::Unknown => "unknown",
                        },
                        analysis_type: node.analysis_type,
                        expression_kind: node.expression_kind.map(expression_kind_name),
                        matches_query_node: node.matches_query_node,
                        matches_source_component: node.matches_source_component,
                        has_opaque_expression: node.has_opaque_expression,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn verdict_name(verdict: ConstraintVerdict) -> String {
    match verdict {
        ConstraintVerdict::Proven => "proven".to_owned(),
        ConstraintVerdict::Contradicted => "contradicted".to_owned(),
        ConstraintVerdict::Ambiguous(reason) => format!("ambiguous:{reason:?}"),
        ConstraintVerdict::Unavailable(reason) => format!("unavailable:{reason:?}"),
    }
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

const fn expression_kind_name(kind: MorphologyGraphExpressionKind) -> &'static str {
    match kind {
        MorphologyGraphExpressionKind::Absent => "absent",
        MorphologyGraphExpressionKind::SpanAligned => "span-aligned",
        MorphologyGraphExpressionKind::Fused => "fused",
        MorphologyGraphExpressionKind::Unaligned => "unaligned",
        MorphologyGraphExpressionKind::Invalid => "invalid",
    }
}

fn span(range: std::ops::Range<usize>) -> Span {
    Span {
        byte_start: range.start,
        byte_end: range.end,
    }
}
