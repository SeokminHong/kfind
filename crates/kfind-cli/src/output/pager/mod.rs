mod protocol;
mod render;
mod terminal;
mod viewport;

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

use crate::Args;

use super::OutputMode;

pub(super) use protocol::{ColumnRange, MatchLine, PagerMatch, write_match_line};

const LIVE_FLUSH_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PagerEvent {
    Data,
    Done,
}

pub struct TerminalPager {
    _file: NamedTempFile,
    writer: Option<LiveWriter>,
    events: SyncSender<PagerEvent>,
    presenter: Option<JoinHandle<io::Result<()>>>,
}

impl TerminalPager {
    #[must_use]
    pub fn from_args(args: &Args, terminal_io: bool) -> Option<Self> {
        if !should_page(args, terminal_io) {
            return None;
        }

        let file = NamedTempFile::new().ok()?;
        let writer_file = file.reopen().ok()?;
        let presenter_file = file.reopen().ok()?;
        let (event_sender, event_receiver) = mpsc::sync_channel(1);
        let (ready_sender, ready_receiver) = mpsc::channel();
        let presenter = thread::Builder::new()
            .name("kfind-tui".to_owned())
            .spawn(move || terminal::present_live(presenter_file, event_receiver, ready_sender))
            .ok()?;

        match ready_receiver.recv().ok()? {
            Ok(()) => Some(Self {
                _file: file,
                writer: Some(LiveWriter::new(writer_file, event_sender.clone())),
                events: event_sender,
                presenter: Some(presenter),
            }),
            Err(_) => {
                let _ = presenter.join();
                None
            }
        }
    }

    #[doc(hidden)]
    pub fn writer(&mut self) -> &mut (impl Write + Send) {
        self.writer
            .as_mut()
            .expect("terminal pager writer is available before finish")
    }

    #[doc(hidden)]
    pub fn finish(mut self) -> io::Result<()> {
        let flush_result = self
            .writer
            .take()
            .map_or(Ok(()), |mut writer| writer.flush());
        let _ = self.events.send(PagerEvent::Done);
        let presenter_result = self
            .presenter
            .take()
            .expect("terminal presenter is available before finish")
            .join()
            .map_err(|_| io::Error::other("TUI presenter panicked"))?;

        match flush_result {
            Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error),
            _ => presenter_result,
        }
    }
}

struct LiveWriter {
    inner: BufWriter<File>,
    events: SyncSender<PagerEvent>,
    last_flush: Instant,
    has_flushed: bool,
}

impl LiveWriter {
    fn new(file: File, events: SyncSender<PagerEvent>) -> Self {
        Self {
            inner: BufWriter::new(file),
            events,
            last_flush: Instant::now(),
            has_flushed: false,
        }
    }

    fn publish(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        match self.events.try_send(PagerEvent::Data) {
            Ok(()) | Err(TrySendError::Full(PagerEvent::Data)) => {}
            Err(TrySendError::Full(PagerEvent::Done)) => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "TUI presenter is finishing",
                ));
            }
            Err(TrySendError::Disconnected(_)) => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "TUI presenter stopped",
                ));
            }
        }
        self.last_flush = Instant::now();
        self.has_flushed = true;
        Ok(())
    }
}

impl Write for LiveWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        if buffer[..written].contains(&b'\n')
            && (!self.has_flushed || self.last_flush.elapsed() >= LIVE_FLUSH_INTERVAL)
        {
            self.publish()?;
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.publish()
    }
}

fn should_page(args: &Args, terminal_io: bool) -> bool {
    terminal_io
        && !args.init
        && !args.no_pager
        && !args
            .paths
            .iter()
            .any(|path| path == std::path::Path::new("-"))
        && OutputMode::from_args(args) == OutputMode::Standard
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args(extra: &[&str]) -> Args {
        let mut values = vec!["kfind"];
        values.extend_from_slice(extra);
        values.push("walk");
        Args::try_parse_from(values).unwrap()
    }

    #[test]
    fn pages_only_standard_terminal_output() {
        assert!(should_page(&args(&[]), true));
        assert!(!should_page(&args(&[]), false));
        assert!(!should_page(&args(&["--no-pager"]), true));
        assert!(!should_page(&args(&["--json"]), true));
        assert!(!should_page(&args(&["--count"]), true));
        assert!(!should_page(&args(&["--files-with-matches"]), true));
        assert!(!should_page(&args(&["--quiet"]), true));
        let explicit_stdin = Args::try_parse_from(["kfind", "walk", "-"]).unwrap();
        assert!(!should_page(&explicit_stdin, true));
    }

    #[test]
    fn live_writer_publishes_the_first_complete_line() {
        let file = tempfile::tempfile().unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut writer = LiveWriter::new(file, sender);

        writer.write_all(b"first line\n").unwrap();

        assert_eq!(receiver.try_recv(), Ok(PagerEvent::Data));
    }
}
