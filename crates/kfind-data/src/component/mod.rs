use std::collections::BTreeMap;
use std::ops::Range;

use sha2::{Digest, Sha256};
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::{
    DataError, DataErrorKind, MecabConnectionMatrix, MecabSourceMorphologyEntry, SourceLocation,
};

mod graph;
mod payload;

pub use graph::{
    MorphologyGraphAnalysis, MorphologyGraphComponent, MorphologyGraphExpressionKind,
    MorphologyGraphPosClass, MorphologyGraphProjectionStats, MorphologyGraphResource,
    MorphologyGraphResourceStats, decode_morphology_graph_resource,
    encode_morphology_graph_resource, validate_morphology_graph_projection,
};
use payload::{PayloadLayout, StringLayout};

const MAGIC: &[u8; 8] = b"KFCMPLT\0";
const SCHEMA_VERSION: u32 = 1;
const INDEX_KIND_DOUBLE_ARRAY: u8 = 1;
const SECTION_COUNT: usize = 6;
const HEADER_LEN: usize = 304;

pub const COMPONENT_RESOURCE_SOURCE_DIGEST: [u8; 32] = [
    0xfd, 0x62, 0xd3, 0xd6, 0xd8, 0xfa, 0x85, 0x14, 0x55, 0x28, 0x06, 0x5f, 0xab, 0xad, 0x4d, 0x7c,
    0xb2, 0x0f, 0x6b, 0x22, 0x01, 0xe7, 0x1b, 0xe4, 0x08, 0x1a, 0x4e, 0x97, 0x01, 0xa5, 0xb3, 0x30,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentAnalysis<'a> {
    pub pos: &'a str,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentResourceStats {
    pub schema_version: u32,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub pos_counts: BTreeMap<String, u32>,
    pub right_contexts: u16,
    pub left_contexts: u16,
}

#[derive(Clone, Debug)]
struct Sections {
    index: Range<usize>,
    payload: Range<usize>,
    strings: Range<usize>,
    matrix: Range<usize>,
    char_def: Range<usize>,
    unk_def: Range<usize>,
}

#[derive(Debug)]
pub struct ComponentResource {
    bytes: Box<[u8]>,
    stats: ComponentResourceStats,
    sections: Sections,
    payload: PayloadLayout,
    strings: StringLayout,
}

impl ComponentResource {
    #[must_use]
    pub fn stats(&self) -> &ComponentResourceStats {
        &self.stats
    }

    #[must_use]
    pub fn into_bytes(self) -> Box<[u8]> {
        self.bytes
    }

    pub fn common_prefixes<'a>(
        &'a self,
        input: &[u8],
        mut emit: impl FnMut(usize, &[ComponentAnalysis<'a>]),
    ) {
        let index = DoubleArray::new(&self.bytes[self.sections.index.clone()]);
        let payload = &self.bytes[self.sections.payload.clone()];
        let strings = &self.bytes[self.sections.strings.clone()];
        for (group, length) in index.common_prefix_search(input) {
            if let Some(analyses) = self.payload.group(payload, group, strings, &self.strings) {
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
        let offset = self
            .sections
            .matrix
            .start
            .checked_add(index.checked_mul(2)?)?;
        let bytes = self.bytes.get(offset..offset.checked_add(2)?)?;
        Some(i16::from_le_bytes(bytes.try_into().ok()?))
    }

    #[must_use]
    pub fn right_contexts(&self) -> u16 {
        self.stats.right_contexts
    }

    #[must_use]
    pub fn left_contexts(&self) -> u16 {
        self.stats.left_contexts
    }

    #[must_use]
    pub fn char_def(&self) -> &[u8] {
        &self.bytes[self.sections.char_def.clone()]
    }

    #[must_use]
    pub fn unk_def(&self) -> &[u8] {
        &self.bytes[self.sections.unk_def.clone()]
    }
}

pub fn encode_component_resource(
    source_digest: [u8; 32],
    entries: &[MecabSourceMorphologyEntry],
    matrix: &MecabConnectionMatrix,
    char_def: &[u8],
    unk_def: &[u8],
) -> Result<Vec<u8>, DataError> {
    if entries.is_empty() {
        return Err(build_error("component entries are empty"));
    }
    if char_def.is_empty() || unk_def.is_empty() {
        return Err(build_error("unknown-word definitions are empty"));
    }
    let mut grouped = BTreeMap::<String, Vec<MecabSourceMorphologyEntry>>::new();
    for entry in entries {
        if entry.right_id >= matrix.right_contexts() || entry.left_id >= matrix.left_contexts() {
            return Err(build_error("component entry context ID is out of range"));
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
                u32::try_from(group).map_err(build_conversion_error)?,
            ))
        })
        .collect::<Result<Vec<_>, DataError>>()?;
    let index = DoubleArrayBuilder::build(&keys)
        .ok_or_else(|| build_error("failed to build component Double-Array index"))?;
    let encoded = payload::encode(&groups)?;
    let matrix_bytes = encode_matrix(matrix);
    let sections: [&[u8]; SECTION_COUNT] = [
        &index,
        &encoded.bytes,
        &encoded.strings,
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
    output.extend_from_slice(
        &u32::try_from(groups.len())
            .map_err(build_conversion_error)?
            .to_le_bytes(),
    );
    output.extend_from_slice(&encoded.analysis_count.to_le_bytes());
    output.extend_from_slice(&u32::from(matrix.right_contexts()).to_le_bytes());
    output.extend_from_slice(&u32::from(matrix.left_contexts()).to_le_bytes());
    for section in sections {
        output.extend_from_slice(
            &u64::try_from(section.len())
                .map_err(build_conversion_error)?
                .to_le_bytes(),
        );
    }
    for section in sections {
        output.extend_from_slice(&sha256(section));
    }
    if output.len() != HEADER_LEN {
        return Err(build_error("component header length is inconsistent"));
    }
    for section in sections {
        output.extend_from_slice(section);
    }
    Ok(output)
}

pub fn decode_component_resource(
    source: &str,
    input: Vec<u8>,
    expected_source_digest: &[u8; 32],
) -> Result<ComponentResource, DataError> {
    let bytes = input.into_boxed_slice();
    if bytes.len() < HEADER_LEN || bytes.get(..MAGIC.len()) != Some(MAGIC) {
        return Err(resource_error(source, "truncated header or invalid magic"));
    }
    let mut cursor = MAGIC.len();
    let schema =
        read_u32(&bytes, &mut cursor).map_err(|message| resource_error(source, message))?;
    if schema != SCHEMA_VERSION {
        return Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::ComponentResourceSchema {
                expected: SCHEMA_VERSION,
                actual: schema,
            },
        ));
    }
    if bytes[cursor] != INDEX_KIND_DOUBLE_ARRAY || bytes[cursor + 1..cursor + 4] != [0; 3] {
        return Err(resource_error(
            source,
            "unsupported index kind or reserved bytes",
        ));
    }
    cursor += 4;
    let source_digest =
        read_array::<32>(&bytes, &mut cursor).map_err(|message| resource_error(source, message))?;
    if &source_digest != expected_source_digest {
        return Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::ComponentResourceSourceMismatch,
        ));
    }
    let surface_count =
        read_u32(&bytes, &mut cursor).map_err(|message| resource_error(source, message))?;
    let analysis_count =
        read_u32(&bytes, &mut cursor).map_err(|message| resource_error(source, message))?;
    let right_contexts = read_context_count(source, &bytes, &mut cursor, "right")?;
    let left_contexts = read_context_count(source, &bytes, &mut cursor, "left")?;
    if surface_count == 0 || analysis_count == 0 || right_contexts == 0 || left_contexts == 0 {
        return Err(resource_error(source, "empty resource counts"));
    }
    let mut lengths = [0_usize; SECTION_COUNT];
    for length in &mut lengths {
        *length = usize::try_from(
            read_u64(&bytes, &mut cursor).map_err(|message| resource_error(source, message))?,
        )
        .map_err(|error| resource_error(source, &error.to_string()))?;
    }
    let mut digests = [[0_u8; 32]; SECTION_COUNT];
    for digest in &mut digests {
        *digest =
            read_array(&bytes, &mut cursor).map_err(|message| resource_error(source, message))?;
    }
    if cursor != HEADER_LEN {
        return Err(resource_error(source, "invalid header length"));
    }
    let ranges = section_ranges(source, bytes.len(), cursor, lengths)?;
    for (range, expected) in ranges.iter().zip(digests) {
        if sha256(&bytes[range.clone()]) != expected {
            return Err(resource_error(source, "section digest mismatch"));
        }
    }
    let sections = Sections {
        index: ranges[0].clone(),
        payload: ranges[1].clone(),
        strings: ranges[2].clone(),
        matrix: ranges[3].clone(),
        char_def: ranges[4].clone(),
        unk_def: ranges[5].clone(),
    };
    if sections.index.is_empty() || !sections.index.len().is_multiple_of(4) {
        return Err(resource_error(source, "invalid Double-Array section"));
    }
    if sections.char_def.is_empty() || sections.unk_def.is_empty() {
        return Err(resource_error(
            source,
            "empty unknown-word definition section",
        ));
    }
    let expected_matrix_len = usize::from(right_contexts)
        .checked_mul(usize::from(left_contexts))
        .and_then(|count| count.checked_mul(2))
        .ok_or_else(|| resource_error(source, "connection matrix length overflow"))?;
    if sections.matrix.len() != expected_matrix_len {
        return Err(resource_error(source, "connection matrix length mismatch"));
    }
    let strings = StringLayout::parse(source, &bytes[sections.strings.clone()])?;
    let (payload, pos_counts) = PayloadLayout::parse(
        source,
        &bytes[sections.payload.clone()],
        surface_count,
        analysis_count,
        right_contexts,
        left_contexts,
        &bytes[sections.strings.clone()],
        &strings,
    )?;
    Ok(ComponentResource {
        bytes,
        stats: ComponentResourceStats {
            schema_version: SCHEMA_VERSION,
            surface_count,
            analysis_count,
            pos_counts,
            right_contexts,
            left_contexts,
        },
        sections,
        payload,
        strings,
    })
}

fn encode_matrix(matrix: &MecabConnectionMatrix) -> Vec<u8> {
    matrix
        .costs()
        .iter()
        .flat_map(|cost| cost.to_le_bytes())
        .collect()
}

fn section_ranges(
    source: &str,
    input_len: usize,
    mut cursor: usize,
    lengths: [usize; SECTION_COUNT],
) -> Result<[Range<usize>; SECTION_COUNT], DataError> {
    let mut ranges = Vec::with_capacity(SECTION_COUNT);
    for length in lengths {
        let end = cursor
            .checked_add(length)
            .ok_or_else(|| resource_error(source, "section length overflow"))?;
        if end > input_len {
            return Err(resource_error(source, "truncated section"));
        }
        ranges.push(cursor..end);
        cursor = end;
    }
    if cursor != input_len {
        return Err(resource_error(source, "section lengths do not match file"));
    }
    ranges
        .try_into()
        .map_err(|_| resource_error(source, "invalid section count"))
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

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

fn build_conversion_error(error: impl ToString) -> DataError {
    build_error(&error.to_string())
}

fn build_error(message: &str) -> DataError {
    DataError::new(
        SourceLocation::new("component-resource"),
        DataErrorKind::ComponentResourceBuild(message.to_owned()),
    )
}

fn resource_error(source: &str, message: &str) -> DataError {
    DataError::new(
        SourceLocation::new(source),
        DataErrorKind::ComponentResourceCorrupt(message.to_owned()),
    )
}

#[cfg(test)]
mod tests;
