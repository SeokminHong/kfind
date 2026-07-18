#![no_main]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use kfind_matcher::MorphMatcher;
use kfind_search::{
    ExecutionOptions, InputOptions, ResultOrder, SearchConfig, SearchEvent, SearchSummary,
    WalkOptions, execute_search_with_stdin,
};
use libfuzzer_sys::fuzz_target;

mod support;

fuzz_target!(|data: &[u8]| {
    let before_context = data.first().map_or(0, |byte| usize::from(byte % 3));
    let after_context = data.get(1).map_or(0, |byte| usize::from(byte % 3));
    let channel_capacity = data.get(2).map_or(1, |byte| usize::from(byte % 4));
    let capture_records = data.get(3).is_none_or(|byte| byte & 1 == 0);
    let input = &data[data.len().min(4)..];

    let unspecified = execute(
        input,
        before_context,
        after_context,
        channel_capacity,
        capture_records,
        ResultOrder::Unspecified,
    );
    let ordered = execute(
        input,
        before_context,
        after_context,
        channel_capacity,
        capture_records,
        ResultOrder::Path,
    );

    assert_eq!(ordered, unspecified);
});

fn execute(
    input: &[u8],
    before_context: usize,
    after_context: usize,
    channel_capacity: usize,
    capture_records: bool,
    order: ResultOrder,
) -> (SearchSummary, Vec<SearchEvent>) {
    let config = SearchConfig {
        paths: vec![PathBuf::from("-")],
        walk: WalkOptions {
            threads: Some(1),
            ..WalkOptions::default()
        },
        input: InputOptions {
            before_context,
            after_context,
            capture_records,
            ..InputOptions::default()
        },
        execution: ExecutionOptions {
            order,
            channel_capacity,
            ..ExecutionOptions::default()
        },
    };
    let mut events = Vec::new();
    let mut active_path = None;
    let summary = execute_search_with_stdin(
        Arc::clone(matcher()),
        config,
        io::Cursor::new(input),
        |event| {
            match event {
                SearchEvent::FileStart { path } => {
                    assert!(active_path.replace(path.clone()).is_none());
                }
                SearchEvent::Record { path, record } => {
                    assert_eq!(active_path.as_deref(), Some(path.as_path()));
                    if let kfind_search::SearchRecord::Line(line) = record {
                        for matched in &line.matches {
                            assert!(matched.span.start <= matched.span.end);
                            assert!(matched.span.end <= line.bytes.len());
                        }
                    }
                }
                SearchEvent::FileEnd(result) => {
                    if capture_records {
                        assert_eq!(active_path.take().as_deref(), Some(result.path.as_path()));
                    } else {
                        assert!(active_path.is_none());
                    }
                }
                SearchEvent::Issue(issue) => {
                    assert_eq!(issue.path.as_deref(), Some(Path::new("-")));
                    if capture_records {
                        assert_eq!(active_path.take().as_deref(), Some(Path::new("-")));
                    } else {
                        assert!(active_path.is_none());
                    }
                }
            }
            events.push(event.clone());
            Ok(())
        },
    )
    .expect("in-memory executor search must not fail");
    assert!(active_path.is_none());
    (summary, events)
}

fn matcher() -> &'static Arc<MorphMatcher> {
    static MATCHER: OnceLock<Arc<MorphMatcher>> = OnceLock::new();
    MATCHER.get_or_init(|| Arc::new(support::build_match_fixture().1))
}
