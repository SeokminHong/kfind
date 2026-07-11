//! Command-line contract for the `kfind` binary.

mod args;
mod assets;
mod locale;
mod output;
mod parse;
mod run;

pub use args::{
    Args, BoundaryArg, ColorArg, EncodingArg, ExpandArg, NormalizationArg, PosArg, SortArg,
};
pub use assets::{AssetGenerationError, DistributionAssets, generate_distribution_assets};
pub use locale::Language;
pub use output::{
    FilenameMode, OutputError, OutputMode, OutputOptions, OutputWriter, ResolvedColor,
};
pub use parse::{CliParseError, parse_args_from};
pub use run::{CliError, ExitStatus, run_with_io};
