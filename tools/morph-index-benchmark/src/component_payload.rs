use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use kfind_data::MecabSourceMorphologyEntry;

const ANALYSIS_BYTES: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompactComponentAnalysis<'a> {
    pub pos: &'a str,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

pub fn encode(
    groups: &[(String, Vec<MecabSourceMorphologyEntry>)],
) -> Result<(Vec<u8>, Vec<u8>, u32)> {
    let mut pos_counts = BTreeMap::<String, u32>::new();
    for (_, analyses) in groups {
        for analysis in analyses {
            *pos_counts.entry(analysis.pos.clone()).or_default() += 1;
        }
    }
    let pos_ids = pos_counts
        .keys()
        .enumerate()
        .map(|(index, pos)| Ok((pos.clone(), u32::try_from(index)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let strings = encode_strings(pos_ids.keys())?;
    let analysis_count: u32 = pos_counts.values().sum();
    let mut output = Vec::new();
    output.extend_from_slice(&u32::try_from(groups.len())?.to_le_bytes());
    output.extend_from_slice(&analysis_count.to_le_bytes());
    output.extend_from_slice(&u32::try_from(pos_counts.len())?.to_le_bytes());
    for (pos, count) in &pos_counts {
        output.extend_from_slice(&pos_ids[pos].to_le_bytes());
        output.extend_from_slice(&count.to_le_bytes());
    }
    let mut offset = 0_u32;
    output.extend_from_slice(&offset.to_le_bytes());
    for (_, analyses) in groups {
        offset = offset
            .checked_add(u32::try_from(analyses.len())?)
            .ok_or_else(|| anyhow::anyhow!("component analysis count overflow"))?;
        output.extend_from_slice(&offset.to_le_bytes());
    }
    for (_, analyses) in groups {
        for analysis in analyses {
            output.extend_from_slice(&analysis.left_id.to_le_bytes());
            output.extend_from_slice(&analysis.right_id.to_le_bytes());
            output.extend_from_slice(&analysis.word_cost.to_le_bytes());
            output.extend_from_slice(&pos_ids[&analysis.pos].to_le_bytes());
        }
    }
    Ok((output, strings, analysis_count))
}

fn encode_strings<'a>(values: impl Iterator<Item = &'a String>) -> Result<Vec<u8>> {
    let values = values.collect::<Vec<_>>();
    let mut output = Vec::new();
    output.extend_from_slice(&u32::try_from(values.len())?.to_le_bytes());
    let mut offset = 0_u32;
    output.extend_from_slice(&offset.to_le_bytes());
    for value in &values {
        offset = offset
            .checked_add(u32::try_from(value.len())?)
            .ok_or_else(|| anyhow::anyhow!("component string table overflow"))?;
        output.extend_from_slice(&offset.to_le_bytes());
    }
    for value in values {
        output.extend_from_slice(value.as_bytes());
    }
    Ok(output)
}

pub struct StringTable<'a> {
    offsets: &'a [u8],
    values: &'a [u8],
    count: u32,
}

impl<'a> StringTable<'a> {
    pub fn parse(input: &'a [u8]) -> Result<Self> {
        let count = read_u32_at(input, 0)
            .ok_or_else(|| anyhow::anyhow!("component string table is truncated"))?;
        let values_start = 4_usize
            .checked_add(
                (usize::try_from(count)? + 1)
                    .checked_mul(4)
                    .ok_or_else(|| anyhow::anyhow!("component string offsets overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("component string table overflow"))?;
        let offsets = input
            .get(4..values_start)
            .ok_or_else(|| anyhow::anyhow!("component string offsets are truncated"))?;
        let values = input
            .get(values_start..)
            .ok_or_else(|| anyhow::anyhow!("component string values are truncated"))?;
        let table = Self {
            offsets,
            values,
            count,
        };
        let mut previous = 0_u32;
        for index in 0..=usize::try_from(count)? {
            let offset = table
                .offset(index)
                .ok_or_else(|| anyhow::anyhow!("component string offset is truncated"))?;
            ensure!(
                offset >= previous && usize::try_from(offset)? <= values.len(),
                "component string offsets are invalid"
            );
            ensure!(
                index > 0 || offset == 0,
                "component first string offset is not zero"
            );
            previous = offset;
        }
        ensure!(
            usize::try_from(previous)? == values.len(),
            "component string table length mismatch"
        );
        for id in 0..count {
            ensure!(table.get(id).is_some(), "component POS is not valid UTF-8");
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

    fn len(&self) -> usize {
        usize::try_from(self.count).expect("validated string count fits usize")
    }

    fn offset(&self, index: usize) -> Option<u32> {
        read_u32_at(self.offsets, index.checked_mul(4)?)
    }
}

pub struct PayloadView<'a> {
    input: &'a [u8],
    surface_count: u32,
    analysis_count: u32,
    offsets_start: usize,
    records_start: usize,
}

impl<'a> PayloadView<'a> {
    pub fn parse(
        input: &'a [u8],
        surface_count: u32,
        analysis_count: u32,
        right_contexts: u16,
        left_contexts: u16,
        strings: &StringTable<'_>,
    ) -> Result<(Self, BTreeMap<String, u32>)> {
        ensure!(
            read_u32_at(input, 0) == Some(surface_count)
                && read_u32_at(input, 4) == Some(analysis_count),
            "component payload counts mismatch"
        );
        let pos_count = read_u32_at(input, 8)
            .ok_or_else(|| anyhow::anyhow!("component POS counts are truncated"))?;
        let offsets_start = 12_usize
            .checked_add(
                usize::try_from(pos_count)?
                    .checked_mul(8)
                    .ok_or_else(|| anyhow::anyhow!("component POS counts overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("component payload header overflow"))?;
        let records_start = offsets_start
            .checked_add(
                (usize::try_from(surface_count)? + 1)
                    .checked_mul(4)
                    .ok_or_else(|| anyhow::anyhow!("component payload offsets overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("component payload overflow"))?;
        let expected_len = records_start
            .checked_add(
                usize::try_from(analysis_count)?
                    .checked_mul(ANALYSIS_BYTES)
                    .ok_or_else(|| anyhow::anyhow!("component analysis records overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("component payload length overflow"))?;
        ensure!(
            input.len() == expected_len,
            "component payload length mismatch"
        );
        let view = Self {
            input,
            surface_count,
            analysis_count,
            offsets_start,
            records_start,
        };
        view.validate_offsets()?;
        let (expected, pos_counts) = view.pos_counts(pos_count, strings)?;
        let mut actual = vec![0_u32; strings.len()];
        for index in 0..analysis_count {
            let record = view
                .record(index)
                .ok_or_else(|| anyhow::anyhow!("component analysis record is invalid"))?;
            let left_id = u16::from_le_bytes(record[0..2].try_into()?);
            let right_id = u16::from_le_bytes(record[2..4].try_into()?);
            ensure!(
                left_id < left_contexts && right_id < right_contexts,
                "component analysis context ID is out of range"
            );
            let pos_id = usize::try_from(
                read_u32_at(record, 8)
                    .ok_or_else(|| anyhow::anyhow!("component analysis POS is truncated"))?,
            )?;
            let count = actual
                .get_mut(pos_id)
                .ok_or_else(|| anyhow::anyhow!("component analysis POS ID is invalid"))?;
            *count = count
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("component POS count overflow"))?;
        }
        ensure!(actual == expected, "component POS counts mismatch");
        Ok((view, pos_counts))
    }

    pub fn group(
        &self,
        group: u32,
        strings: &StringTable<'a>,
    ) -> Option<Vec<CompactComponentAnalysis<'a>>> {
        if group >= self.surface_count {
            return None;
        }
        let index = usize::try_from(group).ok()?;
        let start = read_u32_at(self.input, self.offsets_start + index * 4)?;
        let end = read_u32_at(self.input, self.offsets_start + (index + 1) * 4)?;
        (start..end)
            .map(|record| self.analysis(record, strings))
            .collect()
    }

    fn analysis(
        &self,
        index: u32,
        strings: &StringTable<'a>,
    ) -> Option<CompactComponentAnalysis<'a>> {
        let input = self.record(index)?;
        Some(CompactComponentAnalysis {
            left_id: u16::from_le_bytes(input[0..2].try_into().ok()?),
            right_id: u16::from_le_bytes(input[2..4].try_into().ok()?),
            word_cost: i32::from_le_bytes(input[4..8].try_into().ok()?),
            pos: strings.get(read_u32_at(input, 8)?)?,
        })
    }

    fn record(&self, index: u32) -> Option<&'a [u8]> {
        if index >= self.analysis_count {
            return None;
        }
        let offset = self
            .records_start
            .checked_add(usize::try_from(index).ok()?.checked_mul(ANALYSIS_BYTES)?)?;
        self.input.get(offset..offset + ANALYSIS_BYTES)
    }

    fn validate_offsets(&self) -> Result<()> {
        let mut previous = 0_u32;
        for index in 0..=usize::try_from(self.surface_count)? {
            let offset = read_u32_at(self.input, self.offsets_start + index * 4)
                .ok_or_else(|| anyhow::anyhow!("component payload offset is truncated"))?;
            ensure!(
                offset >= previous && offset <= self.analysis_count,
                "component payload offsets are invalid"
            );
            ensure!(
                index > 0 || offset == 0,
                "component first payload offset is not zero"
            );
            previous = offset;
        }
        ensure!(
            previous == self.analysis_count,
            "component final payload offset mismatch"
        );
        Ok(())
    }

    fn pos_counts(
        &self,
        count: u32,
        strings: &StringTable<'_>,
    ) -> Result<(Vec<u32>, BTreeMap<String, u32>)> {
        let mut by_id = vec![0_u32; strings.len()];
        let mut by_pos = BTreeMap::new();
        for index in 0..usize::try_from(count)? {
            let offset = 12 + index * 8;
            let id = read_u32_at(self.input, offset)
                .ok_or_else(|| anyhow::anyhow!("component POS ID is truncated"))?;
            let value = read_u32_at(self.input, offset + 4)
                .filter(|value| *value > 0)
                .ok_or_else(|| anyhow::anyhow!("component POS count is invalid"))?;
            let pos = strings
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("component POS ID is invalid"))?;
            let slot = by_id
                .get_mut(usize::try_from(id)?)
                .ok_or_else(|| anyhow::anyhow!("component POS ID is invalid"))?;
            ensure!(*slot == 0, "component POS count is duplicated");
            *slot = value;
            ensure!(
                by_pos.insert(pos.to_owned(), value).is_none(),
                "component POS count is duplicated"
            );
        }
        Ok((by_id, by_pos))
    }
}

fn read_u32_at(input: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        input.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}
