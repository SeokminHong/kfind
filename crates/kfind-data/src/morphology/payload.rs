use std::collections::{BTreeMap, BTreeSet};

use crate::{DataError, MecabSourceMorphologyEntry};

use super::{
    MorphologyAnalysis, binary_conversion_error, binary_error, read_u32_at, resource_error,
};

const ANALYSIS_BYTES: usize = 28;

pub(super) struct EncodedPayload {
    pub bytes: Vec<u8>,
    pub strings: Vec<u8>,
    pub analysis_count: u32,
}

pub(super) fn encode(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
) -> Result<EncodedPayload, DataError> {
    let analysis_count = u32::try_from(groups.iter().map(|(_, group)| group.len()).sum::<usize>())
        .map_err(binary_conversion_error)?;
    let (strings, string_ids) = encode_strings(groups)?;
    let mut pos_counts = BTreeMap::<u32, u32>::new();
    for entry in groups.iter().flat_map(|(_, group)| group) {
        let pos = string_id(&string_ids, &entry.pos)?;
        let count = pos_counts.entry(pos).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| binary_error("POS count overflow"))?;
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u32::try_from(groups.len())
            .map_err(binary_conversion_error)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&analysis_count.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(pos_counts.len())
            .map_err(binary_conversion_error)?
            .to_le_bytes(),
    );
    for (pos, count) in &pos_counts {
        bytes.extend_from_slice(&pos.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    let mut offset = 0_u32;
    bytes.extend_from_slice(&offset.to_le_bytes());
    for (_, group) in groups {
        offset = offset
            .checked_add(u32::try_from(group.len()).map_err(binary_conversion_error)?)
            .ok_or_else(|| binary_error("analysis offset overflow"))?;
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    for entry in groups.iter().flat_map(|(_, group)| group) {
        bytes.extend_from_slice(&entry.left_id.to_le_bytes());
        bytes.extend_from_slice(&entry.right_id.to_le_bytes());
        bytes.extend_from_slice(&entry.word_cost.to_le_bytes());
        for value in [
            &entry.pos,
            &entry.analysis_type,
            &entry.start_pos,
            &entry.end_pos,
            &entry.expression,
        ] {
            bytes.extend_from_slice(&string_id(&string_ids, value)?.to_le_bytes());
        }
    }
    Ok(EncodedPayload {
        bytes,
        strings,
        analysis_count,
    })
}

fn encode_strings(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
) -> Result<(Vec<u8>, BTreeMap<String, u32>), DataError> {
    let mut unique = BTreeSet::new();
    for entry in groups.iter().flat_map(|(_, group)| group) {
        unique.extend([
            entry.pos.as_str(),
            entry.analysis_type.as_str(),
            entry.start_pos.as_str(),
            entry.end_pos.as_str(),
            entry.expression.as_str(),
        ]);
    }
    let ids = unique
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            Ok((
                value.to_owned(),
                u32::try_from(index).map_err(binary_conversion_error)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, DataError>>()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u32::try_from(ids.len())
            .map_err(binary_conversion_error)?
            .to_le_bytes(),
    );
    let mut offset = 0_u32;
    bytes.extend_from_slice(&offset.to_le_bytes());
    for value in ids.keys() {
        offset = offset
            .checked_add(u32::try_from(value.len()).map_err(binary_conversion_error)?)
            .ok_or_else(|| binary_error("string table offset overflow"))?;
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    for value in ids.keys() {
        bytes.extend_from_slice(value.as_bytes());
    }
    Ok((bytes, ids))
}

fn string_id(ids: &BTreeMap<String, u32>, value: &str) -> Result<u32, DataError> {
    ids.get(value)
        .copied()
        .ok_or_else(|| binary_error("analysis metadata string is not interned"))
}

pub(super) struct StringTable<'a> {
    offsets: &'a [u8],
    values: &'a [u8],
    count: u32,
}

impl<'a> StringTable<'a> {
    pub fn parse(source: &str, input: &'a [u8]) -> Result<Self, DataError> {
        let count = read_u32_at(input, 0)
            .ok_or_else(|| resource_error(source, "truncated string table"))?;
        let offset_count = usize::try_from(count)
            .map_err(|error| resource_error(source, &error.to_string()))?
            .checked_add(1)
            .ok_or_else(|| resource_error(source, "string offset count overflow"))?;
        let values_start = 4_usize
            .checked_add(
                offset_count
                    .checked_mul(4)
                    .ok_or_else(|| resource_error(source, "string offsets overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "string table header overflow"))?;
        let offsets = input
            .get(4..values_start)
            .ok_or_else(|| resource_error(source, "truncated string offsets"))?;
        let values = input
            .get(values_start..)
            .ok_or_else(|| resource_error(source, "truncated string values"))?;
        let table = Self {
            offsets,
            values,
            count,
        };
        let mut previous = 0_u32;
        for index in 0..offset_count {
            let offset = table
                .offset(index)
                .ok_or_else(|| resource_error(source, "truncated string offset"))?;
            if offset < previous
                || usize::try_from(offset).map_or(true, |offset| offset > values.len())
                || (index == 0 && offset != 0)
            {
                return Err(resource_error(source, "invalid string offset order"));
            }
            previous = offset;
        }
        if usize::try_from(previous).ok() != Some(values.len()) {
            return Err(resource_error(source, "final string offset mismatch"));
        }
        for id in 0..count {
            table
                .get(id)
                .ok_or_else(|| resource_error(source, "invalid UTF-8 in string table"))?;
        }
        Ok(table)
    }

    pub fn get(&self, id: u32) -> Option<&'a str> {
        if id >= self.count {
            return None;
        }
        let start = usize::try_from(self.offset(usize::try_from(id).ok()?)?).ok()?;
        let end = usize::try_from(self.offset(usize::try_from(id).ok()?.checked_add(1)?)?).ok()?;
        std::str::from_utf8(self.values.get(start..end)?).ok()
    }

    fn offset(&self, index: usize) -> Option<u32> {
        read_u32_at(self.offsets, index.checked_mul(4)?)
    }
}

pub(super) struct PayloadView<'a> {
    input: &'a [u8],
    surface_count: u32,
    analysis_count: u32,
    offsets_start: usize,
    records_start: usize,
}

impl<'a> PayloadView<'a> {
    pub fn parse(
        source: &str,
        input: &'a [u8],
        surface_count: u32,
        analysis_count: u32,
        right_contexts: u16,
        left_contexts: u16,
        strings: &StringTable<'_>,
    ) -> Result<(Self, BTreeMap<String, u32>), DataError> {
        if read_u32_at(input, 0) != Some(surface_count)
            || read_u32_at(input, 4) != Some(analysis_count)
        {
            return Err(resource_error(source, "payload counts mismatch"));
        }
        let pos_count =
            read_u32_at(input, 8).ok_or_else(|| resource_error(source, "truncated POS counts"))?;
        let offsets_start = 12_usize
            .checked_add(
                usize::try_from(pos_count)
                    .map_err(|error| resource_error(source, &error.to_string()))?
                    .checked_mul(8)
                    .ok_or_else(|| resource_error(source, "POS counts overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "payload header overflow"))?;
        let offset_count = usize::try_from(surface_count)
            .map_err(|error| resource_error(source, &error.to_string()))?
            .checked_add(1)
            .ok_or_else(|| resource_error(source, "payload offset count overflow"))?;
        let records_start = offsets_start
            .checked_add(
                offset_count
                    .checked_mul(4)
                    .ok_or_else(|| resource_error(source, "payload offsets overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "payload records overflow"))?;
        let expected_len = records_start
            .checked_add(
                usize::try_from(analysis_count)
                    .map_err(|error| resource_error(source, &error.to_string()))?
                    .checked_mul(ANALYSIS_BYTES)
                    .ok_or_else(|| resource_error(source, "payload records overflow"))?,
            )
            .ok_or_else(|| resource_error(source, "payload length overflow"))?;
        if input.len() != expected_len {
            return Err(resource_error(source, "payload length mismatch"));
        }
        let view = Self {
            input,
            surface_count,
            analysis_count,
            offsets_start,
            records_start,
        };
        view.validate_offsets(source)?;
        let expected_pos_counts = view.pos_counts(source, pos_count, strings)?;
        let mut actual_pos_counts = BTreeMap::<String, u32>::new();
        for index in 0..analysis_count {
            let analysis = view
                .analysis(index, strings)
                .ok_or_else(|| resource_error(source, "invalid analysis record"))?;
            if analysis.left_id >= left_contexts || analysis.right_id >= right_contexts {
                return Err(resource_error(
                    source,
                    "analysis context ID is out of range",
                ));
            }
            *actual_pos_counts
                .entry(analysis.pos.to_owned())
                .or_default() += 1;
        }
        if actual_pos_counts != expected_pos_counts {
            return Err(resource_error(
                source,
                "POS counts do not match analysis records",
            ));
        }
        Ok((view, actual_pos_counts))
    }

    pub fn group(
        &self,
        group: u32,
        strings: &StringTable<'a>,
    ) -> Option<Vec<MorphologyAnalysis<'a>>> {
        if group >= self.surface_count {
            return None;
        }
        let index = usize::try_from(group).ok()?;
        let start = read_u32_at(self.input, self.offsets_start + index * 4)?;
        let end = read_u32_at(self.input, self.offsets_start + (index + 1) * 4)?;
        (start..end)
            .map(|analysis| self.analysis(analysis, strings))
            .collect()
    }

    fn validate_offsets(&self, source: &str) -> Result<(), DataError> {
        let mut previous = 0_u32;
        for index in 0..=usize::try_from(self.surface_count)
            .map_err(|error| resource_error(source, &error.to_string()))?
        {
            let offset = read_u32_at(self.input, self.offsets_start + index * 4)
                .ok_or_else(|| resource_error(source, "truncated payload offset"))?;
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

    fn pos_counts(
        &self,
        source: &str,
        count: u32,
        strings: &StringTable<'_>,
    ) -> Result<BTreeMap<String, u32>, DataError> {
        let mut output = BTreeMap::new();
        for index in
            0..usize::try_from(count).map_err(|error| resource_error(source, &error.to_string()))?
        {
            let offset = 12 + index * 8;
            let id = read_u32_at(self.input, offset)
                .ok_or_else(|| resource_error(source, "truncated POS ID"))?;
            let value = read_u32_at(self.input, offset + 4)
                .filter(|value| *value > 0)
                .ok_or_else(|| resource_error(source, "invalid POS count"))?;
            let pos = strings
                .get(id)
                .ok_or_else(|| resource_error(source, "invalid POS string ID"))?;
            if output.insert(pos.to_owned(), value).is_some() {
                return Err(resource_error(source, "duplicate POS count"));
            }
        }
        Ok(output)
    }

    fn analysis(&self, index: u32, strings: &StringTable<'a>) -> Option<MorphologyAnalysis<'a>> {
        if index >= self.analysis_count {
            return None;
        }
        let offset = self
            .records_start
            .checked_add(usize::try_from(index).ok()?.checked_mul(ANALYSIS_BYTES)?)?;
        let input = self.input.get(offset..offset + ANALYSIS_BYTES)?;
        Some(MorphologyAnalysis {
            left_id: u16::from_le_bytes(input[0..2].try_into().ok()?),
            right_id: u16::from_le_bytes(input[2..4].try_into().ok()?),
            word_cost: i32::from_le_bytes(input[4..8].try_into().ok()?),
            pos: strings.get(read_u32_at(input, 8)?)?,
            analysis_type: strings.get(read_u32_at(input, 12)?)?,
            start_pos: strings.get(read_u32_at(input, 16)?)?,
            end_pos: strings.get(read_u32_at(input, 20)?)?,
            expression: strings.get(read_u32_at(input, 24)?)?,
        })
    }
}
