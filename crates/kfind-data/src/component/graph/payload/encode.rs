use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use unicode_normalization::UnicodeNormalization;

use crate::component::{build_conversion_error, build_error};
use crate::{
    DataError, MecabSourceMorphologyEntry, MorphologyExpressionAlignmentKind,
    align_morphology_expression, morphology_pos_transitions,
};

use super::super::MorphologyGraphExpressionKind;
use super::NO_SPAN;

pub(in crate::component::graph) struct EncodedGraphPayload {
    pub bytes: Vec<u8>,
    pub strings: Vec<u8>,
    pub analysis_count: u32,
}

#[derive(Debug)]
struct PreparedAnalysis<'a> {
    source: &'a MecabSourceMorphologyEntry,
    expression_kind: MorphologyGraphExpressionKind,
    components: Vec<PreparedComponent>,
}

#[derive(Debug)]
struct PreparedComponent {
    surface: String,
    pos: String,
    span: Option<Range<usize>>,
}

pub(in crate::component::graph) fn encode_graph_payload(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
) -> Result<EncodedGraphPayload, DataError> {
    let prepared = groups
        .iter()
        .map(|(_, analyses)| analyses.iter().map(prepare_analysis).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let analysis_count = u32::try_from(prepared.iter().map(Vec::len).sum::<usize>())
        .map_err(build_conversion_error)?;
    let component_count = u32::try_from(
        prepared
            .iter()
            .flatten()
            .map(|analysis| analysis.components.len())
            .sum::<usize>(),
    )
    .map_err(build_conversion_error)?;
    let transitions = collect_transitions(&prepared);
    let transition_count = u32::try_from(transitions.len()).map_err(build_conversion_error)?;
    let (strings, string_ids) = encode_strings(groups, &prepared, &transitions)?;
    let mut pos_counts = BTreeMap::<u32, u32>::new();
    for analysis in prepared.iter().flatten() {
        let pos = string_id(&string_ids, &analysis.source.pos)?;
        let count = pos_counts.entry(pos).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| build_error("graph POS count overflow"))?;
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u32::try_from(groups.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&analysis_count.to_le_bytes());
    bytes.extend_from_slice(&component_count.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(pos_counts.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&transition_count.to_le_bytes());
    for (pos, count) in &pos_counts {
        bytes.extend_from_slice(&pos.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    for (surface, _) in groups {
        bytes.extend_from_slice(&string_id(&string_ids, surface)?.to_le_bytes());
    }
    let mut analysis_offset = 0_u32;
    bytes.extend_from_slice(&analysis_offset.to_le_bytes());
    for analyses in &prepared {
        analysis_offset = analysis_offset
            .checked_add(u32::try_from(analyses.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("graph analysis offset overflow"))?;
        bytes.extend_from_slice(&analysis_offset.to_le_bytes());
    }
    let mut component_offset = 0_u32;
    for analysis in prepared.iter().flatten() {
        let source = analysis.source;
        bytes.extend_from_slice(&source.left_id.to_le_bytes());
        bytes.extend_from_slice(&source.right_id.to_le_bytes());
        bytes.extend_from_slice(&source.word_cost.to_le_bytes());
        for value in [
            &source.pos,
            &source.analysis_type,
            &source.start_pos,
            &source.end_pos,
        ] {
            bytes.extend_from_slice(&string_id(&string_ids, value)?.to_le_bytes());
        }
        bytes.push(analysis.expression_kind.encode());
        bytes.extend_from_slice(&[0; 3]);
        bytes.extend_from_slice(&component_offset.to_le_bytes());
        let count = u32::try_from(analysis.components.len()).map_err(build_conversion_error)?;
        bytes.extend_from_slice(&count.to_le_bytes());
        component_offset = component_offset
            .checked_add(count)
            .ok_or_else(|| build_error("graph component offset overflow"))?;
    }
    for component in prepared
        .iter()
        .flatten()
        .flat_map(|analysis| &analysis.components)
    {
        bytes.extend_from_slice(&string_id(&string_ids, &component.surface)?.to_le_bytes());
        bytes.extend_from_slice(&string_id(&string_ids, &component.pos)?.to_le_bytes());
        match &component.span {
            Some(span) => {
                bytes.extend_from_slice(
                    &u32::try_from(span.start)
                        .map_err(build_conversion_error)?
                        .to_le_bytes(),
                );
                bytes.extend_from_slice(
                    &u32::try_from(span.end)
                        .map_err(build_conversion_error)?
                        .to_le_bytes(),
                );
            }
            None => {
                bytes.extend_from_slice(&NO_SPAN.to_le_bytes());
                bytes.extend_from_slice(&NO_SPAN.to_le_bytes());
            }
        }
    }
    for (end_pos, start_pos) in &transitions {
        bytes.extend_from_slice(&string_id(&string_ids, end_pos)?.to_le_bytes());
        bytes.extend_from_slice(&string_id(&string_ids, start_pos)?.to_le_bytes());
    }
    Ok(EncodedGraphPayload {
        bytes,
        strings,
        analysis_count,
    })
}

fn prepare_analysis(entry: &MecabSourceMorphologyEntry) -> PreparedAnalysis<'_> {
    if matches!(entry.expression.as_str(), "" | "*") {
        return PreparedAnalysis {
            source: entry,
            expression_kind: MorphologyGraphExpressionKind::Absent,
            components: Vec::new(),
        };
    }
    let alignment = align_morphology_expression(&entry.surface, &entry.expression);
    let expression_kind = match alignment.kind {
        MorphologyExpressionAlignmentKind::SpanAligned => {
            MorphologyGraphExpressionKind::SpanAligned
        }
        MorphologyExpressionAlignmentKind::Fused => MorphologyGraphExpressionKind::Fused,
        MorphologyExpressionAlignmentKind::Unaligned => MorphologyGraphExpressionKind::Unaligned,
        MorphologyExpressionAlignmentKind::Invalid => MorphologyGraphExpressionKind::Invalid,
    };
    let components = alignment
        .components
        .into_iter()
        .map(|component| PreparedComponent {
            surface: component.surface.nfc().collect(),
            pos: component.pos.to_owned(),
            span: component.span,
        })
        .collect();
    PreparedAnalysis {
        source: entry,
        expression_kind,
        components,
    }
}

fn encode_strings(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
    prepared: &[Vec<PreparedAnalysis<'_>>],
    transitions: &BTreeSet<(String, String)>,
) -> Result<(Vec<u8>, BTreeMap<String, u32>), DataError> {
    let mut unique = BTreeSet::new();
    for (surface, _) in groups {
        unique.insert(surface.clone());
    }
    for analysis in prepared.iter().flatten() {
        let source = analysis.source;
        unique.extend([
            source.pos.clone(),
            source.analysis_type.clone(),
            source.start_pos.clone(),
            source.end_pos.clone(),
        ]);
        for component in &analysis.components {
            unique.insert(component.surface.clone());
            unique.insert(component.pos.clone());
        }
    }
    for (end_pos, start_pos) in transitions {
        unique.insert(end_pos.clone());
        unique.insert(start_pos.clone());
    }
    let ids = unique
        .into_iter()
        .enumerate()
        .map(|(index, value)| Ok((value, u32::try_from(index).map_err(build_conversion_error)?)))
        .collect::<Result<BTreeMap<_, _>, DataError>>()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u32::try_from(ids.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    let mut offset = 0_u32;
    bytes.extend_from_slice(&offset.to_le_bytes());
    for value in ids.keys() {
        offset = offset
            .checked_add(u32::try_from(value.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("graph string table offset overflow"))?;
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    for value in ids.keys() {
        bytes.extend_from_slice(value.as_bytes());
    }
    Ok((bytes, ids))
}

fn collect_transitions(prepared: &[Vec<PreparedAnalysis<'_>>]) -> BTreeSet<(String, String)> {
    let mut transitions = BTreeSet::new();
    for analysis in prepared.iter().flatten() {
        transitions.extend(morphology_pos_transitions(
            &analysis.source.pos,
            &analysis.source.expression,
        ));
    }
    transitions
}

fn string_id(ids: &BTreeMap<String, u32>, value: &str) -> Result<u32, DataError> {
    ids.get(value)
        .copied()
        .ok_or_else(|| build_error("graph metadata string is not interned"))
}
