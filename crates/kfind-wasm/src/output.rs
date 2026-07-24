use kfind::PhraseMatch;
use kfind::expert::QueryPlan;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    lemma: Option<String>,
    rule_path: Vec<String>,
}

pub fn serialize_matches(
    text: &str,
    matches: &[PhraseMatch],
    plan: &QueryPlan,
) -> Result<JsValue, JsError> {
    let output = convert_matches(text, matches, plan).map_err(|message| JsError::new(&message))?;
    serde_wasm_bindgen::to_value(&output)
        .map_err(|error| JsError::new(&format!("failed to serialize matches: {error}")))
}

fn convert_matches(
    text: &str,
    matches: &[PhraseMatch],
    plan: &QueryPlan,
) -> Result<Vec<MatchOutput>, String> {
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
                    .enumerate()
                    .map(|(atom_index, atom)| {
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
                                .map(|origin| {
                                    let lemma = if origin.rule_path.is_empty() {
                                        None
                                    } else {
                                        let analysis = plan
                                            .atoms
                                            .get(atom_index)
                                            .and_then(|atom| {
                                                atom.analyses
                                                    .get(usize::from(origin.analysis_index))
                                            })
                                            .ok_or_else(|| {
                                                format!(
                                                    "match origin references missing analysis {} for atom {atom_index}",
                                                    origin.analysis_index
                                                )
                                            })?;
                                        Some(analysis.lemma.to_string())
                                    };
                                    Ok(OriginOutput {
                                        analysis_index: origin.analysis_index,
                                        lemma,
                                        rule_path: origin
                                            .rule_path
                                            .iter()
                                            .map(|rule| rule.as_str().to_owned())
                                            .collect(),
                                    })
                                })
                                .collect::<Result<_, String>>()?,
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
    use kfind::expert::MatcherExt;

    #[test]
    fn converts_utf8_byte_offsets_to_utf16_code_units() {
        let text = "😀 길을 걸어 갔다.";
        let start = "😀 길을 ".len();
        let end = start + "걸어".len();
        let engine = kfind::Engine::new().unwrap();
        let matcher = engine
            .compile("걷다", &kfind::CompileOptions::default())
            .unwrap();
        let output = convert_matches(
            text,
            &[PhraseMatch {
                span: start..end,
                atoms: Vec::new(),
            }],
            matcher.plan(),
        )
        .unwrap();

        assert_eq!(output[0].start, "😀 길을 ".encode_utf16().count());
        assert_eq!(output[0].end, "😀 길을 걸어".encode_utf16().count());
    }

    #[test]
    fn includes_the_generated_lemma_in_each_match_origin() {
        let text = "길을 걸었다.";
        let engine = kfind::Engine::new().unwrap();
        let matcher = engine
            .compile("걷다", &kfind::CompileOptions::default())
            .unwrap();
        let matched = matcher.find_all(text.as_bytes());
        let output = convert_matches(text, &matched, matcher.plan()).unwrap();

        assert!(
            output[0].atoms[0]
                .origins
                .iter()
                .all(|origin| origin.lemma.as_deref() == Some("걷다"))
        );
    }

    #[test]
    fn omits_the_lemma_for_direct_match_origins() {
        let text = "기준표식";
        let engine = kfind::Engine::new().unwrap();
        let matcher = engine
            .compile(
                "기준표식",
                &kfind::CompileOptions {
                    boundary: kfind::BoundaryPolicy::Any,
                    expand: kfind::ExpandMode::Literal,
                    global_pos: Some(kfind::CoarsePos::Literal),
                    ..kfind::CompileOptions::default()
                },
            )
            .unwrap();
        let matched = matcher.find_all(text.as_bytes());
        let output = convert_matches(text, &matched, matcher.plan()).unwrap();

        assert!(
            output[0].atoms[0]
                .origins
                .iter()
                .all(|origin| origin.lemma.is_none())
        );
    }
}
