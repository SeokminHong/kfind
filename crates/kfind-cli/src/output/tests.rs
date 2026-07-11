use std::io::{self, Write};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use kfind_morph::RuleId;
use kfind_query::{
    CompileOptions, LexiconQueryAnalyzer, Lexicons, Origin, PhraseMatch, QueryPlan, VerifiedSpan,
    compile_query,
};
use kfind_search::{FileSearchResult, SearchLine, SearchLineKind, SearchRecord};
use serde_json::Value;

use super::*;
use crate::Args;

fn query_plan() -> QueryPlan {
    let lexicons = Arc::new(Lexicons::embedded().unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    compile_query("걷다", &CompileOptions::default(), &analyzer).unwrap()
}

fn matched_span(range: Range<usize>) -> PhraseMatch {
    PhraseMatch {
        span: range.clone(),
        atoms: vec![VerifiedSpan {
            core: range.start..range.start + "걸".len(),
            token: range,
            origins: vec![Origin {
                analysis_index: 0,
                rule_path: vec![RuleId::from("lexical.d-to-l")],
            }],
        }],
    }
}

fn match_line(line_number: u64, bytes: &[u8]) -> SearchLine {
    let start = std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.find("걸어"))
        .expect("test line contains a UTF-8 match");
    SearchLine {
        kind: SearchLineKind::Match,
        line_number: Some(line_number),
        absolute_byte_offset: 0,
        bytes: bytes.to_vec(),
        matches: vec![matched_span(start..start + "걸어".len())],
    }
}

fn result(records: Vec<SearchRecord>, matching_lines: u64) -> FileSearchResult {
    FileSearchResult {
        path: PathBuf::from("sample.txt"),
        records,
        matching_lines,
        matched_spans: Some(matching_lines),
        binary_byte_offset: None,
    }
}

#[test]
fn standard_output_supports_prefixes_columns_context_and_safe_text() {
    let records = vec![
        SearchRecord::Line(SearchLine {
            kind: SearchLineKind::BeforeContext,
            line_number: Some(1),
            absolute_byte_offset: 0,
            bytes: b"before\x1b\t\n".to_vec(),
            matches: Vec::new(),
        }),
        SearchRecord::ContextBreak,
        SearchRecord::Line(match_line(3, "길을 걸어 갔다.\n".as_bytes())),
    ];
    let options = OutputOptions {
        filename: FilenameMode::Always,
        line_number: true,
        column: true,
        ..OutputOptions::default()
    };
    let mut output = OutputWriter::new(Vec::new(), options);

    output
        .write_file(&result(records, 1), &query_plan())
        .unwrap();
    let text = String::from_utf8(output.into_inner()).unwrap();

    assert_eq!(
        text,
        "sample.txt-1- before\\u{001B}\\t\n--\nsample.txt:3:4: 길을 걸어 갔다.\n"
    );
}

#[test]
fn color_highlights_token_spans_only() {
    let records = vec![SearchRecord::Line(match_line(
        1,
        "길을 걸어 갔다.\n".as_bytes(),
    ))];
    let options = OutputOptions {
        color: ResolvedColor::Enabled,
        ..OutputOptions::default()
    };
    let mut output = OutputWriter::new(Vec::new(), options);

    output
        .write_file(&result(records, 1), &query_plan())
        .unwrap();

    assert_eq!(
        String::from_utf8(output.into_inner()).unwrap(),
        "길을 \x1b[1;31m걸어\x1b[0m 갔다.\n"
    );
}

#[test]
fn count_means_matching_lines_and_files_mode_filters_non_matches() {
    let plan = query_plan();
    let count_options = OutputOptions {
        mode: OutputMode::Count,
        filename: FilenameMode::Always,
        ..OutputOptions::default()
    };
    let mut count = OutputWriter::new(Vec::new(), count_options);
    count.write_file(&result(Vec::new(), 7), &plan).unwrap();
    assert_eq!(count.into_inner(), b"sample.txt:7\n");

    let file_options = OutputOptions {
        mode: OutputMode::FilesWithMatches,
        ..OutputOptions::default()
    };
    let mut files = OutputWriter::new(Vec::new(), file_options);
    files.write_file(&result(Vec::new(), 0), &plan).unwrap();
    files.write_file(&result(Vec::new(), 1), &plan).unwrap();
    assert_eq!(files.into_inner(), b"sample.txt\n");
}

#[test]
fn json_preserves_offsets_origins_and_escapes_controls() {
    let records = vec![SearchRecord::Line(match_line(
        3,
        "길을 걸어\u{1b} 갔다.\n".as_bytes(),
    ))];
    let options = OutputOptions {
        mode: OutputMode::JsonLines,
        column: true,
        ..OutputOptions::default()
    };
    let mut output = OutputWriter::new(Vec::new(), options);
    output
        .write_file(&result(records, 1), &query_plan())
        .unwrap();
    let bytes = output.into_inner();
    let raw = std::str::from_utf8(&bytes).unwrap();

    assert!(raw.contains("\\u001b"));
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["type"], "match");
    assert_eq!(value["column"], 4);
    assert_eq!(value["offset_unit"], "utf8-bytes");
    assert_eq!(value["spans"][0]["surface"], "걸어");
    assert_eq!(value["spans"][0]["origins"][0]["lemma"], "걷다");
    assert_eq!(
        value["spans"][0]["origins"][0]["rules"][0],
        "lexical.d-to-l"
    );
}

#[test]
fn json_uses_base64_for_non_utf8_text() {
    let records = vec![SearchRecord::Line(SearchLine {
        kind: SearchLineKind::Match,
        line_number: Some(1),
        absolute_byte_offset: 0,
        bytes: b"\xff\x1b\n".to_vec(),
        matches: Vec::new(),
    })];
    let options = OutputOptions {
        mode: OutputMode::JsonLines,
        ..OutputOptions::default()
    };
    let mut output = OutputWriter::new(Vec::new(), options);

    output
        .write_file(&result(records, 1), &query_plan())
        .unwrap();
    let value: Value = serde_json::from_slice(&output.into_inner()).unwrap();

    assert!(value["text"].is_null());
    assert_eq!(value["text_base64"], "/xs=");
    assert_eq!(value["encoding"], "bytes");
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_are_safe_in_text_and_lossless_in_json() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let mut non_utf8 = result(Vec::new(), 2);
    non_utf8.path = PathBuf::from(OsString::from_vec(b"bad\xff.txt".to_vec()));
    let plan = query_plan();

    let mut text = OutputWriter::new(
        Vec::new(),
        OutputOptions {
            mode: OutputMode::Count,
            filename: FilenameMode::Always,
            ..OutputOptions::default()
        },
    );
    text.write_file(&non_utf8, &plan).unwrap();
    assert_eq!(text.into_inner(), b"bad\\xFF.txt:2\n");

    non_utf8.records = vec![SearchRecord::ContextBreak];
    let mut json = OutputWriter::new(
        Vec::new(),
        OutputOptions {
            mode: OutputMode::JsonLines,
            ..OutputOptions::default()
        },
    );
    json.write_file(&non_utf8, &plan).unwrap();
    let value: Value = serde_json::from_slice(&json.into_inner()).unwrap();
    assert!(value["path"].is_null());
    assert_eq!(value["path_base64"], "YmFk/y50eHQ=");
    assert_eq!(value["path_encoding"], "bytes");
}

#[test]
fn explain_outputs_show_the_plan_and_match_provenance() {
    let plan = query_plan();
    let mut query = OutputWriter::new(Vec::new(), OutputOptions::default());
    query.write_query_plan(&plan).unwrap();
    let query = String::from_utf8(query.into_inner()).unwrap();
    assert!(query.contains("query: 걷다"));
    assert!(query.contains("alternation: d-to-l"));
    assert!(query.contains("  verifier_states:"));
    assert!(query.contains("normalization: nfc"));
    assert!(query.contains("estimated_matcher_bytes:"));

    let records = vec![SearchRecord::Line(match_line(1, "걸어\n".as_bytes()))];
    let mut matched = OutputWriter::new(
        Vec::new(),
        OutputOptions {
            explain_match: true,
            ..OutputOptions::default()
        },
    );
    matched.write_file(&result(records, 1), &plan).unwrap();
    let matched = String::from_utf8(matched.into_inner()).unwrap();
    assert!(matched.contains("generated_from: 걷다"));
    assert!(matched.contains("- lexical.d-to-l"));
}

#[test]
fn explain_query_reports_full_pos_preview_candidates_and_loaded_path() {
    let plan = query_plan();
    let candidates = [
        PathBuf::from("first/missing/lexicon.bin"),
        PathBuf::from("second/missing/lexicon.bin"),
    ];
    let preview = FullPosStatus::Preview {
        candidate_paths: candidates.clone().into(),
    };
    let mut output = OutputWriter::new(Vec::new(), OutputOptions::default());
    output
        .write_query_plan_with_full_pos(&plan, &preview)
        .unwrap();
    let text = String::from_utf8(output.into_inner()).unwrap();

    assert!(text.contains("status: preview (core lexicon only)"));
    let first = text.find("first/missing/lexicon.bin").unwrap();
    let second = text.find("second/missing/lexicon.bin").unwrap();
    assert!(first < second);

    let loaded = FullPosStatus::Loaded {
        path: PathBuf::from("share/kfind/lexicon.bin"),
    };
    let mut output = OutputWriter::new(Vec::new(), OutputOptions::default());
    output
        .write_query_plan_with_full_pos(&plan, &loaded)
        .unwrap();
    let text = String::from_utf8(output.into_inner()).unwrap();
    assert!(text.contains("status: loaded"));
    assert!(text.contains("path: share/kfind/lexicon.bin"));
}

#[test]
fn args_resolve_auto_color_and_filename_policy() {
    let args = Args::try_parse_from(["kfind", "걷다"]).unwrap();
    let options = OutputOptions::from_args(&args, true, true);
    assert_eq!(options.color, ResolvedColor::Enabled);
    assert!(options.with_filename());

    let json_args = Args::try_parse_from(["kfind", "--json", "걷다"]).unwrap();
    let json_options = OutputOptions::from_args(&json_args, true, false);
    assert_eq!(json_options.color, ResolvedColor::Disabled);
}

#[test]
fn broken_pipe_is_reported_without_hiding_its_kind() {
    struct BrokenWriter;

    impl Write for BrokenWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut output = OutputWriter::new(
        BrokenWriter,
        OutputOptions {
            mode: OutputMode::Count,
            ..OutputOptions::default()
        },
    );
    let error = output
        .write_file(&result(Vec::new(), 1), &query_plan())
        .unwrap_err();

    assert!(error.is_broken_pipe());
}
