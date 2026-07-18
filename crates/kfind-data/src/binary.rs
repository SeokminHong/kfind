use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::sync::OnceLock;

use serde::Serialize;

use crate::lexicon::{DataFinePos, LexiconData};
use crate::validation::require_nfc;
use crate::{DataError, DataErrorKind, SourceLocation};

const MAGIC: &[u8; 8] = b"KFPOS\0\x01\0";
const MAX_ENTRY_COUNT: u32 = 1_000_000;
const MAX_BINARY_BYTES: usize = 128 * 1024 * 1024;
const MAX_DECODED_LEMMA_BYTES: usize = 64 * 1024 * 1024;
const MIN_ENCODED_ENTRY_BYTES: usize = 3;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PosLexiconEntry {
    pub lemma: String,
    pub pos: DataFinePos,
}

pub struct DecodedPosLexicon {
    lemma_bytes: Box<str>,
    entries: Box<[PackedPosLexiconEntry]>,
    materialized_entries: OnceLock<Box<[PosLexiconEntry]>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackedPosLexiconEntry {
    lemma_start: u32,
    lemma_len: u32,
    pos: DataFinePos,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PosLexiconStats {
    pub schema_version: u32,
    pub entry_count: usize,
    pub unique_lemma_count: usize,
    pub predicate_entry_count: usize,
    pub predicate_lemma_count: usize,
    pub pos_conflict_lemma_count: usize,
    pub predicate_pos_conflict_lemma_count: usize,
    pub entries_by_pos: BTreeMap<String, usize>,
}

/// Validated input for the POS-only binary artifact.
///
/// This artifact answers coarse lexicon lookup only. Predicate alternations,
/// flags, overrides, and duplicate analyses remain in [`LexiconData`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedPosLexicon {
    entries: Box<[PosLexiconEntry]>,
}

impl ApprovedPosLexicon {
    pub fn entries(&self) -> &[PosLexiconEntry] {
        &self.entries
    }

    pub(crate) fn from_entries(mut entries: Vec<PosLexiconEntry>) -> Self {
        entries.sort_unstable();
        entries.dedup();
        Self {
            entries: entries.into_boxed_slice(),
        }
    }
}

/// Collects the POS index from a validated core lexicon.
///
/// Callers must retain `lexicon` as the source of full predicate analyses.
pub fn collect_pos_entries(lexicon: &LexiconData) -> ApprovedPosLexicon {
    let entries = lexicon
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
    ApprovedPosLexicon::from_entries(entries)
}

impl DecodedPosLexicon {
    pub fn entries(&self) -> &[PosLexiconEntry] {
        self.materialized_entries.get_or_init(|| {
            self.entries
                .iter()
                .map(|entry| PosLexiconEntry {
                    lemma: self.lemma(entry).to_owned(),
                    pos: entry.pos,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
    }

    pub fn lookup(&self, lemma: &str) -> &[PosLexiconEntry] {
        let entries = self.entries();
        let start = entries.partition_point(|entry| entry.lemma.as_str() < lemma);
        let end = entries[start..].partition_point(|entry| entry.lemma.as_str() == lemma) + start;
        &entries[start..end]
    }

    pub fn lookup_fine_pos(&self, lemma: &str) -> impl ExactSizeIterator<Item = DataFinePos> + '_ {
        let range = self.lookup_range(lemma);
        self.entries[range].iter().map(|entry| entry.pos)
    }

    pub fn stats(&self) -> PosLexiconStats {
        let mut entries_by_pos = BTreeMap::new();
        let mut unique_lemma_count = 0;
        let mut predicate_entry_count = 0;
        let mut predicate_lemma_count = 0;
        let mut pos_conflict_lemma_count = 0;
        let mut predicate_pos_conflict_lemma_count = 0;
        let mut at = 0;
        while at < self.entries.len() {
            let lemma = self.lemma(&self.entries[at]);
            let end = self.entries[at..].partition_point(|entry| self.lemma(entry) == lemma) + at;
            let analyses = &self.entries[at..end];
            unique_lemma_count += 1;
            pos_conflict_lemma_count += usize::from(analyses.len() > 1);
            let predicate_analyses = analyses
                .iter()
                .filter(|entry| entry.pos.is_predicate())
                .count();
            predicate_entry_count += predicate_analyses;
            predicate_lemma_count += usize::from(predicate_analyses > 0);
            predicate_pos_conflict_lemma_count += usize::from(predicate_analyses > 1);
            for entry in analyses {
                *entries_by_pos
                    .entry(entry.pos.as_str().to_owned())
                    .or_default() += 1;
            }
            at = end;
        }
        PosLexiconStats {
            schema_version: 1,
            entry_count: self.entries.len(),
            unique_lemma_count,
            predicate_entry_count,
            predicate_lemma_count,
            pos_conflict_lemma_count,
            predicate_pos_conflict_lemma_count,
            entries_by_pos,
        }
    }

    fn lookup_range(&self, lemma: &str) -> Range<usize> {
        let start = self
            .entries
            .partition_point(|entry| self.lemma(entry) < lemma);
        let end = self.entries[start..].partition_point(|entry| self.lemma(entry) == lemma) + start;
        start..end
    }

    fn lemma(&self, entry: &PackedPosLexiconEntry) -> &str {
        let start = entry.lemma_start as usize;
        let end = start + entry.lemma_len as usize;
        &self.lemma_bytes[start..end]
    }
}

impl Clone for DecodedPosLexicon {
    fn clone(&self) -> Self {
        let materialized_entries = OnceLock::new();
        if let Some(entries) = self.materialized_entries.get() {
            materialized_entries
                .set(entries.clone())
                .expect("a new OnceLock must be empty");
        }
        Self {
            lemma_bytes: self.lemma_bytes.clone(),
            entries: self.entries.clone(),
            materialized_entries,
        }
    }
}

impl fmt::Debug for DecodedPosLexicon {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedPosLexicon")
            .field("lemma_bytes", &self.lemma_bytes.len())
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl PartialEq for DecodedPosLexicon {
    fn eq(&self, other: &Self) -> bool {
        self.lemma_bytes == other.lemma_bytes && self.entries == other.entries
    }
}

impl Eq for DecodedPosLexicon {}

pub fn encode_pos_lexicon(lexicon: &ApprovedPosLexicon) -> Result<Vec<u8>, DataError> {
    let entries = lexicon.entries();
    let mut decoded_lemma_bytes = 0_usize;
    for entry in entries {
        require_nfc("POS lexicon encoder", None, "lemma", &entry.lemma)?;
        if entry.lemma.is_empty() {
            return Err(binary_error("표제어가 비어 있습니다"));
        }
        decoded_lemma_bytes = checked_decoded_bytes(decoded_lemma_bytes, entry.lemma.len())?;
    }
    let count = u32::try_from(entries.len())
        .map_err(|_| binary_error("entry 수가 u32 범위를 초과합니다"))?;
    if count > MAX_ENTRY_COUNT {
        return Err(binary_error("entry 수 상한을 초과합니다"));
    }

    let mut output = Vec::with_capacity(MAGIC.len() + 4 + entries.len() * 8);
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&count.to_le_bytes());
    let mut previous = "";
    for entry in entries {
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
    if output.len() > MAX_BINARY_BYTES {
        return Err(binary_error("binary 파일 크기 상한을 초과합니다"));
    }
    Ok(output)
}

pub fn decode_pos_lexicon(input: &[u8]) -> Result<DecodedPosLexicon, DataError> {
    if input.len() > MAX_BINARY_BYTES {
        return Err(binary_error("binary 파일 크기 상한을 초과합니다"));
    }
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
    let count = count as usize;
    let remaining = input.len() - cursor;
    if count > remaining / MIN_ENCODED_ENTRY_BYTES {
        return Err(binary_error("entry 수가 binary 크기와 일치하지 않습니다"));
    }
    let mut entries = Vec::<PackedPosLexiconEntry>::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| binary_error("entry 저장 공간을 할당할 수 없습니다"))?;
    let mut lemma_bytes = String::new();
    lemma_bytes
        .try_reserve_exact(input.len())
        .map_err(|_| binary_error("표제어 저장 공간을 할당할 수 없습니다"))?;
    let mut lemma = String::new();
    let mut decoded_lemma_bytes = 0_usize;
    for _ in 0..count {
        let prefix = read_varint(input, &mut cursor)? as usize;
        let suffix_len = read_varint(input, &mut cursor)? as usize;
        if prefix > lemma.len() || !lemma.is_char_boundary(prefix) {
            return Err(binary_error(
                "prefix 길이가 이전 표제어의 문자 경계가 아닙니다",
            ));
        }
        let suffix_end = cursor
            .checked_add(suffix_len)
            .filter(|end| *end < input.len())
            .ok_or_else(|| binary_error("suffix가 binary 범위를 벗어납니다"))?;
        let lemma_len = prefix
            .checked_add(suffix_len)
            .ok_or_else(|| binary_error("표제어 길이가 overflow했습니다"))?;
        decoded_lemma_bytes = checked_decoded_bytes(decoded_lemma_bytes, lemma_len)?;
        let suffix = std::str::from_utf8(&input[cursor..suffix_end])
            .map_err(|_| binary_error("표제어가 유효한 UTF-8이 아닙니다"))?;
        lemma.truncate(prefix);
        lemma
            .try_reserve_exact(suffix.len())
            .map_err(|_| binary_error("표제어 저장 공간을 할당할 수 없습니다"))?;
        lemma.push_str(suffix);
        cursor = suffix_end;
        let pos = DataFinePos::from_code(input[cursor])
            .ok_or_else(|| binary_error("알 수 없는 POS code입니다"))?;
        cursor += 1;
        if lemma.is_empty() {
            return Err(binary_error("표제어가 비어 있습니다"));
        }
        require_pos_lemma_nfc(&lemma, suffix)?;
        let same_lemma = if let Some(previous_entry) = entries.last() {
            let previous_start = previous_entry.lemma_start as usize;
            let previous_end = previous_start + previous_entry.lemma_len as usize;
            let previous_lemma = &lemma_bytes.as_bytes()[previous_start..previous_end];
            let lemma_ordering = previous_lemma.cmp(lemma.as_bytes());
            if lemma_ordering == Ordering::Greater
                || (lemma_ordering == Ordering::Equal && previous_entry.pos >= pos)
            {
                return Err(binary_error("entry가 엄격한 정렬 순서가 아닙니다"));
            }
            lemma_ordering == Ordering::Equal
        } else {
            false
        };
        let (lemma_start, lemma_len) = if same_lemma {
            let previous_entry = entries.last().expect("same_lemma requires an entry");
            (previous_entry.lemma_start, previous_entry.lemma_len)
        } else {
            let start = u32::try_from(lemma_bytes.len())
                .map_err(|_| binary_error("표제어 offset이 u32 범위를 초과합니다"))?;
            let len = u32::try_from(lemma.len())
                .map_err(|_| binary_error("표제어 길이가 u32 범위를 초과합니다"))?;
            lemma_bytes.push_str(&lemma);
            (start, len)
        };
        let entry = PackedPosLexiconEntry {
            lemma_start,
            lemma_len,
            pos,
        };
        entries.push(entry);
    }
    if cursor != input.len() {
        return Err(binary_error("마지막 entry 뒤에 불필요한 바이트가 있습니다"));
    }
    Ok(DecodedPosLexicon {
        lemma_bytes: lemma_bytes.into_boxed_str(),
        entries: entries.into_boxed_slice(),
        materialized_entries: OnceLock::new(),
    })
}

fn require_pos_lemma_nfc(lemma: &str, suffix: &str) -> Result<(), DataError> {
    // The prefix comes from an NFC lemma. ASCII and precomposed Hangul suffixes
    // cannot introduce a new composition across that prefix boundary.
    let structurally_nfc_suffix = suffix
        .chars()
        .all(|character| character.is_ascii() || ('가'..='힣').contains(&character));
    if structurally_nfc_suffix {
        return Ok(());
    }
    require_nfc("POS lexicon decoder", None, "lemma", lemma)
}

fn checked_decoded_bytes(current: usize, additional: usize) -> Result<usize, DataError> {
    let total = current
        .checked_add(additional)
        .ok_or_else(|| binary_error("decoded lemma byte 수가 overflow했습니다"))?;
    if total > MAX_DECODED_LEMMA_BYTES {
        return Err(binary_error("decoded lemma byte 수 상한을 초과합니다"));
    }
    Ok(total)
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
    let first = *input
        .get(*cursor)
        .ok_or_else(|| binary_error("varint가 중간에 끝났습니다"))?;
    *cursor += 1;
    if first & 0x80 == 0 {
        return Ok(u32::from(first));
    }

    let mut value = u32::from(first & 0x7f);
    for shift in (7..=28).step_by(7) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rejects_inflated_count_before_reserving_entries() {
        let mut input = MAGIC.to_vec();
        input.extend_from_slice(&MAX_ENTRY_COUNT.to_le_bytes());

        let error = decode_pos_lexicon(&input).unwrap_err();
        assert!(matches!(*error.kind, DataErrorKind::Binary(_)));
    }

    #[test]
    fn decoded_lemma_byte_limit_is_checked_with_overflow_protection() {
        assert!(checked_decoded_bytes(MAX_DECODED_LEMMA_BYTES, 1).is_err());
        assert!(checked_decoded_bytes(usize::MAX, 1).is_err());
    }

    #[test]
    fn varint_fast_path_and_continuation_round_trip_u32_values() {
        for value in [0, 0x7f, 0x80, 0x3fff, 0x4000, u32::MAX] {
            let mut encoded = Vec::new();
            write_varint(&mut encoded, value);
            let mut cursor = 0;

            assert_eq!(read_varint(&encoded, &mut cursor).unwrap(), value);
            assert_eq!(cursor, encoded.len());
        }
    }

    #[test]
    fn approved_entries_are_sorted_and_deduplicated_before_encoding() {
        let approved = ApprovedPosLexicon::from_entries(vec![
            PosLexiconEntry {
                lemma: "사용자".to_owned(),
                pos: DataFinePos::Nng,
            },
            PosLexiconEntry {
                lemma: "걷다".to_owned(),
                pos: DataFinePos::Vv,
            },
            PosLexiconEntry {
                lemma: "걷다".to_owned(),
                pos: DataFinePos::Vv,
            },
            PosLexiconEntry {
                lemma: "걷다".to_owned(),
                pos: DataFinePos::Nng,
            },
        ]);

        let decoded = decode_pos_lexicon(&encode_pos_lexicon(&approved).unwrap()).unwrap();
        assert!(decoded.materialized_entries.get().is_none());
        assert_eq!(
            decoded.lookup_fine_pos("걷다").collect::<Vec<_>>(),
            vec![DataFinePos::Nng, DataFinePos::Vv]
        );
        assert!(decoded.materialized_entries.get().is_none());
        assert_eq!(
            decoded.entries[0].lemma_start,
            decoded.entries[1].lemma_start
        );
        assert_eq!(
            decoded.stats(),
            PosLexiconStats {
                schema_version: 1,
                entry_count: 3,
                unique_lemma_count: 2,
                predicate_entry_count: 1,
                predicate_lemma_count: 1,
                pos_conflict_lemma_count: 1,
                predicate_pos_conflict_lemma_count: 0,
                entries_by_pos: BTreeMap::from([("NNG".to_owned(), 2), ("VV".to_owned(), 1),]),
            }
        );
        assert!(decoded.materialized_entries.get().is_none());

        let unmaterialized_clone = decoded.clone();
        assert_eq!(decoded.entries().len(), 3);
        assert_eq!(decoded.lookup("걷다").len(), 2);
        assert_eq!(decoded, unmaterialized_clone);
    }

    #[test]
    fn packed_decoder_keeps_nfc_validation() {
        let decomposed = "가";
        let mut input = MAGIC.to_vec();
        input.extend_from_slice(&1_u32.to_le_bytes());
        write_varint(&mut input, 0);
        write_varint(&mut input, decomposed.len() as u32);
        input.extend_from_slice(decomposed.as_bytes());
        input.push(DataFinePos::Nng.code());

        let error = decode_pos_lexicon(&input).unwrap_err();
        assert!(matches!(*error.kind, DataErrorKind::NonNfc { .. }));
    }

    #[test]
    fn packed_decoder_checks_nfc_across_the_prefix_suffix_boundary() {
        let combining_ring = "\u{30a}";
        let mut input = MAGIC.to_vec();
        input.extend_from_slice(&2_u32.to_le_bytes());
        write_varint(&mut input, 0);
        write_varint(&mut input, 1);
        input.extend_from_slice(b"A");
        input.push(DataFinePos::Nng.code());
        write_varint(&mut input, 1);
        write_varint(&mut input, combining_ring.len() as u32);
        input.extend_from_slice(combining_ring.as_bytes());
        input.push(DataFinePos::Nng.code());

        let error = decode_pos_lexicon(&input).unwrap_err();
        assert!(matches!(*error.kind, DataErrorKind::NonNfc { .. }));
    }

    #[test]
    fn packed_decoder_keeps_utf8_validation() {
        let mut input = MAGIC.to_vec();
        input.extend_from_slice(&1_u32.to_le_bytes());
        write_varint(&mut input, 0);
        write_varint(&mut input, 1);
        input.push(0xff);
        input.push(DataFinePos::Nng.code());

        let error = decode_pos_lexicon(&input).unwrap_err();
        assert!(matches!(*error.kind, DataErrorKind::Binary(_)));
    }
}
