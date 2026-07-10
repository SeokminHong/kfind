use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use kfind_cli::generate_distribution_assets;

#[derive(Debug, Parser)]
#[command(about = "Generate kfind man page and shell completions")]
struct GenerateAssetsArgs {
    #[arg(value_name = "OUTPUT_DIR")]
    output_directory: PathBuf,
}

fn main() -> ExitCode {
    let args = GenerateAssetsArgs::parse();
    match generate_distribution_assets(&args.output_directory) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("kfind-generate-assets: {error}");
            ExitCode::FAILURE
        }
    }
}
