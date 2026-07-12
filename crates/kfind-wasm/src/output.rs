use kfind::PhraseMatch;
use serde::Serialize;
use wasm_bindgen::{JsError, JsValue};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchOutput {
    start: usize,
    end: usize,
    atoms: Vec<AtomOutput>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct AtomOutput {
    core: SpanOutput,
    token: SpanOutput,
    origins: Vec<OriginOutput>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct SpanOutput {
    start: usize,
    end: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct OriginOutput {
    analysis_index: u16,
    rule_path: Vec<String>,
}

pub fn serialize_matches(text: &str, matches: &[PhraseMatch]) -> Result<JsValue, JsError> {
    let output = convert_matches(text, matches).map_err(|message| JsError::new(&message))?;
    serde_wasm_bindgen::to_value(&output)
        .map_err(|error| JsError::new(&format!("failed to serialize matches: {error}")))
}

fn convert_matches(text: &str, matches: &[PhraseMatch]) -> Result<Vec<MatchOutput>, String> {
    let offsets = Utf16Offsets::new(text, matches)?;
    matches
        .iter()
        .map(|matched| {
            Ok(MatchOutput {
                start: offsets.get(matched.span.start)?,
                end: offsets.get(matched.span.end)?,
                atoms: matched
                    .atoms
                    .iter()
                    .map(|atom| {
                        Ok(AtomOutput {
                            core: SpanOutput {
                                start: offsets.get(atom.core.start)?,
                                end: offsets.get(atom.core.end)?,
                            },
                            token: SpanOutput {
                                start: offsets.get(atom.token.start)?,
                                end: offsets.get(atom.token.end)?,
                            },
                            origins: atom
                                .origins
                                .iter()
                                .map(|origin| OriginOutput {
                                    analysis_index: origin.analysis_index,
                                    rule_path: origin
                                        .rule_path
                                        .iter()
                                        .map(|rule| rule.as_str().to_owned())
                                        .collect(),
                                })
                                .collect(),
                        })
                    })
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}

struct Utf16Offsets {
    values: Vec<(usize, usize)>,
}

impl Utf16Offsets {
    fn new(text: &str, matches: &[PhraseMatch]) -> Result<Self, String> {
        let mut byte_offsets = Vec::new();
        for matched in matches {
            byte_offsets.extend([matched.span.start, matched.span.end]);
            for atom in &matched.atoms {
                byte_offsets.extend([
                    atom.core.start,
                    atom.core.end,
                    atom.token.start,
                    atom.token.end,
                ]);
            }
        }
        byte_offsets.sort_unstable();
        byte_offsets.dedup();

        let mut previous_byte = 0;
        let mut utf16_offset = 0;
        let mut values = Vec::with_capacity(byte_offsets.len());
        for byte_offset in byte_offsets {
            if byte_offset > text.len() || !text.is_char_boundary(byte_offset) {
                return Err(format!(
                    "matcher returned invalid UTF-8 byte offset {byte_offset} for {} byte input",
                    text.len()
                ));
            }
            utf16_offset += text[previous_byte..byte_offset].encode_utf16().count();
            values.push((byte_offset, utf16_offset));
            previous_byte = byte_offset;
        }
        Ok(Self { values })
    }

    fn get(&self, byte_offset: usize) -> Result<usize, String> {
        self.values
            .binary_search_by_key(&byte_offset, |(byte, _)| *byte)
            .map(|index| self.values[index].1)
            .map_err(|_| format!("missing UTF-16 mapping for byte offset {byte_offset}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_utf8_byte_offsets_to_utf16_code_units() {
        let text = "😀 길을 걸어 갔다.";
        let start = "😀 길을 ".len();
        let end = start + "걸어".len();
        let output = convert_matches(
            text,
            &[PhraseMatch {
                span: start..end,
                atoms: Vec::new(),
            }],
        )
        .unwrap();

        assert_eq!(output[0].start, "😀 길을 ".encode_utf16().count());
        assert_eq!(output[0].end, "😀 길을 걸어".encode_utf16().count());
    }
}
