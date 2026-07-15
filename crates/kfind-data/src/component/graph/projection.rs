use unicode_normalization::UnicodeNormalization;

use crate::{
    DataError, DecodedMorphologyResource, MorphologyAnalysis, MorphologyExpressionAlignmentKind,
    align_morphology_expression,
};

use super::{
    MorphologyGraphAnalysis, MorphologyGraphExpressionKind, MorphologyGraphResource, resource_error,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyGraphProjectionStats {
    pub surface_count: u32,
    pub analysis_count: u32,
    pub component_count: u32,
    pub matrix_cost_count: u64,
}

pub fn validate_morphology_graph_projection(
    source: &str,
    full: &DecodedMorphologyResource<'_>,
    graph: &MorphologyGraphResource,
) -> Result<MorphologyGraphProjectionStats, DataError> {
    validate_shared_sections(source, full, graph)?;
    let payload_bytes = &graph.bytes[graph.sections.payload.clone()];
    let string_bytes = &graph.bytes[graph.sections.strings.clone()];
    let mut analysis_count = 0_u32;
    let mut component_count = 0_u32;
    for group in 0..graph.stats.surface_count {
        let (surface, graph_analyses) = graph
            .payload
            .group(payload_bytes, group, string_bytes, &graph.strings)
            .ok_or_else(|| projection_error(source, "graph group is unavailable"))?;
        let full_analyses = exact_full_analyses(source, full, surface)?;
        if graph_analyses.len() != full_analyses.len() {
            return Err(projection_error(source, "analysis count mismatch"));
        }
        for (full_analysis, graph_analysis) in full_analyses.iter().zip(&graph_analyses) {
            validate_analysis(source, surface, *full_analysis, graph_analysis)?;
            analysis_count = analysis_count
                .checked_add(1)
                .ok_or_else(|| projection_error(source, "analysis count overflow"))?;
            component_count = component_count
                .checked_add(
                    u32::try_from(graph_analysis.components.len())
                        .map_err(|error| projection_error(source, &error.to_string()))?,
                )
                .ok_or_else(|| projection_error(source, "component count overflow"))?;
        }
    }
    if analysis_count != full.stats().analysis_count
        || analysis_count != graph.stats.analysis_count
        || component_count != graph.stats.component_count
    {
        return Err(projection_error(source, "projection totals mismatch"));
    }
    let matrix_cost_count = u64::from(graph.stats.right_contexts)
        .checked_mul(u64::from(graph.stats.left_contexts))
        .ok_or_else(|| projection_error(source, "matrix cost count overflow"))?;
    Ok(MorphologyGraphProjectionStats {
        surface_count: graph.stats.surface_count,
        analysis_count,
        component_count,
        matrix_cost_count,
    })
}

fn validate_shared_sections(
    source: &str,
    full: &DecodedMorphologyResource<'_>,
    graph: &MorphologyGraphResource,
) -> Result<(), DataError> {
    if full.stats().surface_count != graph.stats.surface_count
        || full.stats().right_contexts != graph.stats.right_contexts
        || full.stats().left_contexts != graph.stats.left_contexts
        || full.char_def() != graph.char_def()
        || full.unk_def() != graph.unk_def()
    {
        return Err(projection_error(
            source,
            "resource metadata projection mismatch",
        ));
    }
    for right_id in 0..graph.stats.right_contexts {
        for left_id in 0..graph.stats.left_contexts {
            if full.connection_cost(right_id, left_id) != graph.connection_cost(right_id, left_id) {
                return Err(projection_error(
                    source,
                    "connection matrix projection mismatch",
                ));
            }
        }
    }
    Ok(())
}

fn exact_full_analyses<'a>(
    source: &str,
    full: &'a DecodedMorphologyResource<'a>,
    surface: &str,
) -> Result<Vec<MorphologyAnalysis<'a>>, DataError> {
    let mut exact = None;
    full.common_prefixes(surface.as_bytes(), |length, analyses| {
        if length == surface.len() {
            exact = Some(analyses.to_vec());
        }
    });
    exact.ok_or_else(|| projection_error(source, "full resource surface is unavailable"))
}

fn validate_analysis(
    source: &str,
    surface: &str,
    full: MorphologyAnalysis<'_>,
    graph: &MorphologyGraphAnalysis<'_>,
) -> Result<(), DataError> {
    if graph.pos != full.pos
        || graph.left_id != full.left_id
        || graph.right_id != full.right_id
        || graph.word_cost != full.word_cost
        || graph.analysis_type != full.analysis_type
        || graph.start_pos != full.start_pos
        || graph.end_pos != full.end_pos
    {
        return Err(projection_error(
            source,
            "source analysis projection mismatch",
        ));
    }
    if matches!(full.expression, "" | "*") {
        if graph.expression_kind != MorphologyGraphExpressionKind::Absent
            || !graph.components.is_empty()
        {
            return Err(projection_error(
                source,
                "absent expression projection mismatch",
            ));
        }
        return Ok(());
    }
    let alignment = align_morphology_expression(surface, full.expression);
    let expression_kind = match alignment.kind {
        MorphologyExpressionAlignmentKind::SpanAligned => {
            MorphologyGraphExpressionKind::SpanAligned
        }
        MorphologyExpressionAlignmentKind::Fused => MorphologyGraphExpressionKind::Fused,
        MorphologyExpressionAlignmentKind::Unaligned => MorphologyGraphExpressionKind::Unaligned,
        MorphologyExpressionAlignmentKind::Invalid => MorphologyGraphExpressionKind::Invalid,
    };
    if graph.expression_kind != expression_kind
        || graph.components.len() != alignment.components.len()
        || graph
            .components
            .iter()
            .zip(alignment.components)
            .any(|(graph, full)| {
                graph.surface != full.surface.nfc().collect::<String>()
                    || graph.pos != full.pos
                    || graph.span != full.span
            })
    {
        return Err(projection_error(
            source,
            "expression relation projection mismatch",
        ));
    }
    Ok(())
}

fn projection_error(source: &str, message: &str) -> DataError {
    resource_error(source, &format!("morphology graph projection: {message}"))
}
