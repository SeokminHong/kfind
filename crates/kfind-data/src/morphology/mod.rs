use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::{
    DataError, DataErrorKind, MecabConnectionMatrix, MecabSourceMorphologyEntry, SourceLocation,
};

mod payload;

use payload::{PayloadView, StringTable};

const MAGIC: &[u8; 8] = b"KFMORPH\0";
const SCHEMA_VERSION: u32 = 3;
const INDEX_KIND_DOUBLE_ARRAY: u8 = 1;
const HEADER_LEN: usize = 304;
const SECTION_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyAnalysis<'a> {
    pub pos: &'a str,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
    pub analysis_type: &'a str,
    pub start_pos: &'a str,
    pub end_pos: &'a str,
    pub expression: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyResourceStats {
    pub surface_count: u32,
    pub analysis_count: u32,
    pub pos_counts: BTreeMap<String, u32>,
    pub right_contexts: u16,
    pub left_contexts: u16,
}

pub struct DecodedMorphologyResource<'a> {
    stats: MorphologyResourceStats,
    index: DoubleArray<&'a [u8]>,
    payload: PayloadView<'a>,
    strings: StringTable<'a>,
    matrix: &'a [u8],
    char_def: &'a [u8],
    unk_def: &'a [u8],
}

impl<'a> DecodedMorphologyResource<'a> {
    #[must_use]
    pub fn stats(&self) -> &MorphologyResourceStats {
        &self.stats
    }

    pub fn common_prefixes(
        &self,
        input: &[u8],
        mut emit: impl FnMut(usize, &[MorphologyAnalysis<'a>]),
    ) {
        for (group, length) in self.index.common_prefix_search(input) {
            if let Some(analyses) = self.payload.group(group, &self.strings) {
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
}

pub fn encode_morphology_resource(
    source_digest: [u8; 32],
    entries: &[MecabSourceMorphologyEntry],
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
    let mut grouped = BTreeMap::<String, Vec<MecabSourceMorphologyEntry>>::new();
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
    let payload = payload::encode(&groups)?;
    let matrix_bytes = encode_matrix(matrix);
    let stats = MorphologyResourceStats {
        surface_count: u32::try_from(groups.len()).map_err(binary_conversion_error)?,
        analysis_count: payload.analysis_count,
        pos_counts: BTreeMap::new(),
        right_contexts: matrix.right_contexts(),
        left_contexts: matrix.left_contexts(),
    };
    let sections: [&[u8]; SECTION_COUNT] = [
        &index,
        &payload.bytes,
        &payload.strings,
        &matrix_bytes,
        char_def,
        unk_def,
    ];
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
    let strings = StringTable::parse(source, sections[2])?;
    let (payload, pos_counts) = PayloadView::parse(
        source,
        sections[1],
        surface_count,
        analysis_count,
        right_contexts,
        left_contexts,
        &strings,
    )?;
    let stats = MorphologyResourceStats {
        surface_count,
        analysis_count,
        pos_counts,
        right_contexts,
        left_contexts,
    };
    validate_matrix(source, sections[3], &stats)?;
    if sections[0].is_empty() || sections[0].len() % 4 != 0 {
        return Err(resource_error(source, "invalid Double-Array section"));
    }
    if sections[4].is_empty() || sections[5].is_empty() {
        return Err(resource_error(
            source,
            "empty unknown-word definition section",
        ));
    }
    Ok(DecodedMorphologyResource {
        stats,
        index: DoubleArray::new(sections[0]),
        payload,
        strings,
        matrix: sections[3],
        char_def: sections[4],
        unk_def: sections[5],
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
        *byte = (high << 4) | low;
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

fn encode_matrix(matrix: &MecabConnectionMatrix) -> Vec<u8> {
    let mut output = Vec::with_capacity(matrix.costs().len() * 2);
    for cost in matrix.costs() {
        output.extend_from_slice(&cost.to_le_bytes());
    }
    output
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

pub(super) fn read_u32_at(input: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        input.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

pub(super) fn binary_conversion_error(error: impl ToString) -> DataError {
    binary_error(&error.to_string())
}

pub(super) fn binary_error(message: &str) -> DataError {
    DataError::new(
        SourceLocation::new("morphology-resource"),
        DataErrorKind::Binary(message.to_owned()),
    )
}

pub(super) fn resource_error(source: &str, message: &str) -> DataError {
    DataError::new(
        SourceLocation::new(source),
        DataErrorKind::MorphologyResourceCorrupt(message.to_owned()),
    )
}

#[cfg(test)]
mod tests;
