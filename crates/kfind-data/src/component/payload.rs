use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::{
    DataError, MecabSourceMorphologyEntry, MorphologyExpressionAlignmentKind,
    align_morphology_expression,
};

use super::{
    ComponentAnalysis, ComponentPart, build_conversion_error, build_error, resource_error,
};

const ANALYSIS_BYTES: usize = 12;
const COMPONENT_BYTES: usize = 12;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StructuralComponent {
    start: u32,
    end: u32,
    pos: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StructuralAnalysis {
    pos: String,
    components: Vec<StructuralComponent>,
}

pub(super) struct EncodedPayload {
    pub bytes: Vec<u8>,
    pub strings: Vec<u8>,
    pub analysis_count: u32,
    pub component_count: u32,
}

pub(super) fn encode(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
) -> Result<EncodedPayload, DataError> {
    let groups = groups
        .iter()
        .map(|(surface, entries)| {
            let mut analyses = entries
                .iter()
                .map(|entry| structural_analysis(surface, entry))
                .collect::<Result<Vec<_>, DataError>>()?;
            analyses.sort_unstable();
            analyses.dedup();
            Ok((surface, analyses))
        })
        .collect::<Result<Vec<_>, DataError>>()?;
    let mut pos_counts = BTreeMap::<String, u32>::new();
    for analysis in groups.iter().flat_map(|(_, group)| group) {
        let count = pos_counts.entry(analysis.pos.clone()).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| build_error("component POS count overflow"))?;
    }
    let string_ids = groups
        .iter()
        .flat_map(|(_, analyses)| analyses)
        .flat_map(|analysis| {
            std::iter::once(analysis.pos.as_str()).chain(
                analysis
                    .components
                    .iter()
                    .map(|component| component.pos.as_str()),
            )
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            Ok((
                value.to_owned(),
                u32::try_from(index).map_err(build_conversion_error)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, DataError>>()?;
    let strings = encode_strings(string_ids.keys())?;
    let analysis_count = groups.iter().try_fold(0_u32, |total, (_, analyses)| {
        total
            .checked_add(u32::try_from(analyses.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("component analysis count overflow"))
    })?;
    let component_count =
        groups
            .iter()
            .flat_map(|(_, analyses)| analyses)
            .try_fold(0_u32, |total, analysis| {
                total
                    .checked_add(
                        u32::try_from(analysis.components.len()).map_err(build_conversion_error)?,
                    )
                    .ok_or_else(|| build_error("component span count overflow"))
            })?;

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
    for (pos, count) in &pos_counts {
        bytes.extend_from_slice(&string_ids[pos].to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    let mut analysis_offset = 0_u32;
    bytes.extend_from_slice(&analysis_offset.to_le_bytes());
    for (_, analyses) in &groups {
        analysis_offset = analysis_offset
            .checked_add(u32::try_from(analyses.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("component analysis offset overflow"))?;
        bytes.extend_from_slice(&analysis_offset.to_le_bytes());
    }
    for (surface, _) in &groups {
        bytes.extend_from_slice(
            &u32::try_from(surface.len())
                .map_err(build_conversion_error)?
                .to_le_bytes(),
        );
    }
    let mut component_offset = 0_u32;
    for analysis in groups.iter().flat_map(|(_, group)| group) {
        bytes.extend_from_slice(&string_ids[&analysis.pos].to_le_bytes());
        bytes.extend_from_slice(&component_offset.to_le_bytes());
        bytes.extend_from_slice(
            &u32::try_from(analysis.components.len())
                .map_err(build_conversion_error)?
                .to_le_bytes(),
        );
        component_offset = component_offset
            .checked_add(u32::try_from(analysis.components.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("component span offset overflow"))?;
    }
    for component in groups
        .iter()
        .flat_map(|(_, group)| group)
        .flat_map(|analysis| &analysis.components)
    {
        bytes.extend_from_slice(&component.start.to_le_bytes());
        bytes.extend_from_slice(&component.end.to_le_bytes());
        bytes.extend_from_slice(&string_ids[&component.pos].to_le_bytes());
    }
    Ok(EncodedPayload {
        bytes,
        strings,
        analysis_count,
        component_count,
    })
}

fn structural_analysis(
    surface: &str,
    entry: &MecabSourceMorphologyEntry,
) -> Result<StructuralAnalysis, DataError> {
    let aligned = align_morphology_expression(surface, &entry.expression);
    let components = if aligned.kind == MorphologyExpressionAlignmentKind::SpanAligned {
        aligned
            .components
            .into_iter()
            .map(|component| {
                let span = component
                    .span
                    .ok_or_else(|| build_error("aligned component is missing a span"))?;
                Ok(StructuralComponent {
                    start: u32::try_from(span.start).map_err(build_conversion_error)?,
                    end: u32::try_from(span.end).map_err(build_conversion_error)?,
                    pos: component.pos.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?
    } else {
        Vec::new()
    };
    Ok(StructuralAnalysis {
        pos: entry.pos.clone(),
        components,
    })
}

fn encode_strings<'a>(values: impl Iterator<Item = &'a String>) -> Result<Vec<u8>, DataError> {
    let values = values.collect::<Vec<_>>();
    let mut output = Vec::new();
    output.extend_from_slice(
        &u32::try_from(values.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    let mut offset = 0_u32;
    output.extend_from_slice(&offset.to_le_bytes());
    for value in &values {
        offset = offset
            .checked_add(u32::try_from(value.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("component string table overflow"))?;
        output.extend_from_slice(&offset.to_le_bytes());
    }
    for value in values {
        output.extend_from_slice(value.as_bytes());
    }
    Ok(output)
}

#[derive(Clone, Debug)]
pub(super) struct StringLayout {
    count: u32,
    offsets_start: usize,
    values_start: usize,
}

impl StringLayout {
    pub fn parse(source: &str, input: &[u8]) -> Result<Self, DataError> {
        let count = read_u32_at(input, 0)
            .ok_or_else(|| resource_error(source, "truncated string table"))?;
        let offset_count = usize::try_from(count)
            .map_err(|error| resource_error(source, &error.to_string()))?
            .checked_add(1)
            .ok_or_else(|| resource_error(source, "string offset count overflow"))?;
        let offsets_start = 4_usize;
        let values_start = offsets_start
            .checked_add(
                offset_count
                    .checked_mul(4)
                    .ok_or_else(|| resource_error(source, "string offsets overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "string table header overflow"))?;
        if values_start > input.len() {
            return Err(resource_error(source, "truncated string offsets"));
        }
        let layout = Self {
            count,
            offsets_start,
            values_start,
        };
        let mut previous = 0_u32;
        for index in 0..offset_count {
            let offset = layout
                .offset(input, index)
                .ok_or_else(|| resource_error(source, "truncated string offset"))?;
            if offset < previous
                || usize::try_from(offset)
                    .map_or(true, |offset| offset > input.len() - values_start)
                || (index == 0 && offset != 0)
            {
                return Err(resource_error(source, "invalid string offset order"));
            }
            previous = offset;
        }
        if usize::try_from(previous).ok() != Some(input.len() - values_start) {
            return Err(resource_error(source, "final string offset mismatch"));
        }
        for id in 0..count {
            layout
                .get(input, id)
                .ok_or_else(|| resource_error(source, "invalid UTF-8 in string table"))?;
        }
        Ok(layout)
    }

    pub fn get<'a>(&self, input: &'a [u8], id: u32) -> Option<&'a str> {
        if id >= self.count {
            return None;
        }
        let index = usize::try_from(id).ok()?;
        let start = self
            .values_start
            .checked_add(usize::try_from(self.offset(input, index)?).ok()?)?;
        let end = self
            .values_start
            .checked_add(usize::try_from(self.offset(input, index.checked_add(1)?)?).ok()?)?;
        std::str::from_utf8(input.get(start..end)?).ok()
    }

    fn len(&self) -> usize {
        usize::try_from(self.count).expect("validated string count fits usize")
    }

    fn offset(&self, input: &[u8], index: usize) -> Option<u32> {
        read_u32_at(
            input,
            self.offsets_start.checked_add(index.checked_mul(4)?)?,
        )
    }
}

#[derive(Clone, Debug)]
pub(super) struct PayloadLayout {
    surface_count: u32,
    analysis_count: u32,
    component_count: u32,
    offsets_start: usize,
    surface_lengths_start: usize,
    analysis_records_start: usize,
    component_records_start: usize,
}

impl PayloadLayout {
    pub fn parse(
        source: &str,
        input: &[u8],
        surface_count: u32,
        analysis_count: u32,
        component_count: u32,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<(Self, BTreeMap<String, u32>), DataError> {
        if read_u32_at(input, 0) != Some(surface_count)
            || read_u32_at(input, 4) != Some(analysis_count)
            || read_u32_at(input, 8) != Some(component_count)
        {
            return Err(resource_error(source, "payload counts mismatch"));
        }
        let pos_count =
            read_u32_at(input, 12).ok_or_else(|| resource_error(source, "truncated POS counts"))?;
        let offsets_start = 16_usize
            .checked_add(
                usize::try_from(pos_count)
                    .map_err(|error| resource_error(source, &error.to_string()))?
                    .checked_mul(8)
                    .ok_or_else(|| resource_error(source, "POS counts overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "payload header overflow"))?;
        let surface_count_usize = usize::try_from(surface_count)
            .map_err(|error| resource_error(source, &error.to_string()))?;
        let offset_count = surface_count_usize
            .checked_add(1)
            .ok_or_else(|| resource_error(source, "payload offset count overflow"))?;
        let surface_lengths_start = offsets_start
            .checked_add(
                offset_count
                    .checked_mul(4)
                    .ok_or_else(|| resource_error(source, "payload offsets overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "payload surface lengths overflow"))?;
        let analysis_records_start = surface_lengths_start
            .checked_add(
                surface_count_usize
                    .checked_mul(4)
                    .ok_or_else(|| resource_error(source, "surface lengths overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "analysis records overflow"))?;
        let component_records_start = analysis_records_start
            .checked_add(
                usize::try_from(analysis_count)
                    .map_err(|error| resource_error(source, &error.to_string()))?
                    .checked_mul(ANALYSIS_BYTES)
                    .ok_or_else(|| resource_error(source, "analysis records overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "component records overflow"))?;
        let expected_len = component_records_start
            .checked_add(
                usize::try_from(component_count)
                    .map_err(|error| resource_error(source, &error.to_string()))?
                    .checked_mul(COMPONENT_BYTES)
                    .ok_or_else(|| resource_error(source, "component records overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "payload length overflow"))?;
        if input.len() != expected_len {
            return Err(resource_error(source, "payload length mismatch"));
        }
        let layout = Self {
            surface_count,
            analysis_count,
            component_count,
            offsets_start,
            surface_lengths_start,
            analysis_records_start,
            component_records_start,
        };
        layout.validate_offsets(source, input)?;
        let expected = layout.pos_counts(source, input, pos_count, string_bytes, strings)?;
        let actual = layout.validate_records(source, input, string_bytes, strings)?;
        if actual != expected {
            return Err(resource_error(
                source,
                "POS counts do not match analysis records",
            ));
        }
        let pos_counts = expected
            .into_iter()
            .enumerate()
            .filter(|(_, count)| *count > 0)
            .map(|(id, count)| {
                let pos = strings
                    .get(string_bytes, u32::try_from(id).ok()?)?
                    .to_owned();
                Some((pos, count))
            })
            .collect::<Option<BTreeMap<_, _>>>()
            .ok_or_else(|| resource_error(source, "invalid POS string ID"))?;
        Ok((layout, pos_counts))
    }

    pub fn group<'a>(
        &self,
        input: &[u8],
        group: u32,
        string_bytes: &'a [u8],
        strings: &StringLayout,
    ) -> Option<Vec<ComponentAnalysis<'a>>> {
        if group >= self.surface_count {
            return None;
        }
        let index = usize::try_from(group).ok()?;
        let start = read_u32_at(
            input,
            self.offsets_start.checked_add(index.checked_mul(4)?)?,
        )?;
        let end = read_u32_at(
            input,
            self.offsets_start
                .checked_add(index.checked_add(1)?.checked_mul(4)?)?,
        )?;
        (start..end)
            .map(|record| self.analysis(input, record, string_bytes, strings))
            .collect()
    }

    fn analysis<'a>(
        &self,
        input: &[u8],
        index: u32,
        string_bytes: &'a [u8],
        strings: &StringLayout,
    ) -> Option<ComponentAnalysis<'a>> {
        let record = self.analysis_record(input, index)?;
        let component_start = read_u32_at(record, 4)?;
        let component_len = read_u32_at(record, 8)?;
        let components = (component_start..component_start.checked_add(component_len)?)
            .map(|index| {
                let record = self.component_record(input, index)?;
                Some(ComponentPart {
                    span: usize::try_from(read_u32_at(record, 0)?).ok()?
                        ..usize::try_from(read_u32_at(record, 4)?).ok()?,
                    pos: strings.get(string_bytes, read_u32_at(record, 8)?)?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(ComponentAnalysis {
            pos: strings.get(string_bytes, read_u32_at(record, 0)?)?,
            components,
        })
    }

    fn validate_records(
        &self,
        source: &str,
        input: &[u8],
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<Vec<u32>, DataError> {
        let mut actual = vec![0_u32; strings.len()];
        let mut expected_component_start = 0_u32;
        for group in 0..self.surface_count {
            let group_index = usize::try_from(group)
                .map_err(|error| resource_error(source, &error.to_string()))?;
            let surface_len = read_u32_at(
                input,
                self.surface_lengths_start
                    .checked_add(group_index.checked_mul(4).ok_or_else(|| {
                        resource_error(source, "surface length position overflow")
                    })?)
                    .ok_or_else(|| resource_error(source, "surface length position overflow"))?,
            )
            .filter(|length| *length > 0)
            .ok_or_else(|| resource_error(source, "invalid surface length"))?;
            let analysis_start = self.group_offset(input, group_index)?;
            let analysis_end = self.group_offset(input, group_index + 1)?;
            for index in analysis_start..analysis_end {
                let record = self
                    .analysis_record(input, index)
                    .ok_or_else(|| resource_error(source, "invalid analysis record"))?;
                let pos_id = read_u32_at(record, 0)
                    .ok_or_else(|| resource_error(source, "truncated analysis POS"))?;
                strings
                    .get(string_bytes, pos_id)
                    .ok_or_else(|| resource_error(source, "invalid analysis POS ID"))?;
                let count = actual
                    .get_mut(
                        usize::try_from(pos_id)
                            .map_err(|error| resource_error(source, &error.to_string()))?,
                    )
                    .ok_or_else(|| resource_error(source, "invalid analysis POS ID"))?;
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| resource_error(source, "POS count overflow"))?;
                let component_start = read_u32_at(record, 4)
                    .ok_or_else(|| resource_error(source, "truncated component offset"))?;
                let component_len = read_u32_at(record, 8)
                    .ok_or_else(|| resource_error(source, "truncated component length"))?;
                if component_start != expected_component_start {
                    return Err(resource_error(source, "invalid component offset order"));
                }
                expected_component_start = component_start
                    .checked_add(component_len)
                    .filter(|end| *end <= self.component_count)
                    .ok_or_else(|| resource_error(source, "component range overflow"))?;
                self.validate_components(
                    source,
                    input,
                    component_start..expected_component_start,
                    surface_len,
                    string_bytes,
                    strings,
                )?;
            }
        }
        if expected_component_start != self.component_count {
            return Err(resource_error(source, "final component offset mismatch"));
        }
        Ok(actual)
    }

    fn validate_components(
        &self,
        source: &str,
        input: &[u8],
        range: Range<u32>,
        surface_len: u32,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<(), DataError> {
        let has_components = !range.is_empty();
        let mut previous_end = 0_u32;
        for index in range {
            let record = self
                .component_record(input, index)
                .ok_or_else(|| resource_error(source, "invalid component record"))?;
            let start = read_u32_at(record, 0)
                .ok_or_else(|| resource_error(source, "truncated component start"))?;
            let end = read_u32_at(record, 4)
                .filter(|end| start < *end && *end <= surface_len)
                .ok_or_else(|| resource_error(source, "invalid component span"))?;
            if start != previous_end {
                return Err(resource_error(source, "non-contiguous component spans"));
            }
            let pos_id = read_u32_at(record, 8)
                .ok_or_else(|| resource_error(source, "truncated component POS"))?;
            strings
                .get(string_bytes, pos_id)
                .ok_or_else(|| resource_error(source, "invalid component POS ID"))?;
            previous_end = end;
        }
        if has_components && previous_end != surface_len {
            return Err(resource_error(
                source,
                "component spans do not cover surface",
            ));
        }
        Ok(())
    }

    fn analysis_record<'a>(&self, input: &'a [u8], index: u32) -> Option<&'a [u8]> {
        if index >= self.analysis_count {
            return None;
        }
        let offset = self
            .analysis_records_start
            .checked_add(usize::try_from(index).ok()?.checked_mul(ANALYSIS_BYTES)?)?;
        input.get(offset..offset.checked_add(ANALYSIS_BYTES)?)
    }

    fn component_record<'a>(&self, input: &'a [u8], index: u32) -> Option<&'a [u8]> {
        if index >= self.component_count {
            return None;
        }
        let offset = self
            .component_records_start
            .checked_add(usize::try_from(index).ok()?.checked_mul(COMPONENT_BYTES)?)?;
        input.get(offset..offset.checked_add(COMPONENT_BYTES)?)
    }

    fn validate_offsets(&self, source: &str, input: &[u8]) -> Result<(), DataError> {
        let mut previous = 0_u32;
        for index in 0..=usize::try_from(self.surface_count)
            .map_err(|error| resource_error(source, &error.to_string()))?
        {
            let offset = self.group_offset(input, index)?;
            if offset < previous || offset > self.analysis_count || (index == 0 && offset != 0) {
                return Err(resource_error(source, "invalid payload offset order"));
            }
            previous = offset;
        }
        if previous != self.analysis_count {
            return Err(resource_error(source, "final payload offset mismatch"));
        }
        Ok(())
    }

    fn group_offset(&self, input: &[u8], index: usize) -> Result<u32, DataError> {
        read_u32_at(
            input,
            self.offsets_start
                .checked_add(index.checked_mul(4).ok_or_else(|| {
                    resource_error("component-resource", "payload offset position overflow")
                })?)
                .ok_or_else(|| {
                    resource_error("component-resource", "payload offset position overflow")
                })?,
        )
        .ok_or_else(|| resource_error("component-resource", "truncated payload offset"))
    }

    fn pos_counts(
        &self,
        source: &str,
        input: &[u8],
        count: u32,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<Vec<u32>, DataError> {
        let mut by_id = vec![0_u32; strings.len()];
        for index in
            0..usize::try_from(count).map_err(|error| resource_error(source, &error.to_string()))?
        {
            let offset = 16_usize
                .checked_add(
                    index
                        .checked_mul(8)
                        .ok_or_else(|| resource_error(source, "POS count position overflow"))?,
                )
                .ok_or_else(|| resource_error(source, "POS count position overflow"))?;
            let id = read_u32_at(input, offset)
                .ok_or_else(|| resource_error(source, "truncated POS ID"))?;
            let value = read_u32_at(input, offset + 4)
                .filter(|value| *value > 0)
                .ok_or_else(|| resource_error(source, "invalid POS count"))?;
            strings
                .get(string_bytes, id)
                .ok_or_else(|| resource_error(source, "invalid POS string ID"))?;
            let slot = by_id
                .get_mut(
                    usize::try_from(id)
                        .map_err(|error| resource_error(source, &error.to_string()))?,
                )
                .ok_or_else(|| resource_error(source, "invalid POS string ID"))?;
            if *slot != 0 {
                return Err(resource_error(source, "duplicate POS count"));
            }
            *slot = value;
        }
        Ok(by_id)
    }
}

fn read_u32_at(input: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        input.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}
