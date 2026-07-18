use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Receiver;

use super::{
    ExecutionOptions, FileSearchResult, SearchEvent, SearchIssue, SearchIssueKind, SearchRecord,
    SearchRunError, SearchSummary, panic_message,
};

pub(super) enum WorkerEvent {
    File(FileStream),
    Completed(FileSearchResult),
    Issue(SearchIssue),
}

pub(super) struct FileStream {
    pub path: PathBuf,
    pub receiver: Receiver<FileMessage>,
}

pub(super) enum FileMessage {
    Record(SearchRecord),
    Finished(FileSearchResult),
    Issue(SearchIssue),
}

pub(super) fn write_events<F>(
    receiver: Receiver<WorkerEvent>,
    cancelled: Arc<AtomicBool>,
    options: ExecutionOptions,
    mut callback: F,
) -> Result<SearchSummary, SearchRunError>
where
    F: FnMut(&SearchEvent) -> io::Result<()>,
{
    let mut summary = SearchSummary::default();
    for event in receiver {
        let keep_writing = match event {
            WorkerEvent::File(stream) => {
                write_file_stream(stream, &mut summary, &cancelled, options, &mut callback)?
            }
            WorkerEvent::Completed(result) => write_search_event(
                SearchEvent::FileEnd(result),
                &mut summary,
                &cancelled,
                options,
                &mut callback,
            )?,
            WorkerEvent::Issue(issue) => write_search_event(
                SearchEvent::Issue(issue),
                &mut summary,
                &cancelled,
                options,
                &mut callback,
            )?,
        };
        if !keep_writing {
            break;
        }
    }
    Ok(summary)
}

fn write_file_stream<F>(
    stream: FileStream,
    summary: &mut SearchSummary,
    cancelled: &AtomicBool,
    options: ExecutionOptions,
    callback: &mut F,
) -> Result<bool, SearchRunError>
where
    F: FnMut(&SearchEvent) -> io::Result<()>,
{
    let path = stream.path;
    if !write_search_event(
        SearchEvent::FileStart { path: path.clone() },
        summary,
        cancelled,
        options,
        callback,
    )? {
        return Ok(false);
    }
    for message in stream.receiver {
        let event = match message {
            FileMessage::Record(record) => SearchEvent::Record {
                path: path.clone(),
                record,
            },
            FileMessage::Finished(result) => SearchEvent::FileEnd(result),
            FileMessage::Issue(issue) => SearchEvent::Issue(issue),
        };
        let complete = matches!(event, SearchEvent::FileEnd(_) | SearchEvent::Issue(_));
        if !write_search_event(event, summary, cancelled, options, callback)? {
            return Ok(false);
        }
        if complete {
            return Ok(true);
        }
    }
    write_search_event(
        SearchEvent::Issue(disconnected_stream_issue(path)),
        summary,
        cancelled,
        options,
        callback,
    )
}

fn write_search_event<F>(
    event: SearchEvent,
    summary: &mut SearchSummary,
    cancelled: &AtomicBool,
    options: ExecutionOptions,
    callback: &mut F,
) -> Result<bool, SearchRunError>
where
    F: FnMut(&SearchEvent) -> io::Result<()>,
{
    let stop_after_event = options.quiet && event.has_match();
    if stop_after_event {
        cancelled.store(true, Ordering::Release);
    }
    Ok(write_event(&event, summary, cancelled, callback)? && !stop_after_event)
}

fn disconnected_stream_issue(path: PathBuf) -> SearchIssue {
    SearchIssue {
        kind: SearchIssueKind::WorkerPanic,
        path: Some(path),
        message: "file search stream closed before completion".to_owned(),
    }
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
