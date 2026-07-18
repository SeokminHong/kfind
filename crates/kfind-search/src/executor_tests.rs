use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use kfind_matcher::MorphMatcher;
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};

use crate::{
    ExecutionOptions, ResultOrder, SearchConfig, SearchEvent, SearchRunError, WalkOptions,
    execute_search_with_stdin,
};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempTree(PathBuf);

struct CountingReader<R> {
    inner: R,
    bytes_read: Arc<AtomicUsize>,
}

impl<R: io::Read> io::Read for CountingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.bytes_read.fetch_add(read, Ordering::Relaxed);
        Ok(read)
    }
}

impl TempTree {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "kfind-executor-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: impl AsRef<Path>, contents: &str) -> PathBuf {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn matcher() -> Arc<MorphMatcher> {
    let lexicons = Arc::new(Lexicons::embedded().unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan = compile_query("걷다", &CompileOptions::default(), &analyzer).unwrap();
    Arc::new(MorphMatcher::new(Arc::new(plan)).unwrap())
}

fn config(paths: Vec<PathBuf>) -> SearchConfig {
    SearchConfig {
        paths,
        walk: WalkOptions {
            threads: Some(4),
            ..WalkOptions::default()
        },
        ..SearchConfig::default()
    }
}

fn collect(config: SearchConfig) -> (crate::SearchSummary, Vec<SearchEvent>) {
    let mut events = Vec::new();
    let summary = execute_search_with_stdin(matcher(), config, io::empty(), |event| {
        events.push(event.clone());
        Ok(())
    })
    .unwrap();
    (summary, events)
}

#[test]
fn parallel_results_are_delivered_as_complete_file_blocks() {
    let tree = TempTree::new();
    tree.write("a.txt", "길을 걸어 갔다.\n");
    tree.write("b.txt", "멈췄다.\n");
    tree.write("nested/c.txt", "걸어 보자.\n또 걸었다.\n");

    let (summary, events) = collect(config(vec![tree.0.clone()]));
    let file_results = events
        .iter()
        .filter_map(|event| match event {
            SearchEvent::FileEnd(result) => Some(result),
            SearchEvent::FileStart { .. } | SearchEvent::Record { .. } | SearchEvent::Issue(_) => {
                None
            }
        })
        .collect::<Vec<_>>();

    assert_eq!(summary.searched_files, 3);
    assert_eq!(summary.files_with_matches, 2);
    assert_eq!(summary.matching_lines, 3);
    assert_eq!(summary.errors, 0);
    assert_eq!(file_results.len(), 3);
    assert!(file_results.iter().all(|result| result.records.is_empty()));

    let mut active_path = None;
    for event in &events {
        match event {
            SearchEvent::FileStart { path } => {
                assert!(
                    active_path.replace(path).is_none(),
                    "file blocks interleaved"
                );
            }
            SearchEvent::Record { path, record } => {
                assert_eq!(active_path, Some(path));
                if let crate::SearchRecord::Line(line) = record {
                    assert!(line.line_number.is_some());
                }
            }
            SearchEvent::FileEnd(result) => {
                assert_eq!(active_path.take(), Some(&result.path));
            }
            SearchEvent::Issue(_) => {}
        }
    }
    assert!(active_path.is_none());
}

#[test]
fn path_order_streams_sorted_file_blocks() {
    let tree = TempTree::new();
    tree.write("z.txt", "걸어\n");
    tree.write("a.txt", "걸어\n");
    tree.write("m.txt", "걸어\n");
    let mut search = config(vec![tree.0.clone()]);
    search.execution.order = ResultOrder::Path;

    let (_, events) = collect(search);
    let paths = events
        .iter()
        .filter_map(SearchEvent::path)
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    let mut expected = paths.clone();
    expected.sort();

    assert_eq!(paths, expected);
}

#[test]
fn path_order_consumes_high_hit_records_before_eof() {
    const HIGH_HIT_LINES: usize = 20_000;

    let corpus = "걸어\n".repeat(HIGH_HIT_LINES).into_bytes();
    let corpus_bytes = corpus.len();
    let bytes_read = Arc::new(AtomicUsize::new(0));
    let records_seen = Arc::new(AtomicUsize::new(0));
    let reader = CountingReader {
        inner: io::Cursor::new(corpus),
        bytes_read: Arc::clone(&bytes_read),
    };
    let mut search = config(vec![PathBuf::from("-")]);
    search.execution.order = ResultOrder::Path;
    search.execution.channel_capacity = 1;
    let callback_records = Arc::clone(&records_seen);

    let summary = execute_search_with_stdin(matcher(), search, reader, move |event| {
        if matches!(event, SearchEvent::Record { .. }) {
            callback_records.fetch_add(1, Ordering::Relaxed);
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
        }
        Ok(())
    })
    .unwrap();

    assert!(summary.output_closed);
    assert_eq!(records_seen.load(Ordering::Relaxed), 1);
    assert!(
        bytes_read.load(Ordering::Relaxed) < corpus_bytes,
        "path-ordered search buffered the complete high-hit input"
    );
}

#[test]
fn path_order_unit_capacity_drains_parallel_file_streams() {
    const FILES: usize = 32;
    const LINES_PER_FILE: usize = 128;

    let tree = TempTree::new();
    for index in (0..FILES).rev() {
        tree.write(format!("{index:02}.txt"), &"걸어\n".repeat(LINES_PER_FILE));
    }
    let mut search = config(vec![tree.0.clone()]);
    search.execution.order = ResultOrder::Path;
    search.execution.channel_capacity = 0;

    let (summary, events) = collect(search);
    let paths = events
        .iter()
        .filter_map(|event| match event {
            SearchEvent::FileEnd(result) => Some(result.path.clone()),
            SearchEvent::FileStart { .. } | SearchEvent::Record { .. } | SearchEvent::Issue(_) => {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut expected = paths.clone();
    expected.sort();

    assert_eq!(summary.searched_files, FILES as u64);
    assert_eq!(summary.matching_lines, (FILES * LINES_PER_FILE) as u64);
    assert_eq!(summary.errors, 0);
    assert_eq!(paths, expected);
}

#[test]
fn ignored_files_are_skipped_but_explicit_files_are_searched() {
    let tree = TempTree::new();
    tree.write(".git/HEAD", "ref: refs/heads/main\n");
    tree.write(".gitignore", "ignored.txt\n");
    let ignored = tree.write("ignored.txt", "걸어\n");
    tree.write("visible.txt", "걸어\n");

    let (_, walked) = collect(config(vec![tree.0.clone()]));
    assert!(!walked.iter().any(|event| event.path() == Some(&ignored)));

    let (summary, explicit) = collect(config(vec![ignored.clone()]));
    assert!(summary.has_match);
    assert_eq!(explicit[0].path(), Some(ignored.as_path()));
}

#[test]
fn quiet_stops_after_the_first_global_match() {
    let tree = TempTree::new();
    for index in 0..64 {
        tree.write(format!("{index:02}.txt"), "걸어\n걸었다\n");
    }
    let mut search = config(vec![tree.0.clone()]);
    search.execution.quiet = true;

    let (summary, events) = collect(search);

    assert!(summary.has_match);
    assert_eq!(summary.searched_files, 1);
    assert_eq!(summary.matching_lines, 1);
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], SearchEvent::FileEnd(_)));
}

#[test]
fn repeated_stdin_paths_are_read_once() {
    let search = config(vec![PathBuf::from("-"), PathBuf::from("-")]);
    let mut events = Vec::new();

    let summary = execute_search_with_stdin(
        matcher(),
        search,
        "길을 걸어 갔다.\n".as_bytes(),
        |event| {
            events.push(event.clone());
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(summary.searched_files, 1);
    assert_eq!(summary.matching_lines, 1);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].path(), Some(Path::new("-")));
    assert!(matches!(events[1], SearchEvent::Record { .. }));
    assert!(matches!(events[2], SearchEvent::FileEnd(_)));
}

#[test]
fn default_output_consumes_high_hit_records_before_eof() {
    const HIGH_HIT_LINES: usize = 20_000;

    let corpus = "걸어\n".repeat(HIGH_HIT_LINES).into_bytes();
    let corpus_bytes = corpus.len();
    let bytes_read = Arc::new(AtomicUsize::new(0));
    let records_seen = Arc::new(AtomicUsize::new(0));
    let reader = CountingReader {
        inner: io::Cursor::new(corpus),
        bytes_read: Arc::clone(&bytes_read),
    };
    let mut search = config(vec![PathBuf::from("-")]);
    search.execution.channel_capacity = 1;
    let callback_records = Arc::clone(&records_seen);

    let summary = execute_search_with_stdin(matcher(), search, reader, move |event| {
        if matches!(event, SearchEvent::Record { .. }) {
            callback_records.fetch_add(1, Ordering::Relaxed);
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
        }
        Ok(())
    })
    .unwrap();

    assert!(summary.output_closed);
    assert_eq!(records_seen.load(Ordering::Relaxed), 1);
    assert!(
        bytes_read.load(Ordering::Relaxed) < corpus_bytes,
        "search buffered the complete high-hit input before delivering a record"
    );
}

#[test]
fn walk_errors_are_reported_while_other_paths_continue() {
    let tree = TempTree::new();
    let valid = tree.write("valid.txt", "걸어\n");
    let missing = tree.0.join("missing.txt");

    let (summary, events) = collect(config(vec![missing.clone(), valid.clone()]));

    assert_eq!(summary.errors, 1);
    assert_eq!(summary.searched_files, 1);
    assert!(summary.has_match);
    assert!(events.iter().any(|event| event.path() == Some(&missing)));
    assert!(events.iter().any(|event| event.path() == Some(&valid)));
}

#[test]
fn output_errors_and_panics_are_returned_without_unwinding() {
    let tree = TempTree::new();
    let path = tree.write("match.txt", "걸어\n");
    let output_error =
        execute_search_with_stdin(matcher(), config(vec![path.clone()]), io::empty(), |_| {
            Err(io::Error::other("writer failed"))
        });
    assert!(matches!(output_error, Err(SearchRunError::Output(_))));

    let panic_error = execute_search_with_stdin(
        matcher(),
        config(vec![path]),
        io::empty(),
        |_| -> io::Result<()> { panic!("callback failed") },
    );
    assert!(matches!(
        panic_error,
        Err(SearchRunError::CallbackPanic(message)) if message == "callback failed"
    ));
}

#[test]
fn broken_pipe_is_a_normal_early_exit() {
    let tree = TempTree::new();
    let path = tree.write("match.txt", "걸어\n");

    let summary = execute_search_with_stdin(matcher(), config(vec![path]), io::empty(), |_| {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    })
    .unwrap();

    assert!(summary.output_closed);
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_paths_remain_pathbufs() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let tree = TempTree::new();
    let path = tree.write(
        PathBuf::from(OsString::from_vec(b"bad-\xff.txt".to_vec())),
        "걸어\n",
    );

    let (_, events) = collect(config(vec![path.clone()]));

    assert_eq!(events[0].path(), Some(path.as_path()));
}

#[cfg(unix)]
#[test]
fn non_utf8_walk_error_paths_remain_pathbufs() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let tree = TempTree::new();
    let path = tree
        .0
        .join(PathBuf::from(OsString::from_vec(b"missing-\xff".to_vec())));

    let (summary, events) = collect(config(vec![path.clone()]));

    assert_eq!(summary.errors, 1);
    assert_eq!(events[0].path(), Some(path.as_path()));
}

#[test]
fn zero_channel_capacity_still_uses_a_bounded_handoff() {
    let tree = TempTree::new();
    let path = tree.write("match.txt", "걸어\n");
    let mut search = config(vec![path]);
    search.execution = ExecutionOptions {
        channel_capacity: 0,
        ..ExecutionOptions::default()
    };

    let (summary, _) = collect(search);

    assert!(summary.has_match);
}
