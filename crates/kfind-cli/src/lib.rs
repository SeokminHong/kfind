//! Command-line contract for the `kfind` binary.

mod args;
mod output;
mod run;

pub use args::{
    Args, BoundaryArg, ColorArg, EncodingArg, ExpandArg, NormalizationArg, PosArg, SortArg,
};
pub use output::{
    FilenameMode, OutputError, OutputMode, OutputOptions, OutputWriter, ResolvedColor,
};
pub use run::{CliError, ExitStatus, run_with_io};
