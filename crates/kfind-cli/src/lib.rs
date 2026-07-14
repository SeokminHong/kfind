//! Command-line contract for the `kfind` binary.

mod args;
mod assets;
mod diagnostic;
mod init;
mod locale;
mod output;
mod parse;
mod run;

pub use args::{
    AgentArg, Args, BoundaryArg, ColorArg, EncodingArg, ExpandArg, NormalizationArg, PosArg,
    SortArg,
};
pub use assets::{AssetGenerationError, DistributionAssets, generate_distribution_assets};
pub use diagnostic::{LocalizedCliError, write_cli_error};
pub use init::{InitError, run_init_with_io};
pub use locale::Language;
pub use output::{
    FilenameMode, OutputError, OutputMode, OutputOptions, OutputWriter, ResolvedColor,
};
pub use parse::{CliParseError, parse_args_from};
pub use run::{CliError, ExitStatus, run_with_io};
