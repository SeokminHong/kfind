mod artifact;
mod component_artifact;
mod component_benchmark;
mod component_payload;
mod dataset;
mod index;
mod measure;
mod storage;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use artifact::{IndexKind, build_container, parse_digest, validate_container};
use clap::{Parser, Subcommand, ValueEnum};
use component_benchmark::{
    ComponentBuildInput, ComponentFormat, build_component_resources, probe_component_resource,
};
use dataset::Dataset;
use measure::probe;
use storage::StorageMode;

#[derive(Debug, Parser)]
#[command(about = "Build and compare morphology prefix indexes")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build {
        #[arg(long)]
        source_sha256: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(required = true)]
        csv: Vec<PathBuf>,
    },
    Validate {
        #[arg(long)]
        source_sha256: String,
        artifact: PathBuf,
    },
    Probe {
        #[arg(long)]
        source_sha256: String,
        #[arg(long, value_enum)]
        storage: StorageArg,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long)]
        queries: PathBuf,
        artifact: PathBuf,
    },
    BuildComponents {
        #[arg(long)]
        source_sha256: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        matrix: PathBuf,
        #[arg(long)]
        char_def: PathBuf,
        #[arg(long)]
        unk_def: PathBuf,
        #[arg(required = true)]
        csv: Vec<PathBuf>,
    },
    ProbeComponent {
        #[arg(long)]
        source_sha256: String,
        #[arg(long, value_enum)]
        format: ComponentFormat,
        #[arg(long, value_enum)]
        storage: StorageArg,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long)]
        queries: PathBuf,
        artifact: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum StorageArg {
    Resident,
    Mmap,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Build {
            source_sha256,
            output,
            csv,
        } => build(&source_sha256, &output, &csv),
        Command::Validate {
            source_sha256,
            artifact,
        } => {
            let expected = parse_digest(&source_sha256)?;
            let bytes = fs::read(&artifact)
                .with_context(|| format!("failed to read {}", artifact.display()))?;
            let view = validate_container(&bytes, &expected)?;
            println!("{}", serde_json::to_string_pretty(&view.summary())?);
            Ok(())
        }
        Command::Probe {
            source_sha256,
            storage,
            iterations,
            queries,
            artifact,
        } => {
            let storage = match storage {
                StorageArg::Resident => StorageMode::Resident,
                StorageArg::Mmap => StorageMode::Mmap,
            };
            let report = probe(
                &artifact,
                &queries,
                &parse_digest(&source_sha256)?,
                storage,
                iterations,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Command::BuildComponents {
            source_sha256,
            output,
            matrix,
            char_def,
            unk_def,
            csv,
        } => build_component_resources(ComponentBuildInput {
            source_sha256: &source_sha256,
            source_digest: parse_digest(&source_sha256)?,
            output: &output,
            matrix: &matrix,
            char_def: &char_def,
            unk_def: &unk_def,
            csv: &csv,
        }),
        Command::ProbeComponent {
            source_sha256,
            format,
            storage,
            iterations,
            queries,
            artifact,
        } => {
            let storage = match storage {
                StorageArg::Resident => StorageMode::Resident,
                StorageArg::Mmap => StorageMode::Mmap,
            };
            let report = probe_component_resource(
                format,
                &artifact,
                &queries,
                &parse_digest(&source_sha256)?,
                storage,
                iterations,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
    }
}

fn build(source_sha256: &str, output: &PathBuf, csv: &[PathBuf]) -> Result<()> {
    let source_digest = parse_digest(source_sha256)?;
    let dataset = Dataset::load(csv)?;
    fs::create_dir_all(output).with_context(|| format!("failed to create {}", output.display()))?;

    for kind in [IndexKind::DoubleArray, IndexKind::Fst] {
        let index = index::build(kind, dataset.keys())?;
        let container = build_container(
            kind,
            source_digest,
            dataset.surface_count(),
            dataset.analysis_count(),
            dataset.pos_counts(),
            &index,
            dataset.payload(),
        )?;
        let path = output.join(kind.artifact_name());
        fs::write(&path, container)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    fs::write(
        output.join("queries.json"),
        serde_json::to_vec_pretty(&dataset.workload())?,
    )?;
    fs::write(
        output.join("build-report.json"),
        serde_json::to_vec_pretty(&dataset.report(source_sha256))?,
    )?;
    Ok(())
}
