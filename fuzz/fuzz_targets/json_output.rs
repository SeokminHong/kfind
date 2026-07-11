#![no_main]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use kfind_cli::{OutputMode, OutputOptions, OutputWriter};
use kfind_matcher::MorphMatcher;
use kfind_query::QueryPlan;
use kfind_search::{FileSearchResult, SearchLine, SearchLineKind, SearchRecord};
use libfuzzer_sys::fuzz_target;

mod support;

fuzz_target!(|data: &[u8]| {
    let fixture = fixture();
    let matches = fixture.matcher.find_all_with_meta(data);
    let matched_spans = matches.len() as u64;
    let result = FileSearchResult {
        path: PathBuf::from("<fuzz>"),
        records: vec![
            SearchRecord::Line(SearchLine {
                kind: SearchLineKind::Match,
                line_number: Some(1),
                absolute_byte_offset: 0,
                bytes: data.to_vec(),
                matches,
            }),
            SearchRecord::ContextBreak,
        ],
        matching_lines: 1,
        matched_spans: Some(matched_spans),
        binary_byte_offset: None,
    };
    let options = OutputOptions {
        mode: OutputMode::JsonLines,
        column: true,
        ..OutputOptions::default()
    };
    let mut output = OutputWriter::new(Vec::new(), options);
    output
        .write_file(&result, &fixture.plan)
        .expect("writing JSON to memory must succeed");

    for line in output.into_inner().split(|byte| *byte == b'\n') {
        if !line.is_empty() {
            serde_json::from_slice::<serde_json::Value>(line)
                .expect("each output record must be valid JSON");
        }
    }
});

struct Fixture {
    plan: Arc<QueryPlan>,
    matcher: MorphMatcher,
}

fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let (plan, matcher) = support::build_match_fixture();
        Fixture { plan, matcher }
    })
}
