//! Command-line contract for the `kfind` binary.

mod args;
mod output;

pub use args::{
    Args, BoundaryArg, ColorArg, EncodingArg, ExpandArg, NormalizationArg, PosArg, SortArg,
};
pub use output::{
    FilenameMode, OutputError, OutputMode, OutputOptions, OutputWriter, ResolvedColor,
};
