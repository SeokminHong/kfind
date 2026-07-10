use std::cmp::Ordering;

use crate::lexicon::{DataFinePos, LexiconData};
use crate::validation::require_nfc;
use crate::{DataError, DataErrorKind, SourceLocation};

const MAGIC: &[u8; 8] = b"KFPOS\0\x01\0";
const MAX_ENTRY_COUNT: u32 = 10_000_000;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PosLexiconEntry {
    pub lemma: String,
    pub pos: DataFinePos,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedPosLexicon {
    entries: Box<[PosLexiconEntry]>,
}

pub fn collect_pos_entries(lexicon: &LexiconData) -> Vec<PosLexiconEntry> {
    let mut entries = lexicon
        .predicates
        .iter()
        .map(|record| PosLexiconEntry {
            lemma: record.lemma.clone(),
            pos: record.pos,
        })
        .chain(lexicon.nominals.iter().map(|record| PosLexiconEntry {
            lemma: record.lemma.clone(),
            pos: record.pos,
        }))
        .chain(lexicon.modifiers.iter().map(|record| PosLexiconEntry {
            lemma: record.lemma.clone(),
            pos: record.pos,
        }))
        .chain(lexicon.particles.iter().flat_map(|record| {
            record
                .variants
                .iter()
                .cloned()
                .map(|lemma| PosLexiconEntry {
                    lemma,
                    pos: record.pos,
                })
        }))
        .collect::<Vec<_>>();
    entries.sort_unstable();
    entries.dedup();
    entries
}

impl DecodedPosLexicon {
    pub fn entries(&self) -> &[PosLexiconEntry] {
        &self.entries
    }

    pub fn lookup(&self, lemma: &str) -> &[PosLexiconEntry] {
        let start = self
            .entries
            .partition_point(|entry| entry.lemma.as_str() < lemma);
        let end =
            self.entries[start..].partition_point(|entry| entry.lemma.as_str() == lemma) + start;
        &self.entries[start..end]
    }
}

pub fn encode_pos_lexicon(entries: &[PosLexiconEntry]) -> Result<Vec<u8>, DataError> {
    let mut entries = entries.to_vec();
    for entry in &entries {
        require_nfc("POS lexicon encoder", None, "lemma", &entry.lemma)?;
        if entry.lemma.is_empty() {
            return Err(binary_error("표제어가 비어 있습니다"));
        }
    }
    entries.sort_unstable();
    entries.dedup();
    let count = u32::try_from(entries.len())
        .map_err(|_| binary_error("entry 수가 u32 범위를 초과합니다"))?;
    if count > MAX_ENTRY_COUNT {
        return Err(binary_error("entry 수 상한을 초과합니다"));
    }

    let mut output = Vec::with_capacity(MAGIC.len() + 4 + entries.len() * 8);
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&count.to_le_bytes());
    let mut previous = "";
    for entry in &entries {
        let prefix = common_char_boundary_prefix(previous, &entry.lemma);
        let suffix = &entry.lemma.as_bytes()[prefix..];
        write_varint(&mut output, prefix as u32);
        write_varint(
            &mut output,
            u32::try_from(suffix.len()).map_err(|_| binary_error("표제어가 너무 깁니다"))?,
        );
        output.extend_from_slice(suffix);
        output.push(entry.pos.code());
        previous = &entry.lemma;
    }
    Ok(output)
}

pub fn decode_pos_lexicon(input: &[u8]) -> Result<DecodedPosLexicon, DataError> {
    if input.len() < MAGIC.len() + 4 || &input[..MAGIC.len()] != MAGIC {
        return Err(binary_error(
            "magic 또는 format version이 올바르지 않습니다",
        ));
    }
    let mut cursor = MAGIC.len();
    let count = read_u32(input, &mut cursor)?;
    if count > MAX_ENTRY_COUNT {
        return Err(binary_error("entry 수 상한을 초과합니다"));
    }
    let mut entries = Vec::<PosLexiconEntry>::with_capacity(count as usize);
    let mut previous = String::new();
    for _ in 0..count {
        let prefix = read_varint(input, &mut cursor)? as usize;
        let suffix_len = read_varint(input, &mut cursor)? as usize;
        if prefix > previous.len() || !previous.is_char_boundary(prefix) {
            return Err(binary_error(
                "prefix 길이가 이전 표제어의 문자 경계가 아닙니다",
            ));
        }
        let suffix_end = cursor
            .checked_add(suffix_len)
            .filter(|end| *end < input.len())
            .ok_or_else(|| binary_error("suffix가 binary 범위를 벗어납니다"))?;
        let mut lemma_bytes = previous.as_bytes()[..prefix].to_vec();
        lemma_bytes.extend_from_slice(&input[cursor..suffix_end]);
        cursor = suffix_end;
        let pos = DataFinePos::from_code(input[cursor])
            .ok_or_else(|| binary_error("알 수 없는 POS code입니다"))?;
        cursor += 1;
        let lemma = String::from_utf8(lemma_bytes)
            .map_err(|_| binary_error("표제어가 유효한 UTF-8이 아닙니다"))?;
        require_nfc("POS lexicon decoder", None, "lemma", &lemma)?;
        if lemma.is_empty() {
            return Err(binary_error("표제어가 비어 있습니다"));
        }
        let entry = PosLexiconEntry { lemma, pos };
        if entries
            .last()
            .is_some_and(|previous_entry| previous_entry.cmp(&entry) != Ordering::Less)
        {
            return Err(binary_error("entry가 엄격한 정렬 순서가 아닙니다"));
        }
        previous = entry.lemma.clone();
        entries.push(entry);
    }
    if cursor != input.len() {
        return Err(binary_error("마지막 entry 뒤에 불필요한 바이트가 있습니다"));
    }
    Ok(DecodedPosLexicon {
        entries: entries.into_boxed_slice(),
    })
}

fn common_char_boundary_prefix(left: &str, right: &str) -> usize {
    let mut length = left
        .bytes()
        .zip(right.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while !right.is_char_boundary(length) {
        length -= 1;
    }
    length
}

fn write_varint(output: &mut Vec<u8>, mut value: u32) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u32, DataError> {
    let mut value = 0_u32;
    for shift in (0..=28).step_by(7) {
        let byte = *input
            .get(*cursor)
            .ok_or_else(|| binary_error("varint가 중간에 끝났습니다"))?;
        *cursor += 1;
        if shift == 28 && byte > 0x0f {
            return Err(binary_error("varint가 u32 범위를 초과합니다"));
        }
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(binary_error("varint가 너무 깁니다"))
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32, DataError> {
    let end = cursor
        .checked_add(4)
        .filter(|end| *end <= input.len())
        .ok_or_else(|| binary_error("u32가 중간에 끝났습니다"))?;
    let bytes = input[*cursor..end]
        .try_into()
        .map_err(|_| binary_error("u32를 읽을 수 없습니다"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes))
}

fn binary_error(message: &str) -> DataError {
    DataError::new(
        SourceLocation::new("POS lexicon binary"),
        DataErrorKind::Binary(message.to_owned()),
    )
}
