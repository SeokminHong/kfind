use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use unicode_normalization::UnicodeNormalization;

use crate::DataError;
use crate::component::{StringLayout, resource_error};

use super::super::{
    MorphologyGraphAnalysis, MorphologyGraphComponent, MorphologyGraphExpressionKind,
};
use super::{ANALYSIS_BYTES, COMPONENT_BYTES, NO_SPAN, PAYLOAD_HEADER_BYTES, TRANSITION_BYTES};

#[derive(Debug)]
pub(in crate::component::graph) struct GraphPayloadStats {
    pub component_count: u32,
    pub pos_counts: BTreeMap<String, u32>,
    pub expression_counts: BTreeMap<MorphologyGraphExpressionKind, u32>,
    pub transitions: BTreeSet<(String, String)>,
}

#[derive(Clone, Debug)]
pub(in crate::component::graph) struct GraphPayloadLayout {
    surface_count: u32,
    analysis_count: u32,
    component_count: u32,
    transition_count: u32,
    surface_ids_start: usize,
    analysis_offsets_start: usize,
    analyses_start: usize,
    components_start: usize,
    transitions_start: usize,
}

impl GraphPayloadLayout {
    #[allow(clippy::too_many_arguments)]
    pub fn parse(
        source: &str,
        input: &[u8],
        surface_count: u32,
        analysis_count: u32,
        right_contexts: u16,
        left_contexts: u16,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<(Self, GraphPayloadStats), DataError> {
        if read_u32_at(input, 0) != Some(surface_count)
            || read_u32_at(input, 4) != Some(analysis_count)
        {
            return Err(resource_error(source, "graph payload counts mismatch"));
        }
        let component_count = read_u32_at(input, 8)
            .ok_or_else(|| resource_error(source, "truncated graph component count"))?;
        let pos_count = read_u32_at(input, 12)
            .ok_or_else(|| resource_error(source, "truncated graph POS count"))?;
        let transition_count = read_u32_at(input, 16)
            .ok_or_else(|| resource_error(source, "truncated graph transition count"))?;
        let surface_ids_start = checked_add(
            source,
            PAYLOAD_HEADER_BYTES,
            checked_mul(source, to_usize(source, pos_count)?, 8, "POS counts")?,
            "POS counts",
        )?;
        let analysis_offsets_start = checked_add(
            source,
            surface_ids_start,
            checked_mul(source, to_usize(source, surface_count)?, 4, "surface IDs")?,
            "surface IDs",
        )?;
        let offset_count = checked_add(
            source,
            to_usize(source, surface_count)?,
            1,
            "analysis offsets",
        )?;
        let analyses_start = checked_add(
            source,
            analysis_offsets_start,
            checked_mul(source, offset_count, 4, "analysis offsets")?,
            "analysis offsets",
        )?;
        let components_start = checked_add(
            source,
            analyses_start,
            checked_mul(
                source,
                to_usize(source, analysis_count)?,
                ANALYSIS_BYTES,
                "analysis records",
            )?,
            "analysis records",
        )?;
        let transitions_start = checked_add(
            source,
            components_start,
            checked_mul(
                source,
                to_usize(source, component_count)?,
                COMPONENT_BYTES,
                "component records",
            )?,
            "component records",
        )?;
        let expected_len = checked_add(
            source,
            transitions_start,
            checked_mul(
                source,
                to_usize(source, transition_count)?,
                TRANSITION_BYTES,
                "transition records",
            )?,
            "transition records",
        )?;
        if input.len() != expected_len {
            return Err(resource_error(source, "graph payload length mismatch"));
        }
        let layout = Self {
            surface_count,
            analysis_count,
            component_count,
            transition_count,
            surface_ids_start,
            analysis_offsets_start,
            analyses_start,
            components_start,
            transitions_start,
        };
        layout.validate_surfaces(source, input, string_bytes, strings)?;
        layout.validate_analysis_offsets(source, input)?;
        let expected_pos_counts =
            layout.pos_counts(source, input, pos_count, string_bytes, strings)?;
        let (actual_pos_counts, expression_counts) = layout.validate_analyses(
            source,
            input,
            right_contexts,
            left_contexts,
            string_bytes,
            strings,
        )?;
        if actual_pos_counts != expected_pos_counts {
            return Err(resource_error(
                source,
                "graph POS counts do not match analysis records",
            ));
        }
        let transitions = layout.validate_transitions(source, input, string_bytes, strings)?;
        Ok((
            layout,
            GraphPayloadStats {
                component_count,
                pos_counts: expected_pos_counts,
                expression_counts,
                transitions,
            },
        ))
    }

    pub fn group<'a>(
        &self,
        input: &[u8],
        group: u32,
        string_bytes: &'a [u8],
        strings: &StringLayout,
    ) -> Option<(&'a str, Vec<MorphologyGraphAnalysis<'a>>)> {
        let surface = self.surface(input, group, string_bytes, strings)?;
        let range = self.analysis_range(input, group)?;
        let analyses = range
            .map(|analysis| self.analysis(input, analysis, string_bytes, strings))
            .collect::<Option<Vec<_>>>()?;
        Some((surface, analyses))
    }

    pub fn surface<'a>(
        &self,
        input: &[u8],
        group: u32,
        string_bytes: &'a [u8],
        strings: &StringLayout,
    ) -> Option<&'a str> {
        if group >= self.surface_count {
            return None;
        }
        let id = read_u32_at(
            input,
            self.surface_ids_start
                .checked_add(usize::try_from(group).ok()?.checked_mul(4)?)?,
        )?;
        strings.get(string_bytes, id)
    }

    fn validate_surfaces(
        &self,
        source: &str,
        input: &[u8],
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<(), DataError> {
        let mut previous = None;
        for group in 0..self.surface_count {
            let surface = self
                .surface(input, group, string_bytes, strings)
                .ok_or_else(|| resource_error(source, "invalid graph surface string ID"))?;
            if surface.is_empty() || surface.nfc().ne(surface.chars()) {
                return Err(resource_error(source, "graph surface is not non-empty NFC"));
            }
            if previous.is_some_and(|previous| previous >= surface) {
                return Err(resource_error(
                    source,
                    "graph surfaces are not strictly ordered",
                ));
            }
            previous = Some(surface);
        }
        Ok(())
    }

    fn validate_analysis_offsets(&self, source: &str, input: &[u8]) -> Result<(), DataError> {
        let mut previous = 0;
        for group in 0..=self.surface_count {
            let offset = self
                .analysis_offset(input, group)
                .ok_or_else(|| resource_error(source, "truncated graph analysis offset"))?;
            if (group == 0 && offset != 0)
                || offset < previous
                || offset > self.analysis_count
                || (group > 0 && offset == previous)
            {
                return Err(resource_error(source, "invalid graph analysis offsets"));
            }
            previous = offset;
        }
        if previous != self.analysis_count {
            return Err(resource_error(
                source,
                "final graph analysis offset mismatch",
            ));
        }
        Ok(())
    }

    fn pos_counts(
        &self,
        source: &str,
        input: &[u8],
        pos_count: u32,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<BTreeMap<String, u32>, DataError> {
        let mut counts = BTreeMap::new();
        let mut previous_id = None;
        for index in 0..pos_count {
            let offset = PAYLOAD_HEADER_BYTES
                .checked_add(
                    usize::try_from(index)
                        .ok()
                        .and_then(|index| index.checked_mul(8))
                        .ok_or_else(|| resource_error(source, "graph POS count offset overflow"))?,
                )
                .ok_or_else(|| resource_error(source, "graph POS count offset overflow"))?;
            let id = read_u32_at(input, offset)
                .ok_or_else(|| resource_error(source, "truncated graph POS ID"))?;
            let count = read_u32_at(input, offset + 4)
                .ok_or_else(|| resource_error(source, "truncated graph POS count"))?;
            if count == 0 || previous_id.is_some_and(|previous| previous >= id) {
                return Err(resource_error(source, "invalid graph POS counts"));
            }
            let pos = strings
                .get(string_bytes, id)
                .ok_or_else(|| resource_error(source, "invalid graph POS string ID"))?;
            counts.insert(pos.to_owned(), count);
            previous_id = Some(id);
        }
        Ok(counts)
    }

    fn validate_analyses(
        &self,
        source: &str,
        input: &[u8],
        right_contexts: u16,
        left_contexts: u16,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<ValidationCounts, DataError> {
        let mut pos_counts = BTreeMap::new();
        let mut expression_counts = BTreeMap::new();
        let mut expected_component_start = 0;
        for group in 0..self.surface_count {
            let surface = self
                .surface(input, group, string_bytes, strings)
                .ok_or_else(|| resource_error(source, "invalid graph surface"))?;
            let range = self
                .analysis_range(input, group)
                .ok_or_else(|| resource_error(source, "invalid graph analysis range"))?;
            for analysis_index in range {
                let record = self
                    .analysis_record(input, analysis_index)
                    .ok_or_else(|| resource_error(source, "invalid graph analysis record"))?;
                let left_id = read_u16_at(record, 0).expect("validated analysis width");
                let right_id = read_u16_at(record, 2).expect("validated analysis width");
                if left_id >= left_contexts || right_id >= right_contexts {
                    return Err(resource_error(
                        source,
                        "graph analysis context ID is out of range",
                    ));
                }
                for offset in [8, 12, 16, 20] {
                    let id = read_u32_at(record, offset)
                        .ok_or_else(|| resource_error(source, "truncated graph string ID"))?;
                    strings
                        .get(string_bytes, id)
                        .ok_or_else(|| resource_error(source, "invalid graph string ID"))?;
                }
                if record[25..28] != [0; 3] {
                    return Err(resource_error(
                        source,
                        "non-zero graph analysis reserved bytes",
                    ));
                }
                let expression_kind = MorphologyGraphExpressionKind::decode(record[24])
                    .ok_or_else(|| resource_error(source, "invalid graph expression kind"))?;
                let component_start = read_u32_at(record, 28)
                    .ok_or_else(|| resource_error(source, "truncated graph component start"))?;
                let component_count = read_u32_at(record, 32)
                    .ok_or_else(|| resource_error(source, "truncated graph component count"))?;
                if component_start != expected_component_start {
                    return Err(resource_error(
                        source,
                        "non-contiguous graph component ranges",
                    ));
                }
                let component_end = component_start
                    .checked_add(component_count)
                    .ok_or_else(|| resource_error(source, "graph component range overflow"))?;
                if component_end > self.component_count {
                    return Err(resource_error(
                        source,
                        "graph component range out of bounds",
                    ));
                }
                self.validate_relation(
                    source,
                    input,
                    surface,
                    expression_kind,
                    component_start..component_end,
                    string_bytes,
                    strings,
                )?;
                expected_component_start = component_end;
                let pos_id = read_u32_at(record, 8).expect("validated analysis width");
                let pos = strings
                    .get(string_bytes, pos_id)
                    .expect("validated graph POS string ID");
                increment(source, &mut pos_counts, pos.to_owned(), "graph POS count")?;
                increment(
                    source,
                    &mut expression_counts,
                    expression_kind,
                    "graph expression count",
                )?;
            }
        }
        if expected_component_start != self.component_count {
            return Err(resource_error(source, "orphan graph component records"));
        }
        Ok((pos_counts, expression_counts))
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_relation(
        &self,
        source: &str,
        input: &[u8],
        surface: &str,
        kind: MorphologyGraphExpressionKind,
        range: Range<u32>,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<(), DataError> {
        let components = range
            .map(|index| {
                self.component(input, index, string_bytes, strings)
                    .ok_or_else(|| resource_error(source, "invalid graph component record"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        match kind {
            MorphologyGraphExpressionKind::Absent | MorphologyGraphExpressionKind::Invalid => {
                if !components.is_empty() {
                    return Err(resource_error(
                        source,
                        "component-free graph relation has components",
                    ));
                }
            }
            MorphologyGraphExpressionKind::SpanAligned => {
                validate_aligned_components(source, surface, &components)?;
            }
            MorphologyGraphExpressionKind::Fused => {
                validate_opaque_components(source, surface, &components, true)?;
            }
            MorphologyGraphExpressionKind::Unaligned => {
                validate_opaque_components(source, surface, &components, false)?;
            }
        }
        Ok(())
    }

    fn analysis<'a>(
        &self,
        input: &[u8],
        analysis: u32,
        string_bytes: &'a [u8],
        strings: &StringLayout,
    ) -> Option<MorphologyGraphAnalysis<'a>> {
        let record = self.analysis_record(input, analysis)?;
        let component_start = read_u32_at(record, 28)?;
        let component_end = component_start.checked_add(read_u32_at(record, 32)?)?;
        let components = (component_start..component_end)
            .map(|component| self.component(input, component, string_bytes, strings))
            .collect::<Option<Vec<_>>>()?;
        Some(MorphologyGraphAnalysis {
            pos: strings.get(string_bytes, read_u32_at(record, 8)?)?,
            left_id: read_u16_at(record, 0)?,
            right_id: read_u16_at(record, 2)?,
            word_cost: read_i32_at(record, 4)?,
            analysis_type: strings.get(string_bytes, read_u32_at(record, 12)?)?,
            start_pos: strings.get(string_bytes, read_u32_at(record, 16)?)?,
            end_pos: strings.get(string_bytes, read_u32_at(record, 20)?)?,
            expression_kind: MorphologyGraphExpressionKind::decode(record[24])?,
            components,
        })
    }

    fn component<'a>(
        &self,
        input: &[u8],
        component: u32,
        string_bytes: &'a [u8],
        strings: &StringLayout,
    ) -> Option<MorphologyGraphComponent<'a>> {
        let record = self.component_record(input, component)?;
        let start = read_u32_at(record, 8)?;
        let end = read_u32_at(record, 12)?;
        let span = match (start, end) {
            (NO_SPAN, NO_SPAN) => None,
            (NO_SPAN, _) | (_, NO_SPAN) => return None,
            (start, end) => Some(usize::try_from(start).ok()?..usize::try_from(end).ok()?),
        };
        Some(MorphologyGraphComponent {
            surface: strings.get(string_bytes, read_u32_at(record, 0)?)?,
            pos: strings.get(string_bytes, read_u32_at(record, 4)?)?,
            span,
        })
    }

    fn validate_transitions(
        &self,
        source: &str,
        input: &[u8],
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<BTreeSet<(String, String)>, DataError> {
        let mut transitions = BTreeSet::new();
        let mut previous_ids = None;
        for index in 0..self.transition_count {
            let record = self
                .transition_record(input, index)
                .ok_or_else(|| resource_error(source, "invalid graph transition record"))?;
            let end_id = read_u32_at(record, 0)
                .ok_or_else(|| resource_error(source, "truncated graph transition end POS"))?;
            let start_id = read_u32_at(record, 4)
                .ok_or_else(|| resource_error(source, "truncated graph transition start POS"))?;
            if previous_ids.is_some_and(|previous| previous >= (end_id, start_id)) {
                return Err(resource_error(
                    source,
                    "graph transitions are not strictly ordered",
                ));
            }
            let end_pos = strings
                .get(string_bytes, end_id)
                .filter(|pos| !pos.is_empty() && *pos != "*")
                .ok_or_else(|| resource_error(source, "invalid graph transition end POS"))?;
            let start_pos = strings
                .get(string_bytes, start_id)
                .filter(|pos| !pos.is_empty() && *pos != "*")
                .ok_or_else(|| resource_error(source, "invalid graph transition start POS"))?;
            transitions.insert((end_pos.to_owned(), start_pos.to_owned()));
            previous_ids = Some((end_id, start_id));
        }
        Ok(transitions)
    }

    fn analysis_range(&self, input: &[u8], group: u32) -> Option<Range<u32>> {
        if group >= self.surface_count {
            return None;
        }
        Some(
            self.analysis_offset(input, group)?
                ..self.analysis_offset(input, group.checked_add(1)?)?,
        )
    }

    fn analysis_offset(&self, input: &[u8], group: u32) -> Option<u32> {
        read_u32_at(
            input,
            self.analysis_offsets_start
                .checked_add(usize::try_from(group).ok()?.checked_mul(4)?)?,
        )
    }

    fn analysis_record<'a>(&self, input: &'a [u8], analysis: u32) -> Option<&'a [u8]> {
        if analysis >= self.analysis_count {
            return None;
        }
        let start = self.analyses_start.checked_add(
            usize::try_from(analysis)
                .ok()?
                .checked_mul(ANALYSIS_BYTES)?,
        )?;
        input.get(start..start.checked_add(ANALYSIS_BYTES)?)
    }

    fn component_record<'a>(&self, input: &'a [u8], component: u32) -> Option<&'a [u8]> {
        if component >= self.component_count {
            return None;
        }
        let start = self.components_start.checked_add(
            usize::try_from(component)
                .ok()?
                .checked_mul(COMPONENT_BYTES)?,
        )?;
        input.get(start..start.checked_add(COMPONENT_BYTES)?)
    }

    fn transition_record<'a>(&self, input: &'a [u8], transition: u32) -> Option<&'a [u8]> {
        if transition >= self.transition_count {
            return None;
        }
        let start = self.transitions_start.checked_add(
            usize::try_from(transition)
                .ok()?
                .checked_mul(TRANSITION_BYTES)?,
        )?;
        input.get(start..start.checked_add(TRANSITION_BYTES)?)
    }
}

type ValidationCounts = (
    BTreeMap<String, u32>,
    BTreeMap<MorphologyGraphExpressionKind, u32>,
);

fn validate_aligned_components(
    source: &str,
    surface: &str,
    components: &[MorphologyGraphComponent<'_>],
) -> Result<(), DataError> {
    if components.is_empty() || recompose(components) != surface {
        return Err(resource_error(
            source,
            "invalid span-aligned graph components",
        ));
    }
    let mut expected_start = 0;
    for component in components {
        let span = component
            .span
            .as_ref()
            .ok_or_else(|| resource_error(source, "span-aligned graph component has no span"))?;
        if component.surface.is_empty()
            || component.pos.is_empty()
            || span.start != expected_start
            || span.start >= span.end
            || span.end > surface.len()
            || !surface.is_char_boundary(span.start)
            || !surface.is_char_boundary(span.end)
        {
            return Err(resource_error(
                source,
                "invalid span-aligned graph component",
            ));
        }
        expected_start = span.end;
    }
    if expected_start != surface.len() {
        return Err(resource_error(
            source,
            "span-aligned graph components do not cover surface",
        ));
    }
    Ok(())
}

fn validate_opaque_components(
    source: &str,
    surface: &str,
    components: &[MorphologyGraphComponent<'_>],
    must_recompose: bool,
) -> Result<(), DataError> {
    if components.is_empty()
        || components.iter().any(|component| {
            component.surface.is_empty() || component.pos.is_empty() || component.span.is_some()
        })
        || (recompose(components) == surface) != must_recompose
    {
        return Err(resource_error(source, "invalid opaque graph components"));
    }
    Ok(())
}

fn recompose(components: &[MorphologyGraphComponent<'_>]) -> String {
    components
        .iter()
        .flat_map(|component| component.surface.nfd())
        .collect::<String>()
        .nfc()
        .collect()
}

fn increment<K: Ord>(
    source: &str,
    counts: &mut BTreeMap<K, u32>,
    key: K,
    label: &str,
) -> Result<(), DataError> {
    let count = counts.entry(key).or_default();
    *count = count
        .checked_add(1)
        .ok_or_else(|| resource_error(source, &format!("{label} overflow")))?;
    Ok(())
}

fn checked_add(source: &str, left: usize, right: usize, label: &str) -> Result<usize, DataError> {
    left.checked_add(right)
        .ok_or_else(|| resource_error(source, &format!("{label} overflow")))
}

fn checked_mul(source: &str, left: usize, right: usize, label: &str) -> Result<usize, DataError> {
    left.checked_mul(right)
        .ok_or_else(|| resource_error(source, &format!("{label} overflow")))
}

fn to_usize(source: &str, value: u32) -> Result<usize, DataError> {
    usize::try_from(value).map_err(|error| resource_error(source, &error.to_string()))
}

fn read_u16_at(input: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        input.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32_at(input: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        input.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_i32_at(input: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        input.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
