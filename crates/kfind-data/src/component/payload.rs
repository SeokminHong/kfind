use std::collections::{BTreeMap, BTreeSet};

use crate::{DataError, MecabSourceMorphologyEntry};

use super::{ComponentAnalysis, build_conversion_error, build_error, resource_error};

const ANALYSIS_BYTES: usize = 28;

pub(super) struct EncodedPayload {
    pub bytes: Vec<u8>,
    pub strings: Vec<u8>,
    pub analysis_count: u32,
}

pub(super) fn encode(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
) -> Result<EncodedPayload, DataError> {
    let mut pos_counts = BTreeMap::<String, u32>::new();
    for entry in groups.iter().flat_map(|(_, group)| group) {
        let count = pos_counts.entry(entry.pos.clone()).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| build_error("component POS count overflow"))?;
    }
    let string_ids = groups
        .iter()
        .flat_map(|(_, analyses)| analyses)
        .flat_map(|entry| {
            [
                entry.pos.as_str(),
                entry.analysis_type.as_str(),
                entry.start_pos.as_str(),
                entry.end_pos.as_str(),
                entry.expression.as_str(),
            ]
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
    let analysis_count = pos_counts.values().try_fold(0_u32, |total, count| {
        total
            .checked_add(*count)
            .ok_or_else(|| build_error("component analysis count overflow"))
    })?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u32::try_from(groups.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&analysis_count.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(pos_counts.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    for (pos, count) in &pos_counts {
        bytes.extend_from_slice(&string_ids[pos].to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    let mut offset = 0_u32;
    bytes.extend_from_slice(&offset.to_le_bytes());
    for (_, analyses) in groups {
        offset = offset
            .checked_add(u32::try_from(analyses.len()).map_err(build_conversion_error)?)
            .ok_or_else(|| build_error("component analysis offset overflow"))?;
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
            bytes.extend_from_slice(&string_ids[value].to_le_bytes());
        }
    }
    Ok(EncodedPayload {
        bytes,
        strings,
        analysis_count,
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
    offsets_start: usize,
    records_start: usize,
}

impl PayloadLayout {
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
        let layout = Self {
            surface_count,
            analysis_count,
            offsets_start,
            records_start,
        };
        layout.validate_offsets(source, input)?;
        let expected = layout.pos_counts(source, input, pos_count, string_bytes, strings)?;
        let mut actual = vec![0_u32; strings.len()];
        for index in 0..analysis_count {
            let record = layout
                .record(input, index)
                .ok_or_else(|| resource_error(source, "invalid analysis record"))?;
            let left_id = u16::from_le_bytes(record[0..2].try_into().expect("record width"));
            let right_id = u16::from_le_bytes(record[2..4].try_into().expect("record width"));
            if left_id >= left_contexts || right_id >= right_contexts {
                return Err(resource_error(
                    source,
                    "analysis context ID is out of range",
                ));
            }
            let pos_id = usize::try_from(
                read_u32_at(record, 8)
                    .ok_or_else(|| resource_error(source, "truncated analysis POS"))?,
            )
            .map_err(|error| resource_error(source, &error.to_string()))?;
            let count = actual
                .get_mut(pos_id)
                .ok_or_else(|| resource_error(source, "invalid analysis POS ID"))?;
            *count = count
                .checked_add(1)
                .ok_or_else(|| resource_error(source, "POS count overflow"))?;
            for offset in [12, 16, 20, 24] {
                let string_id = read_u32_at(record, offset)
                    .ok_or_else(|| resource_error(source, "truncated analysis metadata"))?;
                strings
                    .get(string_bytes, string_id)
                    .ok_or_else(|| resource_error(source, "invalid analysis metadata string ID"))?;
            }
        }
        if actual != expected {
            return Err(resource_error(
                source,
                "POS counts do not match analysis records",
            ));
        }
        let pos_counts = expected
            .into_iter()
            .enumerate()
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
        let record = self.record(input, index)?;
        Some(ComponentAnalysis {
            left_id: u16::from_le_bytes(record[0..2].try_into().ok()?),
            right_id: u16::from_le_bytes(record[2..4].try_into().ok()?),
            word_cost: i32::from_le_bytes(record[4..8].try_into().ok()?),
            pos: strings.get(string_bytes, read_u32_at(record, 8)?)?,
            analysis_type: strings.get(string_bytes, read_u32_at(record, 12)?)?,
            start_pos: strings.get(string_bytes, read_u32_at(record, 16)?)?,
            end_pos: strings.get(string_bytes, read_u32_at(record, 20)?)?,
            expression: strings.get(string_bytes, read_u32_at(record, 24)?)?,
        })
    }

    fn record<'a>(&self, input: &'a [u8], index: u32) -> Option<&'a [u8]> {
        if index >= self.analysis_count {
            return None;
        }
        let offset = self
            .records_start
            .checked_add(usize::try_from(index).ok()?.checked_mul(ANALYSIS_BYTES)?)?;
        input.get(offset..offset.checked_add(ANALYSIS_BYTES)?)
    }

    fn validate_offsets(&self, source: &str, input: &[u8]) -> Result<(), DataError> {
        let mut previous = 0_u32;
        for index in 0..=usize::try_from(self.surface_count)
            .map_err(|error| resource_error(source, &error.to_string()))?
        {
            let offset = read_u32_at(
                input,
                self.offsets_start
                    .checked_add(index.checked_mul(4).ok_or_else(|| {
                        resource_error(source, "payload offset position overflow")
                    })?)
                    .ok_or_else(|| resource_error(source, "payload offset position overflow"))?,
            )
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
        input: &[u8],
        count: u32,
        string_bytes: &[u8],
        strings: &StringLayout,
    ) -> Result<Vec<u32>, DataError> {
        let mut by_id = vec![0_u32; strings.len()];
        for index in
            0..usize::try_from(count).map_err(|error| resource_error(source, &error.to_string()))?
        {
            let offset = 12_usize
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
