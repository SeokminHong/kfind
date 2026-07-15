use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::{
    DataError, DataErrorKind, MecabConnectionMatrix, MecabSourceMorphologyEntry, SourceLocation,
};

mod payload;
mod projection;

use super::{
    HEADER_LEN, INDEX_KIND_DOUBLE_ARRAY, MAGIC, SECTION_COUNT, StringLayout,
    build_conversion_error, build_error, read_array, read_context_count, read_u32, read_u64,
    resource_error, section_ranges, sha256,
};
use payload::{GraphPayloadLayout, encode_graph_payload};
pub use projection::{MorphologyGraphProjectionStats, validate_morphology_graph_projection};

const SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MorphologyGraphExpressionKind {
    Absent,
    SpanAligned,
    Fused,
    Unaligned,
    Invalid,
}

impl MorphologyGraphExpressionKind {
    const fn encode(self) -> u8 {
        match self {
            Self::Absent => 0,
            Self::SpanAligned => 1,
            Self::Fused => 2,
            Self::Unaligned => 3,
            Self::Invalid => 4,
        }
    }

    const fn decode(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Absent),
            1 => Some(Self::SpanAligned),
            2 => Some(Self::Fused),
            3 => Some(Self::Unaligned),
            4 => Some(Self::Invalid),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyGraphComponent<'a> {
    pub surface: &'a str,
    pub surface_id: MorphologyGraphStringId,
    pub pos: &'a str,
    pub pos_id: MorphologyGraphStringId,
    pub span: Option<Range<usize>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyGraphAnalysis<'a> {
    pub surface_id: MorphologyGraphStringId,
    pub pos: &'a str,
    pub pos_id: MorphologyGraphStringId,
    pub start_pos: &'a str,
    pub end_pos: &'a str,
    pub expression_kind: MorphologyGraphExpressionKind,
    pub components: Vec<MorphologyGraphComponent<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MorphologyGraphStringId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyGraphPosClass(u16);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyGraphResourceStats {
    pub schema_version: u32,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub component_count: u32,
    pub transition_count: u32,
    pub pos_counts: BTreeMap<String, u32>,
    pub expression_counts: BTreeMap<MorphologyGraphExpressionKind, u32>,
    pub right_contexts: u16,
    pub left_contexts: u16,
}

#[derive(Clone, Debug)]
struct Sections {
    index: Range<usize>,
    payload: Range<usize>,
    strings: Range<usize>,
    char_def: Range<usize>,
    unk_def: Range<usize>,
}

#[derive(Debug)]
struct TransitionMatrix {
    classes: BTreeMap<String, MorphologyGraphPosClass>,
    words_per_row: usize,
    bits: Box<[u64]>,
}

#[derive(Debug)]
pub struct MorphologyGraphResource {
    bytes: Box<[u8]>,
    stats: MorphologyGraphResourceStats,
    sections: Sections,
    payload: GraphPayloadLayout,
    strings: StringLayout,
    transitions: BTreeSet<(String, String)>,
    transition_matrix: TransitionMatrix,
}

impl MorphologyGraphResource {
    #[must_use]
    pub fn stats(&self) -> &MorphologyGraphResourceStats {
        &self.stats
    }

    #[must_use]
    pub fn into_bytes(self) -> Box<[u8]> {
        self.bytes
    }

    pub fn common_prefixes<'a>(
        &'a self,
        input: &[u8],
        mut emit: impl FnMut(usize, &str, &[MorphologyGraphAnalysis<'a>]),
    ) {
        let index = DoubleArray::new(&self.bytes[self.sections.index.clone()]);
        let payload = &self.bytes[self.sections.payload.clone()];
        let strings = &self.bytes[self.sections.strings.clone()];
        for (group, length) in index.common_prefix_search(input) {
            if let Some((surface, analyses)) =
                self.payload.group(payload, group, strings, &self.strings)
            {
                emit(length, surface, &analyses);
            }
        }
    }

    #[must_use]
    pub fn allows_transition(&self, end_pos: &str, start_pos: &str) -> bool {
        self.transition_class(end_pos)
            .zip(self.transition_class(start_pos))
            .is_some_and(|(end, start)| self.allows_transition_classes(end, start))
    }

    #[must_use]
    pub fn transition_class(&self, pos: &str) -> Option<MorphologyGraphPosClass> {
        self.transition_matrix.classes.get(pos).copied()
    }

    #[must_use]
    pub fn allows_transition_classes(
        &self,
        end: MorphologyGraphPosClass,
        start: MorphologyGraphPosClass,
    ) -> bool {
        let start = usize::from(start.0);
        let Some(word) = usize::from(end.0)
            .checked_mul(self.transition_matrix.words_per_row)
            .and_then(|row| row.checked_add(start / u64::BITS as usize))
            .and_then(|index| self.transition_matrix.bits.get(index))
        else {
            return false;
        };
        word & (1_u64 << (start % u64::BITS as usize)) != 0
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

pub fn encode_morphology_graph_resource(
    source_digest: [u8; 32],
    entries: &[MecabSourceMorphologyEntry],
    matrix: &MecabConnectionMatrix,
    char_def: &[u8],
    unk_def: &[u8],
) -> Result<Vec<u8>, DataError> {
    if entries.is_empty() {
        return Err(build_error("morphology graph entries are empty"));
    }
    if char_def.is_empty() || unk_def.is_empty() {
        return Err(build_error("unknown-word definitions are empty"));
    }
    let mut grouped = BTreeMap::<String, Vec<MecabSourceMorphologyEntry>>::new();
    for entry in entries {
        if entry.right_id >= matrix.right_contexts() || entry.left_id >= matrix.left_contexts() {
            return Err(build_error(
                "morphology graph entry context ID is out of range",
            ));
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
        .ok_or_else(|| build_error("failed to build morphology graph Double-Array index"))?;
    let encoded = encode_graph_payload(&groups)?;
    let sections: [&[u8]; SECTION_COUNT] = [
        &index,
        &encoded.bytes,
        &encoded.strings,
        &[],
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
        return Err(build_error(
            "morphology graph header length is inconsistent",
        ));
    }
    for section in sections {
        output.extend_from_slice(section);
    }
    Ok(output)
}

pub fn decode_morphology_graph_resource(
    source: &str,
    input: Vec<u8>,
    expected_source_digest: &[u8; 32],
) -> Result<MorphologyGraphResource, DataError> {
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
    if !ranges[3].is_empty() {
        return Err(resource_error(
            source,
            "structural graph contains a scoring matrix",
        ));
    }
    let strings = StringLayout::parse(source, &bytes[sections.strings.clone()])?;
    let (payload, payload_stats) = GraphPayloadLayout::parse(
        source,
        &bytes[sections.payload.clone()],
        surface_count,
        analysis_count,
        &bytes[sections.strings.clone()],
        &strings,
    )?;
    validate_index(
        source,
        &bytes[sections.index.clone()],
        &bytes[sections.payload.clone()],
        &bytes[sections.strings.clone()],
        &strings,
        &payload,
        surface_count,
    )?;
    let transitions = payload_stats.transitions;
    let transition_matrix = build_transition_matrix(source, &transitions)?;
    Ok(MorphologyGraphResource {
        bytes,
        stats: MorphologyGraphResourceStats {
            schema_version: SCHEMA_VERSION,
            surface_count,
            analysis_count,
            component_count: payload_stats.component_count,
            transition_count: u32::try_from(transitions.len())
                .map_err(|error| resource_error(source, &error.to_string()))?,
            pos_counts: payload_stats.pos_counts,
            expression_counts: payload_stats.expression_counts,
            right_contexts,
            left_contexts,
        },
        sections,
        payload,
        strings,
        transitions,
        transition_matrix,
    })
}

fn build_transition_matrix(
    source: &str,
    transitions: &BTreeSet<(String, String)>,
) -> Result<TransitionMatrix, DataError> {
    let positions = transitions
        .iter()
        .flat_map(|(end, start)| [end.clone(), start.clone()])
        .collect::<BTreeSet<_>>();
    let mut classes = BTreeMap::new();
    for (index, pos) in positions.into_iter().enumerate() {
        let class = u16::try_from(index)
            .map(MorphologyGraphPosClass)
            .map_err(|error| resource_error(source, &error.to_string()))?;
        classes.insert(pos, class);
    }
    let words_per_row = classes.len().div_ceil(u64::BITS as usize);
    let matrix_len = classes
        .len()
        .checked_mul(words_per_row)
        .ok_or_else(|| resource_error(source, "transition matrix size overflow"))?;
    let mut matrix = vec![0_u64; matrix_len];
    for (end, start) in transitions {
        let end = usize::from(classes[end].0);
        let start = usize::from(classes[start].0);
        matrix[end * words_per_row + start / u64::BITS as usize] |=
            1_u64 << (start % u64::BITS as usize);
    }
    Ok(TransitionMatrix {
        classes,
        words_per_row,
        bits: matrix.into_boxed_slice(),
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_index(
    source: &str,
    index_bytes: &[u8],
    payload_bytes: &[u8],
    string_bytes: &[u8],
    strings: &StringLayout,
    payload: &GraphPayloadLayout,
    surface_count: u32,
) -> Result<(), DataError> {
    let index = DoubleArray::new(index_bytes);
    for group in 0..surface_count {
        let surface = payload
            .surface(payload_bytes, group, string_bytes, strings)
            .ok_or_else(|| resource_error(source, "invalid graph surface"))?;
        if index.exact_match_search(surface.as_bytes()) != Some(group) {
            return Err(resource_error(source, "surface index and payload mismatch"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
