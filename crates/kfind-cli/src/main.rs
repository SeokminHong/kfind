use kfind_cli::{
    CliError, ExitStatus, Language, OutputError, TerminalPager, parse_args_from, run_init_with_io,
    run_with_io, write_cli_error,
};
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
    let stderr_is_terminal = stderr.is_terminal();
    let mut stderr = BufWriter::new(stderr);

    let result = if args.init {
        let mut stdout = BufWriter::new(stdout);
        run_init_with_io(
            &args,
            language,
            stdin.lock(),
            &mut stdout,
            &mut stderr,
            stdin_is_terminal && stderr_is_terminal,
        )
        .map(|()| ExitStatus::Match)
        .map_err(CliError::Init)
    } else if let Some(mut pager) = TerminalPager::from_args(&args, stdout_is_terminal) {
        let result = run_with_io(
            &args,
            language,
            stdin.lock(),
            pager.writer(),
            &mut stderr,
            stdin_is_terminal,
            stdout_is_terminal,
        );
        let finish_result = pager
            .finish()
            .map_err(OutputError::Io)
            .map_err(CliError::Output);
        result.and_then(|status| finish_result.map(|()| status))
    } else {
        let mut stdout = BufWriter::new(stdout);
        run_with_io(
            &args,
            language,
            stdin.lock(),
            &mut stdout,
            &mut stderr,
            stdin_is_terminal,
            stdout_is_terminal,
        )
    };

    match result {
        Ok(status) => ExitCode::from(status.code()),
        Err(error) => {
            let _ = write_cli_error(&mut stderr, &error, language);
            ExitCode::from(ExitStatus::Error.code())
        }
    }
}
