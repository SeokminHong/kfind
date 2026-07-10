use std::io::Write;
use std::ops::Range;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use kfind_query::{QueryPlan, VerifiedSpan};
use kfind_search::{FileSearchResult, SearchLine, SearchLineKind, SearchRecord};
use serde::Serialize;

use super::explain::pos_label;
use super::text::{first_scalar_column, line_content, path_bytes};
use super::{OutputError, OutputOptions};

pub(super) fn write_file(
    writer: &mut impl Write,
    result: &FileSearchResult,
    plan: &QueryPlan,
    options: OutputOptions,
) -> Result<(), OutputError> {
    for record in &result.records {
        match record {
            SearchRecord::Line(line) => {
                let value = JsonLine::new(&result.path, line, plan, options.column);
                write_json_line(writer, &value)?;
            }
            SearchRecord::ContextBreak => {
                let value = JsonContextBreak {
                    record_type: "context_break",
                    path: JsonPath::new(&result.path),
                };
                write_json_line(writer, &value)?;
            }
        }
    }
    Ok(())
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), OutputError> {
    let mut bytes = Vec::new();
    serde_json::to_writer(&mut bytes, value).map_err(OutputError::Json)?;
    bytes.push(b'\n');
    writer.write_all(&bytes).map_err(OutputError::Io)
}

#[derive(Serialize)]
struct JsonLine {
    #[serde(rename = "type")]
    record_type: &'static str,
    #[serde(flatten)]
    path: JsonPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_kind: Option<&'static str>,
    #[serde(flatten)]
    text: JsonText,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    spans: Vec<JsonSpan>,
    offset_unit: &'static str,
}

impl JsonLine {
    fn new(path: &Path, line: &SearchLine, plan: &QueryPlan, include_column: bool) -> Self {
        let is_match = line.kind == SearchLineKind::Match;
        Self {
            record_type: if is_match { "match" } else { "context" },
            path: JsonPath::new(path),
            line: line.line_number,
            column: (include_column && is_match)
                .then(|| first_scalar_column(line))
                .flatten(),
            context_kind: context_kind(line.kind),
            text: JsonText::new(line_content(&line.bytes)),
            spans: json_spans(line, plan),
            offset_unit: "utf8-bytes",
        }
    }
}

#[derive(Serialize)]
struct JsonContextBreak {
    #[serde(rename = "type")]
    record_type: &'static str,
    #[serde(flatten)]
    path: JsonPath,
}

#[derive(Serialize)]
struct JsonPath {
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_encoding: Option<&'static str>,
}

impl JsonPath {
    fn new(path: &Path) -> Self {
        let bytes = path_bytes(path);
        match std::str::from_utf8(&bytes) {
            Ok(path) => Self {
                path: Some(path.to_owned()),
                path_base64: None,
                path_encoding: None,
            },
            Err(_) => Self {
                path: None,
                path_base64: Some(STANDARD.encode(&bytes)),
                path_encoding: Some("bytes"),
            },
        }
    }
}

#[derive(Serialize)]
struct JsonText {
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<&'static str>,
}

impl JsonText {
    fn new(bytes: &[u8]) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(text) => Self {
                text: Some(text.to_owned()),
                text_base64: None,
                encoding: None,
            },
            Err(_) => Self {
                text: None,
                text_base64: Some(STANDARD.encode(bytes)),
                encoding: Some("bytes"),
            },
        }
    }
}

#[derive(Serialize)]
struct JsonSpan {
    atom: usize,
    core: JsonRange,
    token: JsonRange,
    surface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_encoding: Option<&'static str>,
    origins: Vec<JsonOrigin>,
}

impl JsonSpan {
    fn new(atom_index: usize, span: &VerifiedSpan, bytes: &[u8], plan: &QueryPlan) -> Self {
        let surface_bytes = bytes.get(span.token.clone()).unwrap_or_default();
        let (surface, surface_base64, surface_encoding) = match std::str::from_utf8(surface_bytes) {
            Ok(surface) => (Some(surface.to_owned()), None, None),
            Err(_) => (None, Some(STANDARD.encode(surface_bytes)), Some("bytes")),
        };
        let origins = span
            .origins
            .iter()
            .map(|origin| {
                let analysis = plan
                    .atoms
                    .get(atom_index)
                    .and_then(|atom| atom.analyses.get(usize::from(origin.analysis_index)));
                JsonOrigin {
                    analysis_index: origin.analysis_index,
                    lemma: analysis.map(|analysis| analysis.lemma.to_string()),
                    pos: analysis.map(|analysis| pos_label(analysis.coarse_pos)),
                    rules: origin
                        .rule_path
                        .iter()
                        .map(|rule| rule.as_str().to_owned())
                        .collect(),
                }
            })
            .collect();
        Self {
            atom: atom_index,
            core: JsonRange::from(span.core.clone()),
            token: JsonRange::from(span.token.clone()),
            surface,
            surface_base64,
            surface_encoding,
            origins,
        }
    }
}

fn json_spans(line: &SearchLine, plan: &QueryPlan) -> Vec<JsonSpan> {
    line.matches
        .iter()
        .flat_map(|matched| matched.atoms.iter().enumerate())
        .map(|(atom_index, span)| JsonSpan::new(atom_index, span, &line.bytes, plan))
        .collect()
}

#[derive(Serialize)]
struct JsonRange {
    start: usize,
    end: usize,
}

impl From<Range<usize>> for JsonRange {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Serialize)]
struct JsonOrigin {
    analysis_index: u16,
    lemma: Option<String>,
    pos: Option<&'static str>,
    rules: Vec<String>,
}

const fn context_kind(kind: SearchLineKind) -> Option<&'static str> {
    match kind {
        SearchLineKind::Match => None,
        SearchLineKind::BeforeContext => Some("before"),
        SearchLineKind::AfterContext => Some("after"),
        SearchLineKind::OtherContext => Some("other"),
    }
}
