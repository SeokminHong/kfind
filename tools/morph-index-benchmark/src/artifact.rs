use anyhow::{Result, bail, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"KFMORPH\0";
const SCHEMA_VERSION: u32 = 1;
const POS_COUNT: usize = 23;
const HEADER_LEN: usize = 228;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndexKind {
    DoubleArray,
    Fst,
}

impl IndexKind {
    pub fn artifact_name(self) -> &'static str {
        match self {
            Self::DoubleArray => "morphology-double-array.kfm",
            Self::Fst => "morphology-fst.kfm",
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::DoubleArray => 1,
            Self::Fst => 2,
        }
    }

    fn from_code(code: u8) -> Result<Self> {
        match code {
            1 => Ok(Self::DoubleArray),
            2 => Ok(Self::Fst),
            _ => bail!("artifact corruption: unknown index kind {code}"),
        }
    }
}

#[derive(Debug)]
pub struct ArtifactView<'a> {
    pub kind: IndexKind,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub pos_counts: [u32; POS_COUNT],
    pub index: &'a [u8],
    pub payload: &'a [u8],
}

#[derive(Debug, Serialize)]
pub struct ArtifactSummary {
    pub schema_version: u32,
    pub kind: IndexKind,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub pos_counts: [u32; POS_COUNT],
    pub index_bytes: usize,
    pub payload_bytes: usize,
}

impl ArtifactView<'_> {
    pub fn summary(&self) -> ArtifactSummary {
        ArtifactSummary {
            schema_version: SCHEMA_VERSION,
            kind: self.kind,
            surface_count: self.surface_count,
            analysis_count: self.analysis_count,
            pos_counts: self.pos_counts,
            index_bytes: self.index.len(),
            payload_bytes: self.payload.len(),
        }
    }
}

pub fn build_container(
    kind: IndexKind,
    source_digest: [u8; 32],
    surface_count: u32,
    analysis_count: u32,
    pos_counts: [u32; POS_COUNT],
    index: &[u8],
    payload: &[u8],
) -> Result<Vec<u8>> {
    ensure!(!index.is_empty(), "index must not be empty");
    ensure!(!payload.is_empty(), "payload must not be empty");
    let mut output = Vec::with_capacity(HEADER_LEN + index.len() + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&SCHEMA_VERSION.to_le_bytes());
    output.push(kind.code());
    output.extend_from_slice(&[0; 3]);
    output.extend_from_slice(&source_digest);
    output.extend_from_slice(&surface_count.to_le_bytes());
    output.extend_from_slice(&analysis_count.to_le_bytes());
    for count in pos_counts {
        output.extend_from_slice(&count.to_le_bytes());
    }
    output.extend_from_slice(&u64::try_from(index.len())?.to_le_bytes());
    output.extend_from_slice(&u64::try_from(payload.len())?.to_le_bytes());
    output.extend_from_slice(&sha256(index));
    output.extend_from_slice(&sha256(payload));
    debug_assert_eq!(output.len(), HEADER_LEN);
    output.extend_from_slice(index);
    output.extend_from_slice(payload);
    Ok(output)
}

pub fn validate_container<'a>(
    input: &'a [u8],
    expected_source_digest: &[u8; 32],
) -> Result<ArtifactView<'a>> {
    ensure!(
        input.len() >= HEADER_LEN,
        "artifact corruption: truncated header"
    );
    ensure!(
        &input[..MAGIC.len()] == MAGIC,
        "artifact corruption: bad magic"
    );
    let mut cursor = MAGIC.len();
    let schema_version = read_u32(input, &mut cursor)?;
    ensure!(
        schema_version == SCHEMA_VERSION,
        "schema mismatch: expected {SCHEMA_VERSION}, got {schema_version}"
    );
    let kind = IndexKind::from_code(input[cursor])?;
    cursor += 1;
    ensure!(
        input[cursor..cursor + 3] == [0; 3],
        "artifact corruption: reserved header bytes are not zero"
    );
    cursor += 3;
    let source_digest = read_digest(input, &mut cursor)?;
    ensure!(
        &source_digest == expected_source_digest,
        "source digest mismatch: artifact={}, expected={}",
        hex(&source_digest),
        hex(expected_source_digest)
    );
    let surface_count = read_u32(input, &mut cursor)?;
    let analysis_count = read_u32(input, &mut cursor)?;
    let mut pos_counts = [0; POS_COUNT];
    for count in &mut pos_counts {
        *count = read_u32(input, &mut cursor)?;
    }
    ensure!(
        pos_counts.iter().copied().map(u64::from).sum::<u64>() == u64::from(analysis_count),
        "artifact corruption: POS counts do not equal analysis count"
    );
    let index_len = usize::try_from(read_u64(input, &mut cursor)?)?;
    let payload_len = usize::try_from(read_u64(input, &mut cursor)?)?;
    let index_digest = read_digest(input, &mut cursor)?;
    let payload_digest = read_digest(input, &mut cursor)?;
    ensure!(
        cursor == HEADER_LEN,
        "artifact corruption: invalid header length"
    );
    let index_end = cursor
        .checked_add(index_len)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: index length overflow"))?;
    let payload_end = index_end
        .checked_add(payload_len)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: payload length overflow"))?;
    ensure!(
        payload_end == input.len(),
        "artifact corruption: section lengths do not match file"
    );
    let index = &input[cursor..index_end];
    let payload = &input[index_end..payload_end];
    ensure!(
        sha256(index) == index_digest,
        "artifact corruption: index digest mismatch"
    );
    ensure!(
        sha256(payload) == payload_digest,
        "artifact corruption: payload digest mismatch"
    );
    validate_payload(payload, surface_count, analysis_count)?;
    crate::index::validate(kind, index, surface_count)?;
    Ok(ArtifactView {
        kind,
        surface_count,
        analysis_count,
        pos_counts,
        index,
        payload,
    })
}

pub fn parse_digest(value: &str) -> Result<[u8; 32]> {
    ensure!(
        value.len() == 64,
        "SHA-256 must contain 64 hexadecimal characters"
    );
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| anyhow::anyhow!("SHA-256 contains a non-hexadecimal character"))?;
    }
    Ok(digest)
}

fn validate_payload(input: &[u8], surface_count: u32, analysis_count: u32) -> Result<()> {
    let expected_offsets = usize::try_from(surface_count)? + 1;
    let header_len =
        8_usize
            .checked_add(expected_offsets.checked_mul(4).ok_or_else(|| {
                anyhow::anyhow!("artifact corruption: payload offset table overflow")
            })?)
            .ok_or_else(|| anyhow::anyhow!("artifact corruption: payload header overflow"))?;
    let expected_len = header_len
        .checked_add(
            usize::try_from(analysis_count)?
                .checked_mul(12)
                .ok_or_else(|| anyhow::anyhow!("artifact corruption: payload records overflow"))?,
        )
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: payload length overflow"))?;
    ensure!(
        input.len() == expected_len,
        "artifact corruption: invalid payload length"
    );
    let mut cursor = 0;
    ensure!(
        read_u32(input, &mut cursor)? == surface_count,
        "artifact corruption: payload surface count mismatch"
    );
    ensure!(
        read_u32(input, &mut cursor)? == analysis_count,
        "artifact corruption: payload analysis count mismatch"
    );
    let mut previous = 0;
    for index in 0..expected_offsets {
        let offset = read_u32(input, &mut cursor)?;
        ensure!(
            offset >= previous,
            "artifact corruption: payload offsets are not ordered"
        );
        ensure!(
            offset <= analysis_count,
            "artifact corruption: payload offset out of range"
        );
        if index == 0 {
            ensure!(
                offset == 0,
                "artifact corruption: first payload offset is not zero"
            );
        }
        previous = offset;
    }
    ensure!(
        previous == analysis_count,
        "artifact corruption: final payload offset mismatch"
    );
    for _ in 0..analysis_count {
        ensure!(
            input[cursor] < POS_COUNT as u8,
            "artifact corruption: invalid POS code"
        );
        ensure!(
            input[cursor + 1..cursor + 4] == [0; 3],
            "artifact corruption: payload reserved bytes are not zero"
        );
        cursor += 12;
    }
    Ok(())
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: offset overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: truncated u32"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}

fn read_u64(input: &[u8], cursor: &mut usize) -> Result<u64> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: offset overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: truncated u64"))?;
    *cursor = end;
    Ok(u64::from_le_bytes(bytes.try_into()?))
}

fn read_digest(input: &[u8], cursor: &mut usize) -> Result<[u8; 32]> {
    let end = cursor
        .checked_add(32)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: offset overflow"))?;
    let bytes = input
        .get(*cursor..end)
        .ok_or_else(|| anyhow::anyhow!("artifact corruption: truncated digest"))?;
    *cursor = end;
    Ok(bytes.try_into()?)
}

fn hex(input: &[u8]) -> String {
    input.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_rejects_source_and_content_mismatches() {
        let source = [7; 32];
        let index = crate::index::build(IndexKind::Fst, &[("가".to_owned(), 0)]).unwrap();
        let payload = crate::dataset::encode_payload(&[vec![crate::dataset::analysis(0)]]).unwrap();
        let bytes =
            build_container(IndexKind::Fst, source, 1, 1, pos_counts(), &index, &payload).unwrap();

        let mut wrong_schema = bytes.clone();
        wrong_schema[MAGIC.len()..MAGIC.len() + 4].copy_from_slice(&2_u32.to_le_bytes());
        assert!(
            validate_container(&wrong_schema, &source)
                .unwrap_err()
                .to_string()
                .contains("schema mismatch")
        );
        assert!(
            validate_container(&bytes, &[8; 32])
                .unwrap_err()
                .to_string()
                .contains("source digest mismatch")
        );
        let mut corrupt = bytes;
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        assert!(
            validate_container(&corrupt, &source)
                .unwrap_err()
                .to_string()
                .contains("payload digest mismatch")
        );
    }

    fn pos_counts() -> [u32; POS_COUNT] {
        let mut counts = [0; POS_COUNT];
        counts[0] = 1;
        counts
    }
}
