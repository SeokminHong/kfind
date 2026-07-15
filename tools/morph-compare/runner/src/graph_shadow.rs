use kfind_data::MorphologyGraphExpressionKind;
use kfind_matcher::MorphMatcher;
use kfind_morph::{
    BoundedTokenContext, CandidateSpans, ConstraintEvidenceKind, ConstraintNodeSource,
    ConstraintOutcome, ConstraintProof, ConstraintResolution, ConstraintResolver,
    DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT, ProductPolicy, QueryMorphPattern,
};
use serde::Serialize;

use super::Span;

#[derive(Debug, Serialize)]
pub(super) struct GraphShadowEvidence {
    atom_index: usize,
    branch_index: usize,
    core: Span,
    anchor: Span,
    consumed: Span,
    product_accepted: bool,
    boundary_accepted: bool,
    status: &'static str,
    window: Option<GraphWindowEvidence>,
    resolution: Option<GraphResolutionEvidence>,
    patterns: Vec<GraphPatternEvidence>,
    whole: GraphPolicyEvidence,
    explicit_component: GraphPolicyEvidence,
    possible_analysis: GraphPolicyEvidence,
    unambiguous_analysis: GraphPolicyEvidence,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphWindowEvidence {
    raw: Span,
    normalized: String,
    core: Span,
    anchor: Span,
    consumed: Span,
    token: Span,
}

#[derive(Debug, Serialize)]
struct GraphResolutionEvidence {
    outcome: String,
    supported: Vec<GraphSupportedAnalysisEvidence>,
    proof: GraphProofEvidence,
}

#[derive(Debug, Serialize)]
struct GraphSupportedAnalysisEvidence {
    pattern_index: usize,
    path_index: usize,
    node_index: usize,
    source_node_index: usize,
    lexical_node_indices: Vec<usize>,
    lexical_source_node_indices: Vec<usize>,
    component_index: Option<usize>,
    evidence: &'static str,
    span_relation: String,
    support_span: Span,
    continuation: String,
    continuation_units: Vec<GraphMorphUnitEvidence>,
    context: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphMorphUnitEvidence {
    pos: String,
    span: Option<Span>,
}

#[derive(Debug, Serialize)]
struct GraphPatternEvidence {
    fine_pos: &'static str,
    lexical_form: String,
    token_relation: String,
    continuation: String,
    component_capability: String,
    adjacent: Vec<String>,
    outcome: String,
    whole_accepted: bool,
    explicit_component_accepted: bool,
    possible_analysis_accepted: bool,
    unambiguous_analysis_accepted: bool,
    supported: Vec<GraphSupportedAnalysisEvidence>,
    proof: GraphProofEvidence,
}

#[derive(Debug, Serialize)]
struct GraphPolicyEvidence {
    accepted: bool,
    outcomes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GraphProofEvidence {
    known_node_count: usize,
    unknown_node_count: usize,
    nodes: Vec<GraphNodeEvidence>,
    paths: Vec<GraphPathEvidence>,
}

#[derive(Debug, Serialize)]
struct GraphPathEvidence {
    evidence: &'static str,
    node_indices: Vec<usize>,
}

#[derive(Debug, Serialize)]
struct GraphNodeEvidence {
    surface: String,
    span: Span,
    pos: String,
    start_pos: String,
    end_pos: String,
    source: &'static str,
    expression_kind: Option<&'static str>,
    components: Vec<GraphComponentEvidence>,
    matches_query_node: bool,
    matches_source_component: bool,
    has_opaque_expression: bool,
}

#[derive(Debug, Serialize)]
struct GraphComponentEvidence {
    surface: String,
    pos: String,
    span: Option<Span>,
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
                core: span(candidate.core.clone()),
                anchor: span(candidate.anchor.clone()),
                consumed: span(candidate.consumed.clone()),
                product_accepted: candidate.product_accepted,
                boundary_accepted: candidate.boundary_accepted,
                status,
                window: None,
                resolution: None,
                patterns: Vec::new(),
                whole: unavailable_policy(),
                explicit_component: unavailable_policy(),
                possible_analysis: unavailable_policy(),
                unambiguous_analysis: unavailable_policy(),
                error,
            };
            let Some(resolver) = resolver else {
                return base(unavailable_status.unwrap_or("resource-unavailable"), None);
            };
            let window = match &candidate.window {
                Ok(window) => window,
                Err(error) => return base("window-unavailable", Some(error.to_string())),
            };
            let Some(core) = window.normalized_span(candidate.core.clone()) else {
                return base(
                    "core-unavailable",
                    Some("candidate core does not map to stable NFC boundaries".to_owned()),
                );
            };
            let Some(anchor) = window.normalized_span(candidate.anchor.clone()) else {
                return base(
                    "anchor-unavailable",
                    Some("candidate anchor does not map to stable NFC boundaries".to_owned()),
                );
            };
            let Some(consumed) = window.normalized_span(candidate.consumed.clone()) else {
                return base(
                    "consumed-unavailable",
                    Some(
                        "candidate consumed span does not map to stable NFC boundaries".to_owned(),
                    ),
                );
            };
            let token = 0..window.normalized().len();
            let patterns = candidate
                .patterns
                .iter()
                .map(|pattern| {
                    resolve_pattern(
                        resolver,
                        window.normalized(),
                        CandidateSpans {
                            core: core.clone(),
                            anchor: anchor.clone(),
                            consumed: consumed.clone(),
                            token: token.clone(),
                        },
                        pattern,
                    )
                })
                .collect::<Vec<_>>();
            let resolution = resolver.resolve_candidate(
                BoundedTokenContext::current(window.normalized()),
                CandidateSpans {
                    core: core.clone(),
                    anchor: anchor.clone(),
                    consumed: consumed.clone(),
                    token: token.clone(),
                },
                &candidate.patterns,
                DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
            );
            let whole = policy_evidence(ProductPolicy::Whole, &resolution, &candidate.patterns);
            let explicit_component = policy_evidence(
                ProductPolicy::ExplicitComponent,
                &resolution,
                &candidate.patterns,
            );
            let possible_analysis = policy_evidence(
                ProductPolicy::PossibleAnalysis,
                &resolution,
                &candidate.patterns,
            );
            let unambiguous_analysis = policy_evidence(
                ProductPolicy::UnambiguousAnalysis,
                &resolution,
                &candidate.patterns,
            );
            GraphShadowEvidence {
                atom_index: candidate.atom_index,
                branch_index: candidate.branch_index,
                core: span(candidate.core),
                anchor: span(candidate.anchor),
                consumed: span(candidate.consumed),
                product_accepted: candidate.product_accepted,
                boundary_accepted: candidate.boundary_accepted,
                status: "evaluated",
                window: Some(GraphWindowEvidence {
                    raw: span(window.raw_span()),
                    normalized: window.normalized().to_owned(),
                    core: span(core),
                    anchor: span(anchor),
                    consumed: span(consumed),
                    token: span(token),
                }),
                resolution: Some(resolution_evidence(&resolution)),
                whole,
                explicit_component,
                possible_analysis,
                unambiguous_analysis,
                patterns,
                error: None,
            }
        })
        .collect()
}

fn resolve_pattern(
    resolver: &ConstraintResolver,
    text: &str,
    spans: CandidateSpans,
    pattern: &QueryMorphPattern,
) -> GraphPatternEvidence {
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        spans,
        std::slice::from_ref(pattern),
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
        outcome: outcome_name(resolution.outcome),
        whole_accepted: ProductPolicy::Whole.accepts(&resolution, std::slice::from_ref(pattern)),
        explicit_component_accepted: ProductPolicy::ExplicitComponent
            .accepts(&resolution, std::slice::from_ref(pattern)),
        possible_analysis_accepted: ProductPolicy::PossibleAnalysis
            .accepts(&resolution, std::slice::from_ref(pattern)),
        unambiguous_analysis_accepted: ProductPolicy::UnambiguousAnalysis
            .accepts(&resolution, std::slice::from_ref(pattern)),
        supported: supported_evidence(&resolution),
        proof: proof_evidence(&resolution.proof),
    }
}

fn policy_evidence(
    policy: ProductPolicy,
    resolution: &ConstraintResolution,
    patterns: &[QueryMorphPattern],
) -> GraphPolicyEvidence {
    GraphPolicyEvidence {
        accepted: policy.accepts(resolution, patterns),
        outcomes: vec![outcome_name(resolution.outcome)],
    }
}

fn unavailable_policy() -> GraphPolicyEvidence {
    GraphPolicyEvidence {
        accepted: false,
        outcomes: Vec::new(),
    }
}

fn resolution_evidence(resolution: &ConstraintResolution) -> GraphResolutionEvidence {
    GraphResolutionEvidence {
        outcome: outcome_name(resolution.outcome),
        supported: supported_evidence(resolution),
        proof: proof_evidence(&resolution.proof),
    }
}

fn supported_evidence(resolution: &ConstraintResolution) -> Vec<GraphSupportedAnalysisEvidence> {
    resolution
        .supported
        .analyses
        .iter()
        .map(|analysis| GraphSupportedAnalysisEvidence {
            pattern_index: analysis.pattern_index,
            path_index: analysis.path_index,
            node_index: analysis.node_index,
            source_node_index: analysis.source_node_index,
            lexical_node_indices: analysis.lexical_node_indices.clone(),
            lexical_source_node_indices: analysis.lexical_source_node_indices.clone(),
            component_index: analysis.component_index,
            evidence: evidence_name(analysis.evidence),
            span_relation: format!("{:?}", analysis.span_relation),
            support_span: span(analysis.support_span.clone()),
            continuation: format!("{:?}", analysis.continuation.contract),
            continuation_units: analysis
                .continuation
                .units
                .iter()
                .map(|unit| GraphMorphUnitEvidence {
                    pos: unit.pos.clone(),
                    span: unit.span.clone().map(span),
                })
                .collect(),
            context: analysis
                .context
                .as_ref()
                .map(|context| format!("{context:?}")),
        })
        .collect()
}

fn proof_evidence(proof: &ConstraintProof) -> GraphProofEvidence {
    GraphProofEvidence {
        known_node_count: proof.known_node_count,
        unknown_node_count: proof.unknown_node_count,
        nodes: proof
            .nodes
            .iter()
            .map(|node| GraphNodeEvidence {
                surface: node.surface.clone(),
                span: span(node.span.clone()),
                pos: node.pos.clone(),
                start_pos: node.start_pos.clone(),
                end_pos: node.end_pos.clone(),
                source: match node.source {
                    ConstraintNodeSource::Source => "source",
                    ConstraintNodeSource::Unknown => "unknown",
                },
                expression_kind: node.expression_kind.map(expression_kind_name),
                components: node
                    .components
                    .iter()
                    .map(|component| GraphComponentEvidence {
                        surface: component.surface.clone(),
                        pos: component.pos.clone(),
                        span: component.span.clone().map(span),
                    })
                    .collect(),
                matches_query_node: node.matches_query_node,
                matches_source_component: node.matches_source_component,
                has_opaque_expression: node.has_opaque_expression,
            })
            .collect(),
        paths: proof
            .paths
            .iter()
            .map(|path| GraphPathEvidence {
                evidence: evidence_name(path.evidence),
                node_indices: path.node_indices.clone(),
            })
            .collect(),
    }
}

pub(super) fn outcome_name(outcome: ConstraintOutcome) -> String {
    match outcome {
        ConstraintOutcome::Supported => "supported".to_owned(),
        ConstraintOutcome::Contradicted => "contradicted".to_owned(),
        ConstraintOutcome::Ambiguous(reason) => format!("ambiguous:{reason:?}"),
        ConstraintOutcome::Unavailable(reason) => format!("unavailable:{reason:?}"),
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
