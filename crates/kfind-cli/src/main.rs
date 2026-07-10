use clap::Parser;
use kfind_cli::{Args, ExitStatus, run_with_io};
use std::io::{self, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let stdin_is_terminal = stdin.is_terminal();
    let stdout_is_terminal = stdout.is_terminal();
    let mut stdout = BufWriter::new(stdout);
    let mut stderr = BufWriter::new(stderr);

    match run_with_io(
        &args,
        stdin.lock(),
        &mut stdout,
        &mut stderr,
        stdin_is_terminal,
        stdout_is_terminal,
    ) {
        Ok(status) => ExitCode::from(status.code()),
        Err(error) => {
            let _ = writeln!(stderr, "kfind: {error}");
            ExitCode::from(ExitStatus::Error.code())
        }
    }
}
