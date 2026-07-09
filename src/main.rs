mod cli;
mod query;
mod scan;
mod walk;

use std::fs;
use std::io::{self, BufWriter, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse_env()?;
    let compiled = query::compile_query(&cli);
    let mut out = BufWriter::new(io::stdout().lock());
    if cli.explain_query {
        query::print_explain_query(&mut out, &compiled)?;
    }
    for path in walk::walk_paths(&cli) {
        if let Ok(bytes) = fs::read(&path) {
            let hits = scan::matching_hits(&compiled, &bytes);
            if hits.is_empty() {
                continue;
            }
            if cli.files_with_matches {
                writeln!(out, "{}", path.display())?;
            } else if cli.count {
                writeln!(out, "{}:{}", path.display(), hits.len())?;
            } else {
                scan::print_matches(&mut out, &cli, &path, &bytes, &hits)?;
            }
        }
    }
    Ok(())
}
