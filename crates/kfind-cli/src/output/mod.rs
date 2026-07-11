use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use kfind_query::QueryPlan;
use kfind_search::{FileSearchResult, SearchLineKind, SearchRecord};

use crate::{Args, ColorArg};

mod explain;
mod json;
mod text;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputMode {
    Standard,
    Count,
    FilesWithMatches,
    JsonLines,
    Quiet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilenameMode {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedColor {
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FullPosStatus {
    Loaded { path: PathBuf },
    Preview { candidate_paths: Box<[PathBuf]> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputOptions {
    pub mode: OutputMode,
    pub filename: FilenameMode,
    pub default_with_filename: bool,
    pub line_number: bool,
    pub column: bool,
    pub explain_match: bool,
    pub color: ResolvedColor,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            mode: OutputMode::Standard,
            filename: FilenameMode::Auto,
            default_with_filename: false,
            line_number: false,
            column: false,
            explain_match: false,
            color: ResolvedColor::Disabled,
        }
    }
}

impl OutputOptions {
    #[must_use]
    pub fn from_args(args: &Args, stdout_is_terminal: bool, multiple_inputs: bool) -> Self {
        let mode = if args.quiet {
            OutputMode::Quiet
        } else if args.files_with_matches {
            OutputMode::FilesWithMatches
        } else if args.count {
            OutputMode::Count
        } else if args.json {
            OutputMode::JsonLines
        } else {
            OutputMode::Standard
        };
        let filename = if args.with_filename {
            FilenameMode::Always
        } else if args.no_filename {
            FilenameMode::Never
        } else {
            FilenameMode::Auto
        };
        let color = match args.color {
            ColorArg::Always => ResolvedColor::Enabled,
            ColorArg::Never => ResolvedColor::Disabled,
            ColorArg::Auto if stdout_is_terminal && mode == OutputMode::Standard => {
                ResolvedColor::Enabled
            }
            ColorArg::Auto => ResolvedColor::Disabled,
        };
        Self {
            mode,
            filename,
            default_with_filename: multiple_inputs,
            line_number: args.line_number,
            column: args.column,
            explain_match: args.explain_match,
            color,
        }
    }

    fn with_filename(self) -> bool {
        match self.filename {
            FilenameMode::Auto => self.default_with_filename,
            FilenameMode::Always => true,
            FilenameMode::Never => false,
        }
    }
}

pub struct OutputWriter<W> {
    writer: W,
    options: OutputOptions,
}

impl<W: Write> OutputWriter<W> {
    #[must_use]
    pub const fn new(writer: W, options: OutputOptions) -> Self {
        Self { writer, options }
    }

    pub fn write_query_plan(&mut self, plan: &QueryPlan) -> Result<(), OutputError> {
        explain::write_query_plan(&mut self.writer, plan, None).map_err(OutputError::Io)
    }

    pub(crate) fn write_query_plan_with_full_pos(
        &mut self,
        plan: &QueryPlan,
        full_pos: &FullPosStatus,
    ) -> Result<(), OutputError> {
        explain::write_query_plan(&mut self.writer, plan, Some(full_pos)).map_err(OutputError::Io)
    }

    pub fn write_file(
        &mut self,
        result: &FileSearchResult,
        plan: &QueryPlan,
    ) -> Result<(), OutputError> {
        match self.options.mode {
            OutputMode::Standard => {
                text::write_standard(&mut self.writer, result, plan, self.options)
                    .map_err(OutputError::Io)
            }
            OutputMode::Count => {
                text::write_count(&mut self.writer, result, self.options.with_filename())
                    .map_err(OutputError::Io)
            }
            OutputMode::FilesWithMatches => {
                text::write_filename_if_matched(&mut self.writer, result).map_err(OutputError::Io)
            }
            OutputMode::JsonLines => json::write_file(&mut self.writer, result, plan, self.options),
            OutputMode::Quiet => Ok(()),
        }
    }

    pub(crate) fn write_record(
        &mut self,
        path: &Path,
        record: &SearchRecord,
        plan: &QueryPlan,
    ) -> Result<(), OutputError> {
        if !matches!(
            self.options.mode,
            OutputMode::Standard | OutputMode::JsonLines
        ) {
            return Ok(());
        }
        let matching_lines = u64::from(matches!(
            record,
            SearchRecord::Line(line) if line.kind == SearchLineKind::Match
        ));
        let matched_spans = match record {
            SearchRecord::Line(line) => Some(line.matches.len() as u64),
            SearchRecord::ContextBreak => Some(0),
        };
        self.write_file(
            &FileSearchResult {
                path: path.to_path_buf(),
                records: vec![record.clone()],
                matching_lines,
                matched_spans,
                binary_byte_offset: None,
            },
            plan,
        )
    }

    pub fn flush(&mut self) -> Result<(), OutputError> {
        self.writer.flush().map_err(OutputError::Io)
    }

    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer
    }
}

#[derive(Debug)]
pub enum OutputError {
    Io(io::Error),
    Json(serde_json::Error),
}

impl OutputError {
    #[must_use]
    pub fn is_broken_pipe(&self) -> bool {
        matches!(self, Self::Io(error) if error.kind() == io::ErrorKind::BrokenPipe)
    }
}

impl Display for OutputError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Json(error) => write!(formatter, "failed to serialize JSON output: {error}"),
        }
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

pub(crate) fn write_safe_path(writer: &mut impl Write, path: &Path) -> io::Result<()> {
    text::write_safe_bytes(writer, &text::path_bytes(path))
}

pub(crate) fn write_safe_text(writer: &mut impl Write, value: &str) -> io::Result<()> {
    text::write_safe_bytes(writer, value.as_bytes())
}
