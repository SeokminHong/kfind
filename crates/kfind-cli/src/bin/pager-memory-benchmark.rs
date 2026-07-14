use std::env;
use std::error::Error;

use kfind_cli::run_pager_memory_benchmark;

fn main() -> Result<(), Box<dyn Error>> {
    let (source_lines, matches_per_line, terminal_width) = arguments()?;
    let report = run_pager_memory_benchmark(source_lines, matches_per_line, terminal_width)?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

fn arguments() -> Result<(usize, usize, usize), Box<dyn Error>> {
    let mut values = env::args().skip(1);
    let source_lines = parse_argument(values.next(), "SOURCE_LINES")?;
    let matches_per_line = parse_argument(values.next(), "MATCHES_PER_LINE")?;
    let terminal_width = parse_argument(values.next(), "TERMINAL_WIDTH")?;
    if values.next().is_some() {
        return Err(
            "usage: kfind-pager-memory-benchmark SOURCE_LINES MATCHES_PER_LINE TERMINAL_WIDTH"
                .into(),
        );
    }
    Ok((source_lines, matches_per_line, terminal_width))
}

fn parse_argument(value: Option<String>, name: &str) -> Result<usize, Box<dyn Error>> {
    value
        .ok_or_else(|| {
            format!(
                "missing {name}; usage: kfind-pager-memory-benchmark SOURCE_LINES MATCHES_PER_LINE TERMINAL_WIDTH"
            )
        })?
        .parse()
        .map_err(|error| format!("invalid {name}: {error}").into())
}
