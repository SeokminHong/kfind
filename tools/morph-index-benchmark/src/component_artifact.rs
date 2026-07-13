use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use kfind_data::{MecabConnectionMatrix, MecabSourceMorphologyEntry};
use kfind_morph::{LocalLatticeAnalysis, LocalLatticeResource};
use sha2::{Digest, Sha256};
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::component_payload::{PayloadView, StringTable, encode as encode_payload};

pub use crate::component_payload::CompactComponentAnalysis;

const MAGIC: &[u8; 8] = b"KFCMPLT\0";
const SCHEMA_VERSION: u32 = 1;
const INDEX_KIND_DOUBLE_ARRAY: u8 = 1;
const SECTION_COUNT: usize = 6;
const HEADER_LEN: usize = 304;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct CompactComponentStats {
    pub schema_version: u32,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub pos_counts: BTreeMap<String, u32>,
    pub right_contexts: u16,
    pub left_contexts: u16,
}

pub struct CompactComponentResource<'a> {
    stats: CompactComponentStats,
    index: DoubleArray<&'a [u8]>,
    payload: PayloadView<'a>,
    strings: StringTable<'a>,
    matrix: &'a [u8],
    char_def: &'a [u8],
    unk_def: &'a [u8],
}

impl<'a> CompactComponentResource<'a> {
    pub fn stats(&self) -> &CompactComponentStats {
        &self.stats
    }

    pub fn common_prefixes(
        &self,
        input: &[u8],
        mut emit: impl FnMut(usize, &[CompactComponentAnalysis<'a>]),
    ) {
        for (group, length) in self.index.common_prefix_search(input) {
            if let Some(analyses) = self.payload.group(group, &self.strings) {
                emit(length, &analyses);
            }
        }
    }

    pub fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        if right_id >= self.stats.right_contexts || left_id >= self.stats.left_contexts {
            return None;
        }
        let index = usize::from(right_id)
            .checked_mul(usize::from(self.stats.left_contexts))?
            .checked_add(usize::from(left_id))?;
        let offset = index.checked_mul(2)?;
        Some(i16::from_le_bytes(
            self.matrix
                .get(offset..offset.checked_add(2)?)?
                .try_into()
                .ok()?,
        ))
    }

    pub fn char_def(&self) -> &[u8] {
        self.char_def
    }

    pub fn unk_def(&self) -> &[u8] {
        self.unk_def
    }
}

impl LocalLatticeResource for CompactComponentResource<'_> {
    fn common_prefixes<'a>(
        &'a self,
        input: &[u8],
        emit: &mut dyn FnMut(usize, LocalLatticeAnalysis<'a>),
    ) {
        self.common_prefixes(input, |length, analyses| {
            for analysis in analyses {
                emit(
                    length,
                    LocalLatticeAnalysis {
                        pos: analysis.pos,
                        left_id: analysis.left_id,
                        right_id: analysis.right_id,
                        word_cost: analysis.word_cost,
                    },
                );
            }
        });
    }

    fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        self.connection_cost(right_id, left_id)
    }

    fn right_contexts(&self) -> u16 {
        self.stats.right_contexts
    }

    fn left_contexts(&self) -> u16 {
        self.stats.left_contexts
    }

    fn char_def(&self) -> &[u8] {
        self.char_def
    }

    fn unk_def(&self) -> &[u8] {
        self.unk_def
    }
}

pub fn encode_compact_component_resource(
    source_digest: [u8; 32],
    entries: &[MecabSourceMorphologyEntry],
    matrix: &MecabConnectionMatrix,
    char_def: &[u8],
    unk_def: &[u8],
) -> Result<Vec<u8>> {
    ensure!(!entries.is_empty(), "component entries are empty");
    ensure!(
        !char_def.is_empty() && !unk_def.is_empty(),
        "unknown-word definitions are empty"
    );
    let mut grouped = BTreeMap::<String, Vec<MecabSourceMorphologyEntry>>::new();
    for entry in entries {
        ensure!(
            entry.right_id < matrix.right_contexts() && entry.left_id < matrix.left_contexts(),
            "component entry context ID is out of range"
        );
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
        .map(|(group, (surface, _))| Ok((surface.as_bytes(), u32::try_from(group)?)))
        .collect::<Result<Vec<_>>>()?;
    let index = DoubleArrayBuilder::build(&keys)
        .ok_or_else(|| anyhow::anyhow!("failed to build component Double-Array index"))?;
    let (payload, strings, analysis_count) = encode_payload(&groups)?;
    let matrix_bytes = encode_matrix(matrix);
    let sections: [&[u8]; SECTION_COUNT] =
        [&index, &payload, &strings, &matrix_bytes, char_def, unk_def];
    let mut output = Vec::with_capacity(
        HEADER_LEN + sections.iter().map(|section| section.len()).sum::<usize>(),
    );
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&SCHEMA_VERSION.to_le_bytes());
    output.push(INDEX_KIND_DOUBLE_ARRAY);
    output.extend_from_slice(&[0; 3]);
    output.extend_from_slice(&source_digest);
    output.extend_from_slice(&u32::try_from(groups.len())?.to_le_bytes());
    output.extend_from_slice(&analysis_count.to_le_bytes());
    output.extend_from_slice(&u32::from(matrix.right_contexts()).to_le_bytes());
    output.extend_from_slice(&u32::from(matrix.left_contexts()).to_le_bytes());
    for section in sections {
        output.extend_from_slice(&u64::try_from(section.len())?.to_le_bytes());
    }
    for section in sections {
        output.extend_from_slice(&sha256(section));
    }
    ensure!(
        output.len() == HEADER_LEN,
        "component header length mismatch"
    );
    for section in sections {
        output.extend_from_slice(section);
    }
    Ok(output)
}

pub fn decode_compact_component_resource<'a>(
    input: &'a [u8],
    expected_source_digest: &[u8; 32],
) -> Result<CompactComponentResource<'a>> {
    ensure!(
        input.len() >= HEADER_LEN,
        "component artifact has a truncated header"
    );
    ensure!(
        &input[..MAGIC.len()] == MAGIC,
        "component artifact has invalid magic"
    );
    let mut cursor = MAGIC.len();
    ensure!(
        read_u32(input, &mut cursor)? == SCHEMA_VERSION,
        "component artifact schema mismatch"
    );
    ensure!(
        input.get(cursor).copied() == Some(INDEX_KIND_DOUBLE_ARRAY),
        "component artifact index kind mismatch"
    );
    cursor += 1;
    ensure!(
        input.get(cursor..cursor + 3) == Some(&[0; 3]),
        "component artifact reserved bytes are not zero"
    );
    cursor += 3;
    ensure!(
        read_array::<32>(input, &mut cursor)? == *expected_source_digest,
        "component artifact source digest mismatch"
    );
    let surface_count = read_u32(input, &mut cursor)?;
    let analysis_count = read_u32(input, &mut cursor)?;
    let right_contexts = u16::try_from(read_u32(input, &mut cursor)?)?;
    let left_contexts = u16::try_from(read_u32(input, &mut cursor)?)?;
    ensure!(
        surface_count > 0 && analysis_count > 0 && right_contexts > 0 && left_contexts > 0,
        "component artifact counts are empty"
    );
    let mut lengths = [0_usize; SECTION_COUNT];
    for length in &mut lengths {
        *length = usize::try_from(read_u64(input, &mut cursor)?)?;
    }
    let mut digests = [[0_u8; 32]; SECTION_COUNT];
    for digest in &mut digests {
        *digest = read_array(input, &mut cursor)?;
    }
    ensure!(
        cursor == HEADER_LEN,
        "component artifact header length mismatch"
    );
    let sections = read_sections(input, cursor, &lengths)?;
    for (section, digest) in sections.iter().zip(digests) {
        ensure!(
            sha256(section) == digest,
            "component artifact section digest mismatch"
        );
    }
    ensure!(
        !sections[0].is_empty() && sections[0].len() % 4 == 0,
        "component artifact Double-Array section is invalid"
    );
    ensure!(
        !sections[4].is_empty() && !sections[5].is_empty(),
        "component unknown definitions are empty"
    );
    let matrix_len = usize::from(right_contexts)
        .checked_mul(usize::from(left_contexts))
        .and_then(|count| count.checked_mul(2))
        .ok_or_else(|| anyhow::anyhow!("component matrix length overflow"))?;
    ensure!(
        sections[3].len() == matrix_len,
        "component matrix length mismatch"
    );
    let strings = StringTable::parse(sections[2])?;
    let (payload, pos_counts) = PayloadView::parse(
        sections[1],
        surface_count,
        analysis_count,
        right_contexts,
        left_contexts,
        &strings,
    )?;
    Ok(CompactComponentResource {
        stats: CompactComponentStats {
            schema_version: SCHEMA_VERSION,
            surface_count,
            analysis_count,
            pos_counts,
            right_contexts,
            left_contexts,
        },
        index: DoubleArray::new(sections[0]),
        payload,
        strings,
        matrix: sections[3],
        char_def: sections[4],
        unk_def: sections[5],
    })
}

fn encode_matrix(matrix: &MecabConnectionMatrix) -> Vec<u8> {
    matrix
        .costs()
        .iter()
        .flat_map(|cost| cost.to_le_bytes())
        .collect()
}

fn read_sections<'a>(
    input: &'a [u8],
    start: usize,
    lengths: &[usize; SECTION_COUNT],
) -> Result<[&'a [u8]; SECTION_COUNT]> {
    let mut cursor = start;
    let mut sections = [&input[0..0]; SECTION_COUNT];
    for (section, length) in sections.iter_mut().zip(lengths) {
        let end = cursor
            .checked_add(*length)
            .ok_or_else(|| anyhow::anyhow!("component section length overflow"))?;
        *section = input
            .get(cursor..end)
            .ok_or_else(|| anyhow::anyhow!("component section is truncated"))?;
        cursor = end;
    }
    ensure!(
        cursor == input.len(),
        "component section lengths do not match file"
    );
    Ok(sections)
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32> {
    Ok(u32::from_le_bytes(read_array(input, cursor)?))
}

fn read_u64(input: &[u8], cursor: &mut usize) -> Result<u64> {
    Ok(u64::from_le_bytes(read_array(input, cursor)?))
}

fn read_array<const N: usize>(input: &[u8], cursor: &mut usize) -> Result<[u8; N]> {
    let end = cursor
        .checked_add(N)
        .ok_or_else(|| anyhow::anyhow!("component header offset overflow"))?;
    let value = input
        .get(*cursor..end)
        .ok_or_else(|| anyhow::anyhow!("component header is truncated"))?
        .try_into()?;
    *cursor = end;
    Ok(value)
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use kfind_data::{
        DataFinePos, decode_morphology_resource, encode_morphology_resource,
        parse_mecab_connection_matrix,
    };
    use kfind_morph::{DEFAULT_LATTICE_NODE_LIMIT, evaluate_local_component_paths};

    use super::*;

    #[test]
    fn compact_resource_preserves_component_scoring_fields() {
        let bytes = fixture_resource();
        let resource = decode_compact_component_resource(&bytes, &[7; 32]).unwrap();
        let mut matches = Vec::new();
        resource.common_prefixes("가나다".as_bytes(), |length, analyses| {
            matches.push((length, analyses.to_vec()));
        });

        assert_eq!(resource.stats().surface_count, 2);
        assert_eq!(resource.stats().analysis_count, 2);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[1].0, "가나".len());
        assert_eq!(matches[1].1[0].pos, "NNG+JX");
        assert_eq!(resource.connection_cost(1, 1), Some(4));
        assert_eq!(resource.char_def(), b"char");
        assert_eq!(resource.unk_def(), b"unk");
    }

    #[test]
    fn compact_resource_rejects_schema_source_and_content_mismatches() {
        let bytes = fixture_resource();
        assert!(
            decode_compact_component_resource(&bytes, &[8; 32])
                .err()
                .unwrap()
                .to_string()
                .contains("source digest")
        );

        let mut schema = bytes.clone();
        schema[MAGIC.len()..MAGIC.len() + 4].copy_from_slice(&2_u32.to_le_bytes());
        assert!(
            decode_compact_component_resource(&schema, &[7; 32])
                .err()
                .unwrap()
                .to_string()
                .contains("schema")
        );

        let mut content = bytes;
        *content.last_mut().unwrap() ^= 1;
        assert!(
            decode_compact_component_resource(&content, &[7; 32])
                .err()
                .unwrap()
                .to_string()
                .contains("section digest")
        );
    }

    #[test]
    fn compact_resource_is_lattice_equivalent_to_full_resource() {
        let entries = [
            entry("가", "NNG", 1, 1, 10),
            entry("가나", "NNG+JX", 1, 1, 20),
        ];
        let matrix = parse_mecab_connection_matrix(
            "matrix.def",
            Cursor::new("2 2\n0 0 1\n0 1 2\n1 0 3\n1 1 4\n"),
        )
        .unwrap();
        let char_def = b"DEFAULT 0 1 0\nHANGUL 0 1 2\n0xAC00..0xD7A3 HANGUL\n";
        let unk_def = b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n";
        let full_bytes =
            encode_morphology_resource([7; 32], &entries, &matrix, char_def, unk_def).unwrap();
        let compact_bytes =
            encode_compact_component_resource([7; 32], &entries, &matrix, char_def, unk_def)
                .unwrap();
        let full = decode_morphology_resource("full", &full_bytes, &[7; 32]).unwrap();
        let compact = decode_compact_component_resource(&compact_bytes, &[7; 32]).unwrap();

        let full_report = evaluate_local_component_paths(
            &full,
            "가나",
            0.."가".len(),
            DataFinePos::Nng,
            DEFAULT_LATTICE_NODE_LIMIT,
        )
        .unwrap();
        let compact_report = evaluate_local_component_paths(
            &compact,
            "가나",
            0.."가".len(),
            DataFinePos::Nng,
            DEFAULT_LATTICE_NODE_LIMIT,
        )
        .unwrap();

        assert_eq!(compact_report, full_report);
    }

    fn fixture_resource() -> Vec<u8> {
        let entries = [
            entry("가", "NNG", 1, 1, 10),
            entry("가나", "NNG+JX", 1, 1, 20),
        ];
        let matrix = parse_mecab_connection_matrix(
            "matrix.def",
            Cursor::new("2 2\n0 0 1\n0 1 2\n1 0 3\n1 1 4\n"),
        )
        .unwrap();
        encode_compact_component_resource([7; 32], &entries, &matrix, b"char", b"unk").unwrap()
    }

    fn entry(
        surface: &str,
        pos: &str,
        left_id: u16,
        right_id: u16,
        word_cost: i32,
    ) -> MecabSourceMorphologyEntry {
        MecabSourceMorphologyEntry {
            surface: surface.to_owned(),
            pos: pos.to_owned(),
            left_id,
            right_id,
            word_cost,
            analysis_type: "*".to_owned(),
            start_pos: "*".to_owned(),
            end_pos: "*".to_owned(),
            expression: "*".to_owned(),
        }
    }
}
