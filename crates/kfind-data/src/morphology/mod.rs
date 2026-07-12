use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::{
    DataError, DataErrorKind, DataFinePos, MecabConnectionMatrix, MecabMorphologyEntry,
    SourceLocation,
};

const MAGIC: &[u8; 8] = b"KFMORPH\0";
const SCHEMA_VERSION: u32 = 2;
const INDEX_KIND_DOUBLE_ARRAY: u8 = 1;
const POS_COUNT: usize = 23;
const HEADER_LEN: usize = 356;
const ANALYSIS_BYTES: usize = 12;
const SECTION_COUNT: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyAnalysis {
    pub pos: DataFinePos,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyResourceStats {
    pub surface_count: u32,
    pub analysis_count: u32,
    pub pos_counts: [u32; POS_COUNT],
    pub right_contexts: u16,
    pub left_contexts: u16,
}

pub struct DecodedMorphologyResource<'a> {
    stats: MorphologyResourceStats,
    index: DoubleArray<&'a [u8]>,
    payload: &'a [u8],
    matrix: &'a [u8],
    char_def: &'a [u8],
    unk_def: &'a [u8],
}

impl DecodedMorphologyResource<'_> {
    #[must_use]
    pub fn stats(&self) -> &MorphologyResourceStats {
        &self.stats
    }

    pub fn common_prefixes(
        &self,
        input: &[u8],
        mut emit: impl FnMut(usize, &[MorphologyAnalysis]),
    ) {
        for (group, length) in self.index.common_prefix_search(input) {
            if let Some(analyses) = self.analysis_group(group) {
                emit(length, &analyses);
            }
        }
    }

    #[must_use]
    pub fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        if right_id >= self.stats.right_contexts || left_id >= self.stats.left_contexts {
            return None;
        }
        let index = usize::from(right_id)
            .checked_mul(usize::from(self.stats.left_contexts))?
            .checked_add(usize::from(left_id))?;
        let offset = index.checked_mul(2)?;
        let bytes = self.matrix.get(offset..offset.checked_add(2)?)?;
        Some(i16::from_le_bytes(bytes.try_into().ok()?))
    }

    #[must_use]
    pub fn char_def(&self) -> &[u8] {
        self.char_def
    }

    #[must_use]
    pub fn unk_def(&self) -> &[u8] {
        self.unk_def
    }

    fn analysis_group(&self, group: u32) -> Option<Vec<MorphologyAnalysis>> {
        if group >= self.stats.surface_count {
            return None;
        }
        let offsets_start = 8_usize;
        let start_offset =
            offsets_start.checked_add(usize::try_from(group).ok()?.checked_mul(4)?)?;
        let end_offset = start_offset.checked_add(4)?;
        let start = usize::try_from(read_u32_at(self.payload, start_offset)?).ok()?;
        let end = usize::try_from(read_u32_at(self.payload, end_offset)?).ok()?;
        let records_start = offsets_start
            .checked_add((usize::try_from(self.stats.surface_count).ok()? + 1).checked_mul(4)?)?;
        (start..end)
            .map(|index| {
                let offset = records_start.checked_add(index.checked_mul(ANALYSIS_BYTES)?)?;
                decode_analysis(
                    self.payload
                        .get(offset..offset.checked_add(ANALYSIS_BYTES)?)?,
                )
            })
            .collect()
    }
}

pub fn encode_morphology_resource(
    source_digest: [u8; 32],
    entries: &[MecabMorphologyEntry],
    matrix: &MecabConnectionMatrix,
    char_def: &[u8],
    unk_def: &[u8],
) -> Result<Vec<u8>, DataError> {
    if entries.is_empty() {
        return Err(binary_error("morphology entries are empty"));
    }
    if char_def.is_empty() || unk_def.is_empty() {
        return Err(binary_error("unknown-word definitions are empty"));
    }
    let mut grouped = BTreeMap::<String, Vec<MecabMorphologyEntry>>::new();
    for entry in entries {
        if entry.right_id >= matrix.right_contexts() || entry.left_id >= matrix.left_contexts() {
            return Err(binary_error("morphology entry context ID is out of range"));
        }
        grouped
            .entry(entry.surface.clone())
            .or_default()
            .push(entry.clone());
    }
    for analyses in grouped.values_mut() {
        analyses.sort_unstable();
        analyses.dedup();
    }
    let groups = grouped.into_iter().collect::<Vec<_>>();
    let keys = groups
        .iter()
        .enumerate()
        .map(|(group, (surface, _))| {
            Ok((
                surface.as_bytes(),
                u32::try_from(group).map_err(binary_conversion_error)?,
            ))
        })
        .collect::<Result<Vec<_>, DataError>>()?;
    let index = DoubleArrayBuilder::build(&keys)
        .ok_or_else(|| binary_error("failed to build packed Double-Array index"))?;
    let (payload, analysis_count, pos_counts) = encode_payload(&groups)?;
    let matrix_bytes = encode_matrix(matrix);
    let stats = MorphologyResourceStats {
        surface_count: u32::try_from(groups.len()).map_err(binary_conversion_error)?,
        analysis_count,
        pos_counts,
        right_contexts: matrix.right_contexts(),
        left_contexts: matrix.left_contexts(),
    };
    let sections: [&[u8]; SECTION_COUNT] = [&index, &payload, &matrix_bytes, char_def, unk_def];
    let mut output = Vec::with_capacity(
        HEADER_LEN + sections.iter().map(|section| section.len()).sum::<usize>(),
    );
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&SCHEMA_VERSION.to_le_bytes());
    output.push(INDEX_KIND_DOUBLE_ARRAY);
    output.extend_from_slice(&[0; 3]);
    output.extend_from_slice(&source_digest);
    output.extend_from_slice(&stats.surface_count.to_le_bytes());
    output.extend_from_slice(&stats.analysis_count.to_le_bytes());
    for count in stats.pos_counts {
        output.extend_from_slice(&count.to_le_bytes());
    }
    output.extend_from_slice(&u32::from(stats.right_contexts).to_le_bytes());
    output.extend_from_slice(&u32::from(stats.left_contexts).to_le_bytes());
    for section in sections {
        output.extend_from_slice(
            &u64::try_from(section.len())
                .map_err(binary_conversion_error)?
                .to_le_bytes(),
        );
    }
    for section in sections {
        output.extend_from_slice(&sha256(section));
    }
    if output.len() != HEADER_LEN {
        return Err(binary_error("morphology header length is inconsistent"));
    }
    for section in sections {
        output.extend_from_slice(section);
    }
    Ok(output)
}

pub fn decode_morphology_resource<'a>(
    source: &str,
    input: &'a [u8],
    expected_source_digest: &[u8; 32],
) -> Result<DecodedMorphologyResource<'a>, DataError> {
    if input.len() < HEADER_LEN || input.get(..MAGIC.len()) != Some(MAGIC) {
        return Err(resource_error(source, "truncated header or invalid magic"));
    }
    let mut cursor = MAGIC.len();
    let schema = read_u32(input, &mut cursor).map_err(|message| resource_error(source, message))?;
    if schema != SCHEMA_VERSION {
        return Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::MorphologyResourceSchema {
                expected: SCHEMA_VERSION,
                actual: schema,
            },
        ));
    }
    if input[cursor] != INDEX_KIND_DOUBLE_ARRAY || input[cursor + 1..cursor + 4] != [0; 3] {
        return Err(resource_error(
            source,
            "unsupported index kind or reserved bytes",
        ));
    }
    cursor += 4;
    let source_digest =
        read_array::<32>(input, &mut cursor).map_err(|message| resource_error(source, message))?;
    if &source_digest != expected_source_digest {
        return Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::MorphologyResourceSourceMismatch,
        ));
    }
    let surface_count =
        read_u32(input, &mut cursor).map_err(|message| resource_error(source, message))?;
    let analysis_count =
        read_u32(input, &mut cursor).map_err(|message| resource_error(source, message))?;
    let mut pos_counts = [0_u32; POS_COUNT];
    for count in &mut pos_counts {
        *count = read_u32(input, &mut cursor).map_err(|message| resource_error(source, message))?;
    }
    if pos_counts.iter().copied().map(u64::from).sum::<u64>() != u64::from(analysis_count) {
        return Err(resource_error(
            source,
            "POS counts do not equal analysis count",
        ));
    }
    let right_contexts = read_context_count(source, input, &mut cursor, "right")?;
    let left_contexts = read_context_count(source, input, &mut cursor, "left")?;
    let mut lengths = [0_usize; SECTION_COUNT];
    for length in &mut lengths {
        *length = usize::try_from(
            read_u64(input, &mut cursor).map_err(|message| resource_error(source, message))?,
        )
        .map_err(|error| resource_error(source, &error.to_string()))?;
    }
    let mut digests = [[0_u8; 32]; SECTION_COUNT];
    for digest in &mut digests {
        *digest =
            read_array(input, &mut cursor).map_err(|message| resource_error(source, message))?;
    }
    if cursor != HEADER_LEN {
        return Err(resource_error(source, "invalid header length"));
    }
    let sections = split_sections(source, input, cursor, lengths)?;
    for (section, expected) in sections.iter().zip(digests) {
        if sha256(section) != expected {
            return Err(resource_error(source, "section digest mismatch"));
        }
    }
    let stats = MorphologyResourceStats {
        surface_count,
        analysis_count,
        pos_counts,
        right_contexts,
        left_contexts,
    };
    validate_payload(source, sections[1], &stats)?;
    validate_matrix(source, sections[2], &stats)?;
    if sections[0].is_empty() || sections[0].len() % 4 != 0 {
        return Err(resource_error(source, "invalid Double-Array section"));
    }
    if sections[3].is_empty() || sections[4].is_empty() {
        return Err(resource_error(
            source,
            "empty unknown-word definition section",
        ));
    }
    Ok(DecodedMorphologyResource {
        stats,
        index: DoubleArray::new(sections[0]),
        payload: sections[1],
        matrix: sections[2],
        char_def: sections[3],
        unk_def: sections[4],
    })
}

pub fn parse_sha256(value: &str) -> Result<[u8; 32], DataError> {
    if value.len() != 64 {
        return Err(binary_error(
            "SHA-256 must contain 64 hexadecimal characters",
        ));
    }
    let mut digest = [0_u8; 32];
    for (byte, pair) in digest.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let high = hex_digit(pair[0])
            .ok_or_else(|| binary_error("SHA-256 contains a non-hexadecimal character"))?;
        let low = hex_digit(pair[1])
            .ok_or_else(|| binary_error("SHA-256 contains a non-hexadecimal character"))?;
        *byte = high << 4 | low;
    }
    Ok(digest)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_payload(
    groups: &[(String, Vec<MecabMorphologyEntry>)],
) -> Result<(Vec<u8>, u32, [u32; POS_COUNT]), DataError> {
    let analysis_count = u32::try_from(groups.iter().map(|(_, group)| group.len()).sum::<usize>())
        .map_err(binary_conversion_error)?;
    let mut output = Vec::new();
    output.extend_from_slice(
        &u32::try_from(groups.len())
            .map_err(binary_conversion_error)?
            .to_le_bytes(),
    );
    output.extend_from_slice(&analysis_count.to_le_bytes());
    let mut offset = 0_u32;
    output.extend_from_slice(&offset.to_le_bytes());
    for (_, group) in groups {
        offset = offset
            .checked_add(u32::try_from(group.len()).map_err(binary_conversion_error)?)
            .ok_or_else(|| binary_error("analysis offset overflow"))?;
        output.extend_from_slice(&offset.to_le_bytes());
    }
    let mut pos_counts = [0_u32; POS_COUNT];
    for (_, group) in groups {
        for entry in group {
            let pos = usize::from(entry.pos.code());
            pos_counts[pos] = pos_counts[pos]
                .checked_add(1)
                .ok_or_else(|| binary_error("POS count overflow"))?;
            output.push(entry.pos.code());
            output.extend_from_slice(&[0; 3]);
            output.extend_from_slice(&entry.left_id.to_le_bytes());
            output.extend_from_slice(&entry.right_id.to_le_bytes());
            output.extend_from_slice(&entry.word_cost.to_le_bytes());
        }
    }
    Ok((output, analysis_count, pos_counts))
}

fn encode_matrix(matrix: &MecabConnectionMatrix) -> Vec<u8> {
    let mut output = Vec::with_capacity(matrix.costs().len() * 2);
    for cost in matrix.costs() {
        output.extend_from_slice(&cost.to_le_bytes());
    }
    output
}

fn validate_payload(
    source: &str,
    input: &[u8],
    stats: &MorphologyResourceStats,
) -> Result<(), DataError> {
    let offsets = usize::try_from(stats.surface_count)
        .map_err(|error| resource_error(source, &error.to_string()))?
        .checked_add(1)
        .ok_or_else(|| resource_error(source, "payload offset count overflow"))?;
    let records_start = 8_usize
        .checked_add(
            offsets
                .checked_mul(4)
                .ok_or_else(|| resource_error(source, "payload offsets overflow"))?,
        )
        .ok_or_else(|| resource_error(source, "payload header overflow"))?;
    let expected_len = records_start
        .checked_add(
            usize::try_from(stats.analysis_count)
                .map_err(|error| resource_error(source, &error.to_string()))?
                .checked_mul(ANALYSIS_BYTES)
                .ok_or_else(|| resource_error(source, "payload records overflow"))?,
        )
        .ok_or_else(|| resource_error(source, "payload length overflow"))?;
    if input.len() != expected_len
        || read_u32_at(input, 0) != Some(stats.surface_count)
        || read_u32_at(input, 4) != Some(stats.analysis_count)
    {
        return Err(resource_error(source, "payload counts or length mismatch"));
    }
    let mut previous = 0_u32;
    for index in 0..offsets {
        let offset = read_u32_at(input, 8 + index * 4)
            .ok_or_else(|| resource_error(source, "truncated payload offset"))?;
        if offset < previous || offset > stats.analysis_count || (index == 0 && offset != 0) {
            return Err(resource_error(source, "invalid payload offset order"));
        }
        previous = offset;
    }
    if previous != stats.analysis_count {
        return Err(resource_error(source, "final payload offset mismatch"));
    }
    for index in 0..usize::try_from(stats.analysis_count)
        .map_err(|error| resource_error(source, &error.to_string()))?
    {
        let offset = records_start + index * ANALYSIS_BYTES;
        decode_analysis(&input[offset..offset + ANALYSIS_BYTES])
            .ok_or_else(|| resource_error(source, "invalid analysis record"))?;
    }
    Ok(())
}

fn validate_matrix(
    source: &str,
    input: &[u8],
    stats: &MorphologyResourceStats,
) -> Result<(), DataError> {
    let expected = usize::from(stats.right_contexts)
        .checked_mul(usize::from(stats.left_contexts))
        .and_then(|count| count.checked_mul(2))
        .ok_or_else(|| resource_error(source, "connection matrix length overflow"))?;
    if input.len() != expected {
        return Err(resource_error(source, "connection matrix length mismatch"));
    }
    Ok(())
}

fn split_sections<'a>(
    source: &str,
    input: &'a [u8],
    mut cursor: usize,
    lengths: [usize; SECTION_COUNT],
) -> Result<[&'a [u8]; SECTION_COUNT], DataError> {
    let mut sections = [&input[0..0]; SECTION_COUNT];
    for (slot, length) in sections.iter_mut().zip(lengths) {
        let end = cursor
            .checked_add(length)
            .ok_or_else(|| resource_error(source, "section length overflow"))?;
        *slot = input
            .get(cursor..end)
            .ok_or_else(|| resource_error(source, "truncated section"))?;
        cursor = end;
    }
    if cursor != input.len() {
        return Err(resource_error(source, "section lengths do not match file"));
    }
    Ok(sections)
}

fn decode_analysis(input: &[u8]) -> Option<MorphologyAnalysis> {
    if input.len() != ANALYSIS_BYTES || input[1..4] != [0; 3] {
        return None;
    }
    Some(MorphologyAnalysis {
        pos: DataFinePos::from_code(input[0])?,
        left_id: u16::from_le_bytes(input[4..6].try_into().ok()?),
        right_id: u16::from_le_bytes(input[6..8].try_into().ok()?),
        word_cost: i32::from_le_bytes(input[8..12].try_into().ok()?),
    })
}

fn read_context_count(
    source: &str,
    input: &[u8],
    cursor: &mut usize,
    name: &str,
) -> Result<u16, DataError> {
    let count = read_u32(input, cursor).map_err(|message| resource_error(source, message))?;
    u16::try_from(count).map_err(|_| resource_error(source, &format!("{name} context overflow")))
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(read_array(input, cursor)?))
}

fn read_u64(input: &[u8], cursor: &mut usize) -> Result<u64, &'static str> {
    Ok(u64::from_le_bytes(read_array(input, cursor)?))
}

fn read_array<const N: usize>(input: &[u8], cursor: &mut usize) -> Result<[u8; N], &'static str> {
    let end = cursor.checked_add(N).ok_or("header offset overflow")?;
    let bytes = input.get(*cursor..end).ok_or("truncated header field")?;
    *cursor = end;
    bytes.try_into().map_err(|_| "invalid header field")
}

fn read_u32_at(input: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        input.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

fn binary_conversion_error(error: impl ToString) -> DataError {
    binary_error(&error.to_string())
}

fn binary_error(message: &str) -> DataError {
    DataError::new(
        SourceLocation::new("morphology-resource"),
        DataErrorKind::Binary(message.to_owned()),
    )
}

fn resource_error(source: &str, message: &str) -> DataError {
    DataError::new(
        SourceLocation::new(source),
        DataErrorKind::MorphologyResourceCorrupt(message.to_owned()),
    )
}

#[cfg(test)]
mod tests;
