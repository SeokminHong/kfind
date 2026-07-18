use std::io::{self, Read};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use ignore::{DirEntry, Error as IgnoreError, WalkParallel, WalkState};
use kfind_matcher::MorphMatcher;

use super::writer::{FileMessage, FileStream, WorkerEvent, write_events};
use super::{
    ExecutionOptions, InputOptions, InputSearcher, ResultOrder, SearchIssue, SearchIssueKind,
    SearchRunError, SearchSummary, panic_message, send_file_message, send_stdin, send_worker_event,
};

enum OrderedSource {
    File(PathBuf),
    Stdin,
    Issue(SearchIssue),
}

impl OrderedSource {
    fn path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Stdin => Some(Path::new("-")),
            Self::Issue(issue) => issue.path.as_deref(),
        }
    }
}

struct FileJob {
    path: PathBuf,
    sender: Sender<FileMessage>,
}

pub(super) struct OrderedExecution<R> {
    pub matcher: Arc<MorphMatcher>,
    pub input_options: InputOptions,
    pub execution_options: ExecutionOptions,
    pub stdin_searcher: InputSearcher,
    pub stdin: R,
    pub walker: Option<WalkParallel>,
    pub search_stdin: bool,
    pub worker_threads: usize,
}

pub(super) fn execute<R, F>(
    execution: OrderedExecution<R>,
    callback: F,
) -> Result<SearchSummary, SearchRunError>
where
    R: Read,
    F: FnMut(&super::SearchEvent) -> io::Result<()> + Send,
{
    let OrderedExecution {
        matcher,
        input_options,
        mut execution_options,
        mut stdin_searcher,
        stdin,
        walker,
        search_stdin,
        worker_threads,
    } = execution;
    let sources = collect_sources(walker, search_stdin);
    let mut stdin = Some(stdin);
    let capacity = execution_options.channel_capacity.max(1);
    let (sender, receiver) = bounded::<WorkerEvent>(capacity);
    let cancelled = Arc::new(AtomicBool::new(false));

    thread::scope(|scope| {
        let writer_cancelled = Arc::clone(&cancelled);
        execution_options.order = ResultOrder::Unspecified;
        let writer = scope
            .spawn(move || write_events(receiver, writer_cancelled, execution_options, callback));
        let (job_sender, job_receiver) = bounded::<FileJob>(worker_threads.max(1));
        let mut workers = Vec::with_capacity(worker_threads.max(1));
        for _ in 0..worker_threads.max(1) {
            let jobs = job_receiver.clone();
            let matcher = Arc::clone(&matcher);
            let cancelled = Arc::clone(&cancelled);
            workers
                .push(scope.spawn(move || run_worker(jobs, &matcher, input_options, &cancelled)));
        }
        drop(job_receiver);

        for source in sources {
            if cancelled.load(Ordering::Acquire) {
                break;
            }
            match source {
                OrderedSource::File(path) => {
                    let (stream_sender, stream_receiver) = bounded(capacity);
                    if !send_worker_event(
                        &sender,
                        &cancelled,
                        WorkerEvent::File(FileStream {
                            path: path.clone(),
                            receiver: stream_receiver,
                        }),
                    ) {
                        break;
                    }
                    if job_sender
                        .send(FileJob {
                            path,
                            sender: stream_sender,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                OrderedSource::Stdin => {
                    if let Some(stdin) = stdin.take() {
                        send_stdin(
                            &mut stdin_searcher,
                            &matcher,
                            stdin,
                            &sender,
                            &cancelled,
                            capacity,
                            input_options.capture_records,
                        );
                    }
                }
                OrderedSource::Issue(issue) => {
                    if !send_worker_event(&sender, &cancelled, WorkerEvent::Issue(issue)) {
                        break;
                    }
                }
            }
        }
        drop(job_sender);

        for worker in workers {
            if let Err(payload) = worker.join() {
                send_worker_event(
                    &sender,
                    &cancelled,
                    WorkerEvent::Issue(SearchIssue::worker_panic(None, payload)),
                );
            }
        }
        drop(sender);

        writer
            .join()
            .map_err(|payload| SearchRunError::WriterPanic(panic_message(payload)))?
    })
}

fn collect_sources(walker: Option<WalkParallel>, search_stdin: bool) -> Vec<OrderedSource> {
    let (sender, receiver) = unbounded();
    if let Some(walker) = walker {
        let traversal_sender = sender.clone();
        let traversal = panic::catch_unwind(AssertUnwindSafe(|| {
            walker.run(|| {
                let sender = traversal_sender.clone();
                Box::new(move |entry| {
                    collect_entry(entry, &sender);
                    WalkState::Continue
                })
            });
        }));
        if let Err(payload) = traversal {
            let _ = sender.send(OrderedSource::Issue(SearchIssue::worker_panic(
                None, payload,
            )));
        }
    }
    if search_stdin {
        let _ = sender.send(OrderedSource::Stdin);
    }
    drop(sender);

    let mut sources = receiver.into_iter().collect::<Vec<_>>();
    sources.sort_by(|left, right| left.path().cmp(&right.path()));
    sources
}

fn collect_entry(entry: Result<DirEntry, IgnoreError>, sender: &Sender<OrderedSource>) {
    let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
            let _ = sender.send(OrderedSource::Issue(SearchIssue::walk(&error)));
            return;
        }
    };
    if let Some(error) = entry.error() {
        let _ = sender.send(OrderedSource::Issue(SearchIssue::walk(error)));
    }
    if entry
        .file_type()
        .is_some_and(|file_type| file_type.is_file())
    {
        let _ = sender.send(OrderedSource::File(entry.into_path()));
    }
}

fn run_worker(
    jobs: Receiver<FileJob>,
    matcher: &MorphMatcher,
    input_options: InputOptions,
    cancelled: &AtomicBool,
) {
    let mut searcher = match InputSearcher::new(input_options) {
        Ok(searcher) => searcher,
        Err(error) => {
            for job in jobs {
                let issue = SearchIssue {
                    kind: SearchIssueKind::Input,
                    path: Some(job.path),
                    message: error.to_string(),
                };
                send_file_message(&job.sender, cancelled, FileMessage::Issue(issue));
            }
            return;
        }
    };

    for job in jobs {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        let FileJob { path, sender } = job;
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            searcher.search_path_stream(matcher, &path, |record| {
                send_file_message(&sender, cancelled, FileMessage::Record(record))
            })
        }));
        let message = match result {
            Ok(Ok(result)) => FileMessage::Finished(result),
            Ok(Err(error)) => FileMessage::Issue(SearchIssue::input(path, &error)),
            Err(payload) => FileMessage::Issue(SearchIssue::worker_panic(Some(path), payload)),
        };
        send_file_message(&sender, cancelled, message);
    }
}
