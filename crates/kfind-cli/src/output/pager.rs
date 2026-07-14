use std::ffi::OsStr;
use std::io::{self, BufWriter, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use crate::Args;

use super::OutputMode;

const PAGER_PROGRAM: &str = "less";
const PAGER_OPTIONS: &str = "-RFSX";

pub struct TerminalPager {
    writer: Option<BufWriter<ChildStdin>>,
    child: Child,
}

impl TerminalPager {
    #[must_use]
    pub fn from_args(args: &Args, stdout_is_terminal: bool) -> Option<Self> {
        should_page(args, stdout_is_terminal)
            .then(|| spawn_pager(PAGER_PROGRAM))
            .flatten()
    }

    pub fn writer(&mut self) -> &mut BufWriter<ChildStdin> {
        self.writer.as_mut().expect("the pager writer is available")
    }

    pub fn finish(mut self) -> io::Result<()> {
        let mut writer = self.writer.take().expect("the pager writer is available");
        let flush_result = writer.flush();
        drop(writer);
        let status = self.child.wait()?;
        finish_flush(flush_result)?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "pager exited with status {status}"
            )))
        }
    }
}

fn finish_flush(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

fn should_page(args: &Args, stdout_is_terminal: bool) -> bool {
    stdout_is_terminal
        && !args.init
        && !args.no_pager
        && OutputMode::from_args(args) == OutputMode::Standard
}

fn spawn_pager(program: impl AsRef<OsStr>) -> Option<TerminalPager> {
    let mut child = pager_command(program).spawn().ok()?;
    let Some(writer) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    Some(TerminalPager {
        writer: Some(BufWriter::new(writer)),
        child,
    })
}

fn pager_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command
        .arg(PAGER_OPTIONS)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    command
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn args(extra: &[&str]) -> Args {
        let mut values = vec!["kfind"];
        values.extend_from_slice(extra);
        values.push("걷다");
        Args::try_parse_from(values).unwrap()
    }

    #[test]
    fn only_standard_terminal_output_uses_the_pager() {
        assert!(should_page(&args(&[]), true));
        assert!(!should_page(&args(&[]), false));
        assert!(!should_page(&args(&["--no-pager"]), true));

        for option in ["--json", "--count", "--files-with-matches", "--quiet"] {
            assert!(!should_page(&args(&[option]), true));
        }
    }

    #[test]
    fn pager_chops_long_lines_and_preserves_terminal_output() {
        let command = pager_command(PAGER_PROGRAM);

        assert_eq!(command.get_program(), PAGER_PROGRAM);
        assert_eq!(command.get_args().collect::<Vec<_>>(), [PAGER_OPTIONS]);
    }

    #[test]
    fn missing_pager_falls_back_without_starting_a_child() {
        assert!(spawn_pager("kfind-pager-that-does-not-exist").is_none());
    }

    #[test]
    fn finishing_output_treats_a_broken_pipe_as_normal() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "closed");

        assert!(finish_flush(Err(error)).is_ok());
    }
}
