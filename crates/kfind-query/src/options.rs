use std::error::Error;
use std::fmt;

use kfind_morph::CoarsePos;

use crate::ast::PhrasePolicy;

pub const DEFAULT_MAX_QUERY_SCALARS: usize = 256;
pub const DEFAULT_MAX_ATOMS: usize = 32;
pub const DEFAULT_MAX_ANALYSES_PER_ATOM: usize = 32;
pub const DEFAULT_MAX_PROGRAMS: usize = 4_096;
pub const DEFAULT_MAX_MATCHER_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_MAX_CONTINUATION_DEPTH: usize = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExpandMode {
    Literal,
    #[default]
    Inflection,
    Derivation,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BoundaryPolicy {
    #[default]
    Smart,
    Token,
    Any,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NormalizationMode {
    #[default]
    Nfc,
    Canonical,
    None,
}

/// Hard query-plan limits. Exceeding a limit is always an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanLimits {
    pub max_query_scalars: usize,
    pub max_atoms: usize,
    pub max_analyses_per_atom: usize,
    pub max_programs: usize,
    pub max_matcher_bytes: usize,
    pub max_continuation_depth: usize,
}

impl Default for PlanLimits {
    fn default() -> Self {
        Self {
            max_query_scalars: DEFAULT_MAX_QUERY_SCALARS,
            max_atoms: DEFAULT_MAX_ATOMS,
            max_analyses_per_atom: DEFAULT_MAX_ANALYSES_PER_ATOM,
            max_programs: DEFAULT_MAX_PROGRAMS,
            max_matcher_bytes: DEFAULT_MAX_MATCHER_BYTES,
            max_continuation_depth: DEFAULT_MAX_CONTINUATION_DEPTH,
        }
    }
}

/// Options used while parsing and compiling a query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileOptions {
    pub expand: ExpandMode,
    pub boundary: BoundaryPolicy,
    pub global_pos: Option<CoarsePos>,
    pub normalization: NormalizationMode,
    pub phrase: PhrasePolicy,
    pub limits: PlanLimits,
}

impl CompileOptions {
    /// Applies CLI-style overrides, including the `--literal` shortcut.
    pub fn resolve(overrides: CompileOptionOverrides) -> Result<Self, CompileOptionError> {
        if overrides.literal {
            if let Some(expand) = overrides
                .expand
                .filter(|expand| *expand != ExpandMode::Literal)
            {
                return Err(CompileOptionError::LiteralExpandConflict { expand });
            }
            if let Some(pos) = overrides.pos.filter(|pos| *pos != CoarsePos::Literal) {
                return Err(CompileOptionError::LiteralPosConflict { pos });
            }
        }

        Ok(Self {
            expand: if overrides.literal {
                ExpandMode::Literal
            } else {
                overrides.expand.unwrap_or_default()
            },
            boundary: overrides.boundary.unwrap_or_default(),
            global_pos: if overrides.literal {
                Some(CoarsePos::Literal)
            } else {
                overrides.pos
            },
            normalization: overrides.normalization.unwrap_or_default(),
            phrase: PhrasePolicy {
                max_gap: overrides.max_gap.unwrap_or(crate::ast::DEFAULT_MAX_GAP),
            },
            limits: overrides.limits.unwrap_or_default(),
        })
    }

    #[must_use]
    pub const fn requires_full_pos_lexicon(&self) -> bool {
        !matches!(self.global_pos, Some(CoarsePos::Literal))
    }
}

/// Values that may be explicitly supplied by the CLI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileOptionOverrides {
    pub expand: Option<ExpandMode>,
    pub boundary: Option<BoundaryPolicy>,
    pub pos: Option<CoarsePos>,
    pub normalization: Option<NormalizationMode>,
    pub max_gap: Option<usize>,
    pub limits: Option<PlanLimits>,
    pub literal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileOptionError {
    LiteralExpandConflict { expand: ExpandMode },
    LiteralPosConflict { pos: CoarsePos },
}

impl fmt::Display for CompileOptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiteralExpandConflict { expand } => {
                write!(formatter, "--literal conflicts with --expand {expand:?}")
            }
            Self::LiteralPosConflict { pos } => {
                write!(formatter, "--literal conflicts with --pos {pos:?}")
            }
        }
    }
}

impl Error for CompileOptionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_specification() {
        let options = CompileOptions::default();

        assert_eq!(options.expand, ExpandMode::Inflection);
        assert_eq!(options.boundary, BoundaryPolicy::Smart);
        assert_eq!(options.global_pos, None);
        assert_eq!(options.normalization, NormalizationMode::Nfc);
        assert_eq!(options.phrase.max_gap, 24);
        assert_eq!(options.limits, PlanLimits::default());
        assert!(options.requires_full_pos_lexicon());
    }

    #[test]
    fn literal_shortcut_forces_literal_expansion_and_pos() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            literal: true,
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        assert_eq!(options.expand, ExpandMode::Literal);
        assert_eq!(options.global_pos, Some(CoarsePos::Literal));
        assert!(!options.requires_full_pos_lexicon());
    }

    #[test]
    fn literal_shortcut_rejects_conflicting_expansion() {
        let error = CompileOptions::resolve(CompileOptionOverrides {
            literal: true,
            expand: Some(ExpandMode::Derivation),
            ..CompileOptionOverrides::default()
        })
        .unwrap_err();

        assert_eq!(
            error,
            CompileOptionError::LiteralExpandConflict {
                expand: ExpandMode::Derivation
            }
        );
    }

    #[test]
    fn explicit_literal_pos_does_not_require_full_pos_lexicon() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos: Some(CoarsePos::Literal),
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        assert!(!options.requires_full_pos_lexicon());
    }

    #[test]
    fn literal_shortcut_rejects_conflicting_pos() {
        let error = CompileOptions::resolve(CompileOptionOverrides {
            literal: true,
            pos: Some(CoarsePos::Noun),
            ..CompileOptionOverrides::default()
        })
        .unwrap_err();

        assert_eq!(
            error,
            CompileOptionError::LiteralPosConflict {
                pos: CoarsePos::Noun
            }
        );
    }

    #[test]
    fn literal_shortcut_accepts_equivalent_explicit_values() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            literal: true,
            expand: Some(ExpandMode::Literal),
            pos: Some(CoarsePos::Literal),
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        assert_eq!(options.expand, ExpandMode::Literal);
        assert_eq!(options.global_pos, Some(CoarsePos::Literal));
    }
}
