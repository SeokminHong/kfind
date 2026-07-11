use kfind_cli::{ExitStatus, Language, parse_args_from, run_with_io};
use std::env;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let language = Language::from_env();
    let args = match parse_args_from(env::args_os(), language) {
        Ok(args) => args,
        Err(error) => {
            if error.use_stderr() {
                let _ = write!(io::stderr(), "{error}");
            } else {
                let _ = write!(io::stdout(), "{error}");
            }
            return ExitCode::from(error.exit_code());
        }
    };
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let stdin_is_terminal = stdin.is_terminal();
    let stdout_is_terminal = stdout.is_terminal();
    let mut stdout = BufWriter::new(stdout);
    let mut stderr = BufWriter::new(stderr);

    match run_with_io(
        &args,
        language,
        stdin.lock(),
        &mut stdout,
        &mut stderr,
        stdin_is_terminal,
        stdout_is_terminal,
    ) {
        Ok(status) => ExitCode::from(status.code()),
        Err(error) => {
            let _ = writeln!(stderr, "kfind: {}", error.localized(language));
            ExitCode::from(ExitStatus::Error.code())
        }
    }
}
