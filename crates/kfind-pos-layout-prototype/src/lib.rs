use std::cmp::Ordering;
use std::fmt;

#[cfg(test)]
use kfind_data::DataFinePos;
use kfind_data::PosLexiconEntry;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

const MAGIC: &[u8; 8] = b"KFPACK\x01\0";
const HEADER_LEN: usize = 56;
const RECORD_LEN: usize = 12;
const MAX_ENTRY_COUNT: u32 = 1_000_000;
const MAX_ARTIFACT_BYTES: usize = 128 * 1024 * 1024;
const POS_COUNT: u32 = 23;
const VALID_POS_MASK: u32 = (1_u32 << POS_COUNT) - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationMode {
    Attested,
    Full,
}

pub struct DirectPackedPosLexicon {
    bytes: Box<[u8]>,
    entry_count: u32,
    lemma_count: u32,
    lemma_bytes_len: usize,
}

impl DirectPackedPosLexicon {
    pub fn entry_count(&self) -> u32 {
        self.entry_count
    }

    pub fn lemma_count(&self) -> u32 {
        self.lemma_count
    }

    pub fn lookup_mask(&self, lemma: &str) -> Option<u32> {
        let mut left = 0_usize;
        let mut right = self.lemma_count as usize;
        while left < right {
            let middle = left + (right - left) / 2;
            let (candidate, mask) = self.record(middle)?;
            match candidate.as_bytes().cmp(lemma.as_bytes()) {
                Ordering::Less => left = middle + 1,
                Ordering::Greater => right = middle,
                Ordering::Equal => return Some(mask),
            }
        }
        None
    }

    pub fn artifact_bytes(&self) -> usize {
        self.bytes.len()
    }

    fn record(&self, index: usize) -> Option<(&str, u32)> {
        let record_start = HEADER_LEN
            .checked_add(self.lemma_bytes_len)?
            .checked_add(index.checked_mul(RECORD_LEN)?)?;
        let record_end = record_start.checked_add(RECORD_LEN)?;
        let record = self.bytes.get(record_start..record_end)?;
        let lemma_start = u32::from_le_bytes(record[0..4].try_into().ok()?) as usize;
        let lemma_len = u32::from_le_bytes(record[4..8].try_into().ok()?) as usize;
        let mask = u32::from_le_bytes(record[8..12].try_into().ok()?);
        let lemma_end = lemma_start.checked_add(lemma_len)?;
        let lemma_base = HEADER_LEN.checked_add(lemma_start)?;
        let lemma_limit = HEADER_LEN.checked_add(lemma_end)?;
        let lemma_bytes = self.bytes.get(lemma_base..lemma_limit)?;
        Some((std::str::from_utf8(lemma_bytes).ok()?, mask))
    }
}

impl fmt::Debug for DirectPackedPosLexicon {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectPackedPosLexicon")
            .field("entry_count", &self.entry_count)
            .field("lemma_count", &self.lemma_count)
            .field("artifact_bytes", &self.artifact_bytes())
            .finish()
    }
}

#[derive(Debug)]
pub struct PrototypeError(String);

impl fmt::Display for PrototypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PrototypeError {}

pub fn encode(entries: &[PosLexiconEntry]) -> Result<Vec<u8>, PrototypeError> {
    let mut entries = entries.to_vec();
    entries.sort_unstable();
    entries.dedup();
    let entry_count = checked_count(entries.len(), "entry")?;
    if entry_count > MAX_ENTRY_COUNT {
        return Err(error("entry count exceeds the prototype limit"));
    }

    let mut lemma_bytes = Vec::new();
    let mut records = Vec::new();
    let mut lemma_count = 0_u32;
    let mut at = 0;
    while at < entries.len() {
        let lemma = entries[at].lemma.as_str();
        validate_lemma(lemma)?;
        let end = entries[at..].partition_point(|entry| entry.lemma == lemma) + at;
        let mut mask = 0_u32;
        for entry in &entries[at..end] {
            mask |= 1_u32 << entry.pos.code();
        }
        let lemma_start = checked_count(lemma_bytes.len(), "lemma byte offset")?;
        let lemma_len = checked_count(lemma.len(), "lemma byte")?;
        lemma_bytes.extend_from_slice(lemma.as_bytes());
        records.extend_from_slice(&lemma_start.to_le_bytes());
        records.extend_from_slice(&lemma_len.to_le_bytes());
        records.extend_from_slice(&mask.to_le_bytes());
        lemma_count = lemma_count
            .checked_add(1)
            .ok_or_else(|| error("lemma count overflow"))?;
        at = end;
    }

    let lemma_bytes_len = checked_count(lemma_bytes.len(), "lemma blob byte")?;
    let record_bytes_len = checked_count(records.len(), "record byte")?;
    let mut payload = Vec::with_capacity(lemma_bytes.len() + records.len());
    payload.extend_from_slice(&lemma_bytes);
    payload.extend_from_slice(&records);

    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&lemma_count.to_le_bytes());
    output.extend_from_slice(&lemma_bytes_len.to_le_bytes());
    output.extend_from_slice(&record_bytes_len.to_le_bytes());
    output.extend_from_slice(&sha256(&payload));
    debug_assert_eq!(output.len(), HEADER_LEN);
    output.extend_from_slice(&payload);
    if output.len() > MAX_ARTIFACT_BYTES {
        return Err(error("artifact exceeds the prototype size limit"));
    }
    Ok(output)
}

pub fn decode(
    input: Vec<u8>,
    expected_artifact_sha256: [u8; 32],
    mode: ValidationMode,
) -> Result<DirectPackedPosLexicon, PrototypeError> {
    if input.len() > MAX_ARTIFACT_BYTES {
        return Err(error("artifact exceeds the prototype size limit"));
    }
    if sha256(&input) != expected_artifact_sha256 {
        return Err(error(
            "artifact SHA-256 does not match the caller attestation",
        ));
    }
    if input.len() < HEADER_LEN || &input[..MAGIC.len()] != MAGIC {
        return Err(error("invalid direct packed prototype magic or schema"));
    }

    let mut cursor = MAGIC.len();
    let entry_count = read_u32(&input, &mut cursor)?;
    let lemma_count = read_u32(&input, &mut cursor)?;
    let lemma_bytes_len = read_u32(&input, &mut cursor)? as usize;
    let record_bytes_len = read_u32(&input, &mut cursor)? as usize;
    let payload_digest = read_digest(&input, &mut cursor)?;
    let expected_record_bytes = (lemma_count as usize)
        .checked_mul(RECORD_LEN)
        .ok_or_else(|| error("prototype record byte length overflow"))?;
    let expected_artifact_len = HEADER_LEN
        .checked_add(lemma_bytes_len)
        .and_then(|length| length.checked_add(record_bytes_len));
    if cursor != HEADER_LEN
        || record_bytes_len != expected_record_bytes
        || expected_artifact_len != Some(input.len())
    {
        return Err(error("prototype section lengths do not match the artifact"));
    }
    if entry_count == 0 || entry_count > MAX_ENTRY_COUNT {
        return Err(error(
            "prototype entry count is outside the supported range",
        ));
    }
    if lemma_count == 0 || lemma_count > entry_count {
        return Err(error(
            "prototype lemma count is outside the supported range",
        ));
    }
    if mode == ValidationMode::Full && sha256(&input[HEADER_LEN..]) != payload_digest {
        return Err(error("prototype payload digest mismatch"));
    }

    let lexicon = DirectPackedPosLexicon {
        bytes: input.into_boxed_slice(),
        entry_count,
        lemma_count,
        lemma_bytes_len,
    };
    if mode == ValidationMode::Full {
        validate_all(&lexicon)?;
    }
    Ok(lexicon)
}

pub fn parse_sha256(value: &str) -> Result<[u8; 32], PrototypeError> {
    if value.len() != 64 {
        return Err(error("SHA-256 must contain 64 hexadecimal characters"));
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| error("SHA-256 contains a non-hexadecimal character"))?;
    }
    Ok(digest)
}

fn validate_all(lexicon: &DirectPackedPosLexicon) -> Result<(), PrototypeError> {
    let mut previous = None;
    let mut expected_lemma_start = 0_usize;
    let mut entry_count = 0_u32;
    for index in 0..lexicon.lemma_count as usize {
        let (lemma, mask) = lexicon
            .record(index)
            .ok_or_else(|| error("prototype record is invalid"))?;
        validate_lemma(lemma)?;
        if mask == 0 || mask & !VALID_POS_MASK != 0 {
            return Err(error("prototype record contains an invalid POS mask"));
        }
        if let Some(previous) = previous
            && previous >= lemma.as_bytes()
        {
            return Err(error("prototype lemmas are not strictly sorted"));
        }
        let record_start = HEADER_LEN + lexicon.lemma_bytes_len + index * RECORD_LEN;
        let stored_start = u32::from_le_bytes(
            lexicon.bytes[record_start..record_start + 4]
                .try_into()
                .map_err(|_| error("failed to decode prototype lemma offset"))?,
        ) as usize;
        if stored_start != expected_lemma_start {
            return Err(error("prototype lemma blob is not contiguous"));
        }
        expected_lemma_start = expected_lemma_start
            .checked_add(lemma.len())
            .ok_or_else(|| error("prototype lemma byte count overflow"))?;
        entry_count = entry_count
            .checked_add(mask.count_ones())
            .ok_or_else(|| error("prototype decoded entry count overflow"))?;
        previous = Some(lemma.as_bytes());
    }
    if expected_lemma_start != lexicon.lemma_bytes_len {
        return Err(error("prototype lemma blob has trailing bytes"));
    }
    if entry_count != lexicon.entry_count {
        return Err(error(
            "prototype decoded entry count does not match the header",
        ));
    }
    Ok(())
}

fn validate_lemma(lemma: &str) -> Result<(), PrototypeError> {
    if lemma.is_empty() {
        return Err(error("prototype lemma is empty"));
    }
    if !lemma.nfc().eq(lemma.chars()) {
        return Err(error("prototype lemma is not NFC"));
    }
    Ok(())
}

fn checked_count(value: usize, label: &str) -> Result<u32, PrototypeError> {
    u32::try_from(value).map_err(|_| error(format!("{label} count exceeds u32")))
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32, PrototypeError> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| error("prototype header offset overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| error("truncated prototype u32"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| error("failed to decode prototype u32"))?,
    ))
}

fn read_digest(input: &[u8], cursor: &mut usize) -> Result<[u8; 32], PrototypeError> {
    let end = cursor
        .checked_add(32)
        .ok_or_else(|| error("prototype header offset overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| error("truncated prototype digest"))?;
    *cursor = end;
    bytes
        .try_into()
        .map_err(|_| error("failed to decode prototype digest"))
}

fn error(message: impl Into<String>) -> PrototypeError {
    PrototypeError(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<PosLexiconEntry> {
        vec![
            PosLexiconEntry {
                lemma: "걷다".to_owned(),
                pos: DataFinePos::Nng,
            },
            PosLexiconEntry {
                lemma: "걷다".to_owned(),
                pos: DataFinePos::Vv,
            },
            PosLexiconEntry {
                lemma: "사용자".to_owned(),
                pos: DataFinePos::Nng,
            },
        ]
    }

    #[test]
    fn round_trip_preserves_direct_pos_masks() {
        let encoded = encode(&entries()).unwrap();
        let digest = sha256(&encoded);
        for mode in [ValidationMode::Attested, ValidationMode::Full] {
            let decoded = decode(encoded.clone(), digest, mode).unwrap();
            assert_eq!(decoded.entry_count(), 3);
            assert_eq!(decoded.lemma_count(), 2);
            assert_eq!(
                decoded.lookup_mask("걷다"),
                Some((1_u32 << DataFinePos::Nng.code()) | (1_u32 << DataFinePos::Vv.code()))
            );
        }
    }

    #[test]
    fn attestation_and_full_validation_reject_corruption() {
        let mut encoded = encode(&entries()).unwrap();
        let original_digest = sha256(&encoded);
        encoded[HEADER_LEN - 1] ^= 1;
        assert!(decode(encoded, original_digest, ValidationMode::Attested).is_err());

        let mut encoded = encode(&entries()).unwrap();
        encoded[HEADER_LEN - 1] ^= 1;
        let corrupted_digest = sha256(&encoded);
        assert!(decode(encoded, corrupted_digest, ValidationMode::Full).is_err());
    }
}
