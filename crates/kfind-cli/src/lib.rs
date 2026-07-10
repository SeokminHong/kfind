//! Command-line contract for the `kfind` binary.

mod args;
mod assets;
mod output;
mod run;

pub use args::{
    Args, BoundaryArg, ColorArg, EncodingArg, ExpandArg, NormalizationArg, PosArg, SortArg,
};
pub use assets::{AssetGenerationError, DistributionAssets, generate_distribution_assets};
pub use output::{
    FilenameMode, OutputError, OutputMode, OutputOptions, OutputWriter, ResolvedColor,
};
pub use run::{CliError, ExitStatus, run_with_io};
