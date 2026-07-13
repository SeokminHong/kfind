use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};
use kfind_morph::CoarsePos;
use kfind_query::{
    BoundaryPolicy, CompileOptionError, CompileOptionOverrides, CompileOptions, ExpandMode,
    NormalizationMode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PosArg {
    Auto,
    Noun,
    Pronoun,
    Numeral,
    Verb,
    Adjective,
    Determiner,
    Adverb,
    Particle,
    Interjection,
    Literal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ExpandArg {
    Literal,
    Inflection,
    Derivation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BoundaryArg {
    Smart,
    Token,
    Any,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum NormalizationArg {
    Nfc,
    Canonical,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum EncodingArg {
    Auto,
    #[value(name = "utf-8")]
    Utf8,
    #[value(name = "utf-16le")]
    Utf16le,
    #[value(name = "utf-16be")]
    Utf16be,
    EucKr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SortArg {
    Path,
}

/// Fast Korean lemma and inflection search for code and documents.
#[derive(Debug, Parser)]
#[command(
    name = "kfind",
    version,
    disable_help_flag = true,
    disable_version_flag = true,
    args_conflicts_with_subcommands = true
)]
pub struct Args {
    /// Korean lemma, short phrase, or tagged query.
    pub query: String,

    /// Files and directories to search. Defaults to stdin when piped, otherwise '.'.
    #[arg(value_name = "PATH", num_args = 0..)]
    pub paths: Vec<PathBuf>,

    /// Force one part of speech. Defaults to auto inference.
    #[arg(long, value_enum)]
    pub pos: Option<PosArg>,

    /// Choose literal, inflection, or derivation expansion. Defaults to inflection.
    #[arg(long, value_enum)]
    pub expand: Option<ExpandArg>,

    /// Choose smart, token, or unrestricted boundaries. Defaults to smart.
    #[arg(long, value_enum)]
    pub boundary: Option<BoundaryArg>,

    /// Search only the literal query without morphology expansion.
    #[arg(long)]
    pub literal: bool,

    /// Use only the lexicon embedded in the binary.
    #[arg(long)]
    pub embedded: bool,

    /// Maximum Unicode scalar gap between phrase atoms. Defaults to 24.
    #[arg(long)]
    pub max_gap: Option<usize>,

    /// Choose NFC, canonical NFC+NFD, or byte-exact matching. Defaults to NFC.
    #[arg(long, value_enum)]
    pub unicode_normalization: Option<NormalizationArg>,

    #[arg(long, value_enum, default_value_t = EncodingArg::Auto)]
    pub encoding: EncodingArg,

    #[arg(long, value_name = "GLOB", action = ArgAction::Append)]
    pub glob: Vec<String>,

    #[arg(long = "type", value_name = "TYPE", action = ArgAction::Append)]
    pub file_type: Vec<String>,

    #[arg(long, value_name = "NAME:GLOB", action = ArgAction::Append)]
    pub type_add: Vec<String>,

    #[arg(long)]
    pub hidden: bool,

    #[arg(long)]
    pub no_ignore: bool,

    #[arg(long, value_name = "NUM")]
    pub threads: Option<usize>,

    #[arg(short = 'n', long)]
    pub line_number: bool,

    #[arg(short = 'H', long, conflicts_with = "no_filename")]
    pub with_filename: bool,

    #[arg(short = 'h', long, conflicts_with = "with_filename")]
    pub no_filename: bool,

    #[arg(short = 'C', long, value_name = "NUM")]
    pub context: Option<usize>,

    #[arg(short = 'B', long, value_name = "NUM")]
    pub before_context: Option<usize>,

    #[arg(short = 'A', long, value_name = "NUM")]
    pub after_context: Option<usize>,

    #[arg(short = 'l', long, conflicts_with_all = ["count", "quiet", "json"])]
    pub files_with_matches: bool,

    #[arg(short = 'c', long, conflicts_with_all = ["quiet", "json"])]
    pub count: bool,

    #[arg(short = 'q', long, conflicts_with = "json")]
    pub quiet: bool,

    #[arg(long)]
    pub json: bool,

    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    pub color: ColorArg,

    #[arg(long)]
    pub column: bool,

    #[arg(long, conflicts_with = "json")]
    pub explain_query: bool,

    #[arg(long)]
    pub explain_match: bool,

    #[arg(long, value_enum)]
    pub sort: Option<SortArg>,

    #[arg(long, value_name = "PATH")]
    pub data_dir: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    pub user_lexicon: Option<PathBuf>,

    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(short = 'V', long, action = ArgAction::Version)]
    pub version: Option<bool>,
}

impl Args {
    pub fn compile_options(&self) -> Result<CompileOptions, CompileOptionError> {
        CompileOptions::resolve(CompileOptionOverrides {
            expand: self.expand.map(ExpandMode::from),
            boundary: self.boundary.map(BoundaryPolicy::from),
            pos: self.pos.and_then(PosArg::coarse),
            normalization: self.unicode_normalization.map(NormalizationMode::from),
            max_gap: self.max_gap,
            literal: self.literal,
            ..CompileOptionOverrides::default()
        })
    }
}

impl PosArg {
    const fn coarse(self) -> Option<CoarsePos> {
        match self {
            Self::Auto => None,
            Self::Noun => Some(CoarsePos::Noun),
            Self::Pronoun => Some(CoarsePos::Pronoun),
            Self::Numeral => Some(CoarsePos::Numeral),
            Self::Verb => Some(CoarsePos::Verb),
            Self::Adjective => Some(CoarsePos::Adjective),
            Self::Determiner => Some(CoarsePos::Determiner),
            Self::Adverb => Some(CoarsePos::Adverb),
            Self::Particle => Some(CoarsePos::Particle),
            Self::Interjection => Some(CoarsePos::Interjection),
            Self::Literal => Some(CoarsePos::Literal),
        }
    }
}

impl From<ExpandArg> for ExpandMode {
    fn from(value: ExpandArg) -> Self {
        match value {
            ExpandArg::Literal => Self::Literal,
            ExpandArg::Inflection => Self::Inflection,
            ExpandArg::Derivation => Self::Derivation,
        }
    }
}

impl From<BoundaryArg> for BoundaryPolicy {
    fn from(value: BoundaryArg) -> Self {
        match value {
            BoundaryArg::Smart => Self::Smart,
            BoundaryArg::Token => Self::Token,
            BoundaryArg::Any => Self::Any,
        }
    }
}

impl From<NormalizationArg> for NormalizationMode {
    fn from(value: NormalizationArg) -> Self {
        match value {
            NormalizationArg::Nfc => Self::Nfc,
            NormalizationArg::Canonical => Self::Canonical,
            NormalizationArg::None => Self::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let args = Args::try_parse_from(["kfind", "걷다"]).unwrap();

        let options = args.compile_options().unwrap();
        assert_eq!(options.global_pos, None);
        assert_eq!(options.expand, ExpandMode::Inflection);
        assert_eq!(options.boundary, BoundaryPolicy::Smart);
        assert_eq!(options.phrase.max_gap, 24);
        assert!(args.paths.is_empty());
    }

    #[test]
    fn short_h_controls_filenames() {
        let args = Args::try_parse_from(["kfind", "-h", "걷다", "."]).unwrap();

        assert!(args.no_filename);
        assert_eq!(args.paths, [PathBuf::from(".")]);
    }

    #[test]
    fn options_may_follow_search_paths() {
        let args = Args::try_parse_from([
            "kfind",
            "n:권한 v:검증하다",
            "src",
            "docs",
            "--max-gap",
            "12",
        ])
        .unwrap();

        assert_eq!(args.paths, [PathBuf::from("src"), PathBuf::from("docs")]);
        assert_eq!(args.max_gap, Some(12));
    }

    #[test]
    fn encoding_names_match_the_cli_contract() {
        for (value, expected) in [
            ("utf-8", EncodingArg::Utf8),
            ("utf-16le", EncodingArg::Utf16le),
            ("utf-16be", EncodingArg::Utf16be),
            ("euc-kr", EncodingArg::EucKr),
        ] {
            let args = Args::try_parse_from(["kfind", "--encoding", value, "걷다"]).unwrap();
            assert_eq!(args.encoding, expected);
        }
    }

    #[test]
    fn literal_resolves_both_mode_axes() {
        let args = Args::try_parse_from(["kfind", "--literal", "걸어"]).unwrap();

        let options = args.compile_options().unwrap();
        assert_eq!(options.global_pos, Some(CoarsePos::Literal));
        assert_eq!(options.expand, ExpandMode::Literal);
        assert_eq!(options.boundary, BoundaryPolicy::Smart);
    }

    #[test]
    fn literal_rejects_conflicting_pos() {
        let args = Args::try_parse_from(["kfind", "--literal", "--pos", "verb", "걸어"]).unwrap();

        assert!(matches!(
            args.compile_options(),
            Err(CompileOptionError::LiteralPosConflict {
                pos: CoarsePos::Verb
            })
        ));
    }
}
