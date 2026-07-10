use std::any::Any;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded};
use ignore::{DirEntry, Error as IgnoreError, WalkParallel, WalkState};
use kfind_matcher::MorphMatcher;

use crate::{
    FileSearchResult, InputOptions, InputSearchError, InputSearcher, WalkConfigError, WalkOptions,
    build_walker,
};

const DEFAULT_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResultOrder {
    #[default]
    Unspecified,
    Path,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionOptions {
    pub quiet: bool,
    pub order: ResultOrder,
    pub channel_capacity: usize,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            quiet: false,
            order: ResultOrder::Unspecified,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchConfig {
    pub paths: Vec<PathBuf>,
    pub walk: WalkOptions,
    pub input: InputOptions,
    pub execution: ExecutionOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchIssueKind {
    Walk,
    Input,
    WorkerPanic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchIssue {
    pub kind: SearchIssueKind,
    pub path: Option<PathBuf>,
    pub message: String,
}

impl SearchIssue {
    fn walk(error: &IgnoreError) -> Self {
        Self {
            kind: SearchIssueKind::Walk,
            path: ignore_error_path(error),
            message: error.to_string(),
        }
    }

    fn input(path: PathBuf, error: &InputSearchError) -> Self {
        Self {
            kind: SearchIssueKind::Input,
            path: Some(path),
            message: error.to_string(),
        }
    }

    fn worker_panic(path: Option<PathBuf>, payload: Box<dyn Any + Send>) -> Self {
        Self {
            kind: SearchIssueKind::WorkerPanic,
            path,
            message: panic_message(payload),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchEvent {
    File(FileSearchResult),
    Issue(SearchIssue),
}

impl SearchEvent {
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(result) => Some(&result.path),
            Self::Issue(issue) => issue.path.as_deref(),
        }
    }

    fn has_match(&self) -> bool {
        matches!(self, Self::File(result) if result.has_match())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SearchSummary {
    pub has_match: bool,
    pub searched_files: u64,
    pub files_with_matches: u64,
    pub matching_lines: u64,
    pub errors: u64,
    pub output_closed: bool,
}

impl SearchSummary {
    fn observe(&mut self, event: &SearchEvent) {
        match event {
            SearchEvent::File(result) => {
                self.searched_files += 1;
                self.matching_lines += result.matching_lines;
                if result.has_match() {
                    self.has_match = true;
                    self.files_with_matches += 1;
                }
            }
            SearchEvent::Issue(_) => self.errors += 1,
        }
    }
}

pub fn execute_search<F>(
    matcher: Arc<MorphMatcher>,
    config: SearchConfig,
    callback: F,
) -> Result<SearchSummary, SearchRunError>
where
    F: FnMut(&SearchEvent) -> io::Result<()> + Send,
{
    let stdin = io::stdin();
    execute_search_with_stdin(matcher, config, stdin.lock(), callback)
}

pub fn execute_search_with_stdin<R, F>(
    matcher: Arc<MorphMatcher>,
    config: SearchConfig,
    stdin: R,
    callback: F,
) -> Result<SearchSummary, SearchRunError>
where
    R: Read,
    F: FnMut(&SearchEvent) -> io::Result<()> + Send,
{
    if config.paths.is_empty() {
        return Err(SearchRunError::Walk(WalkConfigError::NoPaths));
    }

    let mut input_options = config.input;
    if config.execution.quiet {
        input_options.stop_after_first_match = true;
    }
    let mut stdin_searcher = InputSearcher::new(input_options).map_err(SearchRunError::Input)?;
    let (file_paths, search_stdin) = split_paths(&config.paths);
    let walker = (!file_paths.is_empty())
        .then(|| build_walker(&file_paths, &config.walk))
        .transpose()
        .map_err(SearchRunError::Walk)?;
    let capacity = config.execution.channel_capacity.max(1);
    let (sender, receiver) = bounded(capacity);
    let cancelled = Arc::new(AtomicBool::new(false));

    thread::scope(|scope| {
        let writer_cancelled = Arc::clone(&cancelled);
        let execution = config.execution;
        let writer =
            scope.spawn(move || write_events(receiver, writer_cancelled, execution, callback));

        if search_stdin && !cancelled.load(Ordering::Acquire) {
            send_stdin(&mut stdin_searcher, &matcher, stdin, &sender, &cancelled);
        }
        if !cancelled.load(Ordering::Acquire) {
            if let Some(walker) = walker {
                run_walker(
                    walker,
                    Arc::clone(&matcher),
                    input_options,
                    &sender,
                    Arc::clone(&cancelled),
                );
            }
        }
        drop(sender);

        writer
            .join()
            .map_err(|payload| SearchRunError::WriterPanic(panic_message(payload)))?
    })
}

fn split_paths(paths: &[PathBuf]) -> (Vec<PathBuf>, bool) {
    let mut search_stdin = false;
    let files = paths
        .iter()
        .filter_map(|path| {
            if path == Path::new("-") {
                search_stdin = true;
                None
            } else {
                Some(path.clone())
            }
        })
        .collect();
    (files, search_stdin)
}

fn send_stdin(
    searcher: &mut InputSearcher,
    matcher: &MorphMatcher,
    stdin: impl Read,
    sender: &Sender<SearchEvent>,
    cancelled: &AtomicBool,
) {
    let path = PathBuf::from("-");
    let event = match panic::catch_unwind(AssertUnwindSafe(|| {
        searcher.search_reader(matcher, path.clone(), stdin)
    })) {
        Ok(Ok(result)) => SearchEvent::File(result),
        Ok(Err(error)) => SearchEvent::Issue(SearchIssue::input(path, &error)),
        Err(payload) => SearchEvent::Issue(SearchIssue::worker_panic(Some(path), payload)),
    };
    send_event(sender, cancelled, event);
}

fn run_walker(
    walker: WalkParallel,
    matcher: Arc<MorphMatcher>,
    input_options: InputOptions,
    sender: &Sender<SearchEvent>,
    cancelled: Arc<AtomicBool>,
) {
    let initialization_error_sent = Arc::new(AtomicBool::new(false));
    let traversal = panic::catch_unwind(AssertUnwindSafe(|| {
        walker.run(|| {
            let matcher = Arc::clone(&matcher);
            let sender = sender.clone();
            let cancelled = Arc::clone(&cancelled);
            let initialization_error_sent = Arc::clone(&initialization_error_sent);
            let mut searcher = match InputSearcher::new(input_options) {
                Ok(searcher) => Some(searcher),
                Err(error) => {
                    if !initialization_error_sent.swap(true, Ordering::AcqRel) {
                        send_event(
                            &sender,
                            &cancelled,
                            SearchEvent::Issue(SearchIssue {
                                kind: SearchIssueKind::Input,
                                path: None,
                                message: error.to_string(),
                            }),
                        );
                    }
                    None
                }
            };
            Box::new(move |entry| {
                if cancelled.load(Ordering::Acquire) {
                    return WalkState::Quit;
                }
                let Some(searcher) = &mut searcher else {
                    return WalkState::Quit;
                };
                let panic_path = entry_path(&entry);
                match panic::catch_unwind(AssertUnwindSafe(|| {
                    process_entry(entry, searcher, &matcher, &sender, &cancelled)
                })) {
                    Ok(state) => state,
                    Err(payload) => {
                        if send_event(
                            &sender,
                            &cancelled,
                            SearchEvent::Issue(SearchIssue::worker_panic(panic_path, payload)),
                        ) {
                            WalkState::Continue
                        } else {
                            WalkState::Quit
                        }
                    }
                }
            })
        });
    }));

    if let Err(payload) = traversal {
        send_event(
            sender,
            &cancelled,
            SearchEvent::Issue(SearchIssue::worker_panic(None, payload)),
        );
    }
}

fn process_entry(
    entry: Result<DirEntry, IgnoreError>,
    searcher: &mut InputSearcher,
    matcher: &MorphMatcher,
    sender: &Sender<SearchEvent>,
    cancelled: &AtomicBool,
) -> WalkState {
    let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
            return send_state(
                sender,
                cancelled,
                SearchEvent::Issue(SearchIssue::walk(&error)),
            );
        }
    };

    if let Some(error) = entry.error() {
        if !send_event(
            sender,
            cancelled,
            SearchEvent::Issue(SearchIssue::walk(error)),
        ) {
            return WalkState::Quit;
        }
    }
    if !entry
        .file_type()
        .is_some_and(|file_type| file_type.is_file())
    {
        return WalkState::Continue;
    }

    let path = entry.into_path();
    let event = match searcher.search_path(matcher, &path) {
        Ok(result) => SearchEvent::File(result),
        Err(error) => SearchEvent::Issue(SearchIssue::input(path, &error)),
    };
    send_state(sender, cancelled, event)
}

fn send_state(
    sender: &Sender<SearchEvent>,
    cancelled: &AtomicBool,
    event: SearchEvent,
) -> WalkState {
    if send_event(sender, cancelled, event) {
        WalkState::Continue
    } else {
        WalkState::Quit
    }
}

fn send_event(sender: &Sender<SearchEvent>, cancelled: &AtomicBool, event: SearchEvent) -> bool {
    !cancelled.load(Ordering::Acquire) && sender.send(event).is_ok()
}

fn write_events<F>(
    receiver: Receiver<SearchEvent>,
    cancelled: Arc<AtomicBool>,
    options: ExecutionOptions,
    mut callback: F,
) -> Result<SearchSummary, SearchRunError>
where
    F: FnMut(&SearchEvent) -> io::Result<()>,
{
    let mut summary = SearchSummary::default();
    if options.order == ResultOrder::Path && !options.quiet {
        let mut events = receiver.iter().collect::<Vec<_>>();
        events.sort_by(|left, right| left.path().cmp(&right.path()));
        for event in events {
            if !write_event(&event, &mut summary, &cancelled, &mut callback)? {
                break;
            }
        }
        return Ok(summary);
    }

    for event in receiver {
        let stop_after_event = options.quiet && event.has_match();
        if stop_after_event {
            cancelled.store(true, Ordering::Release);
        }
        if !write_event(&event, &mut summary, &cancelled, &mut callback)? || stop_after_event {
            break;
        }
    }
    Ok(summary)
}

fn write_event<F>(
    event: &SearchEvent,
    summary: &mut SearchSummary,
    cancelled: &AtomicBool,
    callback: &mut F,
) -> Result<bool, SearchRunError>
where
    F: FnMut(&SearchEvent) -> io::Result<()>,
{
    summary.observe(event);
    match panic::catch_unwind(AssertUnwindSafe(|| callback(event))) {
        Ok(Ok(())) => Ok(true),
        Ok(Err(error)) if error.kind() == io::ErrorKind::BrokenPipe => {
            cancelled.store(true, Ordering::Release);
            summary.output_closed = true;
            Ok(false)
        }
        Ok(Err(error)) => {
            cancelled.store(true, Ordering::Release);
            Err(SearchRunError::Output(error))
        }
        Err(payload) => {
            cancelled.store(true, Ordering::Release);
            Err(SearchRunError::CallbackPanic(panic_message(payload)))
        }
    }
}

fn entry_path(entry: &Result<DirEntry, IgnoreError>) -> Option<PathBuf> {
    match entry {
        Ok(entry) => Some(entry.path().to_path_buf()),
        Err(error) => ignore_error_path(error),
    }
}

fn ignore_error_path(error: &IgnoreError) -> Option<PathBuf> {
    match error {
        IgnoreError::Partial(errors) => errors.iter().find_map(ignore_error_path),
        IgnoreError::WithLineNumber { err, .. } | IgnoreError::WithDepth { err, .. } => {
            ignore_error_path(err)
        }
        IgnoreError::WithPath { path, .. } => Some(path.clone()),
        IgnoreError::Loop { child, .. } => Some(child.clone()),
        IgnoreError::Io(_)
        | IgnoreError::Glob { .. }
        | IgnoreError::UnrecognizedFileType(_)
        | IgnoreError::InvalidDefinition => None,
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else {
        "panic without a string payload".to_owned()
    }
}

#[derive(Debug)]
pub enum SearchRunError {
    Walk(WalkConfigError),
    Input(InputSearchError),
    Output(io::Error),
    CallbackPanic(String),
    WriterPanic(String),
}

impl Display for SearchRunError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Walk(error) => Display::fmt(error, formatter),
            Self::Input(error) => Display::fmt(error, formatter),
            Self::Output(error) => write!(formatter, "failed to write search output: {error}"),
            Self::CallbackPanic(message) => {
                write!(formatter, "output callback panicked: {message}")
            }
            Self::WriterPanic(message) => write!(formatter, "output writer panicked: {message}"),
        }
    }
}

impl Error for SearchRunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Walk(error) => Some(error),
            Self::Input(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::CallbackPanic(_) | Self::WriterPanic(_) => None,
        }
    }
}
