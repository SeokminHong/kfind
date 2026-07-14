mod classify;
mod model;
mod output;

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use kfind_data::load_data_dir;

use crate::classify::collect_candidates;
use crate::model::{UsageError, core_entries, parse_source_records};
use crate::output::{write_predicates, write_report, write_stats};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1).map(PathBuf::from);
    let input = required_argument(&mut arguments, "normalized records TSV")?;
    let data_directory = required_argument(&mut arguments, "kfind data directory")?;
    let output_directory = required_argument(&mut arguments, "output directory")?;
    if arguments.next().is_some() {
        return Err(Box::new(UsageError::new(usage())));
    }

    let records = parse_source_records(&fs::read_to_string(input)?)?;
    let data = load_data_dir(data_directory)?;
    let core = core_entries(&data.lexicon.predicates);
    let candidates = collect_candidates(&records);
    fs::create_dir_all(&output_directory)?;
    write_predicates(
        &output_directory.join("predicates.tsv"),
        &candidates,
        &core,
        &data.rules.derivations,
    )?;
    write_report(
        &output_directory.join("REPORT.tsv"),
        &candidates,
        &core,
        &data.rules.derivations,
    )?;
    write_stats(
        &output_directory.join("STATS.toml"),
        &records,
        &candidates,
        &core,
        &data.rules.derivations,
    )?;
    Ok(())
}

fn required_argument(
    arguments: &mut impl Iterator<Item = PathBuf>,
    name: &str,
) -> Result<PathBuf, UsageError> {
    arguments
        .next()
        .ok_or_else(|| UsageError::new(format!("missing {name}\n{}", usage())))
}

fn usage() -> String {
    "usage: nikl-lexicon-classifier <records.tsv> <data-directory> <output-directory>".to_owned()
}
