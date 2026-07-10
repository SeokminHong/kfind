use std::fmt;
use std::ops::{BitOr, BitOrAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CoarsePos {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FinePos {
    CommonNoun,
    ProperNoun,
    DependentNoun,
    Pronoun,
    Numeral,
    Verb,
    Adjective,
    AuxiliaryVerb,
    AuxiliaryAdjective,
    Copula,
    Determiner,
    GeneralAdverb,
    ConjunctiveAdverb,
    Particle,
    Interjection,
    Foreign,
    Number,
    Code,
    Literal,
}

impl FinePos {
    #[must_use]
    pub const fn coarse(self) -> CoarsePos {
        match self {
            Self::CommonNoun | Self::ProperNoun | Self::DependentNoun => CoarsePos::Noun,
            Self::Pronoun => CoarsePos::Pronoun,
            Self::Numeral | Self::Number => CoarsePos::Numeral,
            Self::Verb | Self::AuxiliaryVerb => CoarsePos::Verb,
            Self::Adjective | Self::AuxiliaryAdjective | Self::Copula => CoarsePos::Adjective,
            Self::Determiner => CoarsePos::Determiner,
            Self::GeneralAdverb | Self::ConjunctiveAdverb => CoarsePos::Adverb,
            Self::Particle => CoarsePos::Particle,
            Self::Interjection => CoarsePos::Interjection,
            Self::Foreign | Self::Code | Self::Literal => CoarsePos::Literal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PredicatePos {
    Verb,
    Adjective,
    AuxiliaryVerb,
    AuxiliaryAdjective,
    Copula,
}

impl PredicatePos {
    #[must_use]
    pub const fn coarse(self) -> CoarsePos {
        match self {
            Self::Verb | Self::AuxiliaryVerb => CoarsePos::Verb,
            Self::Adjective | Self::AuxiliaryAdjective | Self::Copula => CoarsePos::Adjective,
        }
    }

    #[must_use]
    pub const fn fine(self) -> FinePos {
        match self {
            Self::Verb => FinePos::Verb,
            Self::Adjective => FinePos::Adjective,
            Self::AuxiliaryVerb => FinePos::AuxiliaryVerb,
            Self::AuxiliaryAdjective => FinePos::AuxiliaryAdjective,
            Self::Copula => FinePos::Copula,
        }
    }

    #[must_use]
    pub const fn is_action(self) -> bool {
        matches!(self, Self::Verb | Self::AuxiliaryVerb)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LexicalAlternation {
    Regular,
    DToL,
    DropS,
    BToWa,
    BToWo,
    DropH,
    ReuDoubleL,
    Reo,
    Ha,
    UToEo,
    Copula,
    Suppletive,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PredicateFlags(u16);

impl PredicateFlags {
    pub const NONE: Self = Self(0);
    pub const EU_DROP: Self = Self(1 << 0);
    pub const RIEUL_DROP: Self = Self(1 << 1);
    pub const ALLOW_UNCONTRACTED: Self = Self(1 << 2);

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for PredicateFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PredicateFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(Box<str>);

impl RuleId {
    #[must_use]
    pub fn new(value: impl Into<Box<str>>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RuleId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RuleId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContinuationState {
    Terminal,
    AOrEo,
    Past,
    Future,
    Eu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceOverride {
    pub surface: Box<str>,
    pub core_len: usize,
    pub continuation: ContinuationState,
    pub rule_id: RuleId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateEntry {
    pub lemma: Box<str>,
    pub pos: PredicatePos,
    pub alternation: LexicalAlternation,
    pub flags: PredicateFlags,
    pub overrides: Box<[SurfaceOverride]>,
}

impl PredicateEntry {
    #[must_use]
    pub fn new(
        lemma: impl Into<Box<str>>,
        pos: PredicatePos,
        alternation: LexicalAlternation,
    ) -> Self {
        Self {
            lemma: lemma.into(),
            pos,
            alternation,
            flags: PredicateFlags::NONE,
            overrides: Box::new([]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceBranchSpec {
    pub anchor: Box<str>,
    pub core_len: usize,
    pub continuation: ContinuationState,
    pub rule_path: Vec<RuleId>,
    pub pos: PredicatePos,
    pub alternation: LexicalAlternation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndingCategory {
    Prefinal,
    Final,
    Connective,
    Adverbial,
    Adnominal,
    Nominalizer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndingInitial {
    Consonant,
    AOrEo,
    Eu,
    AttachNieun,
    AttachRieul,
    AttachBieup,
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MorphFeatureMask(u16);

impl MorphFeatureMask {
    pub const NONE: Self = Self(0);
    pub const ACTION: Self = Self(1 << 0);
    pub const DESCRIPTIVE: Self = Self(1 << 1);
    pub const COPULA: Self = Self(1 << 2);
    pub const VOWEL_FINAL: Self = Self(1 << 3);
    pub const CONSONANT_FINAL: Self = Self(1 << 4);
    pub const RIEUL_FINAL: Self = Self(1 << 5);
    pub const LIGHT_VOWEL: Self = Self(1 << 6);
    pub const DARK_VOWEL: Self = Self(1 << 7);
    pub const SPECIAL_HA: Self = Self(1 << 8);
    pub const SPECIAL_I: Self = Self(1 << 9);
    pub const SPECIAL_ANI: Self = Self(1 << 10);
    pub const SPECIAL_O: Self = Self(1 << 11);
    pub const SPECIAL_ITDA: Self = Self(1 << 12);

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for MorphFeatureMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndingSpec {
    pub id: RuleId,
    pub category: EndingCategory,
    pub initial: EndingInitial,
    pub surface: Box<str>,
    pub required: MorphFeatureMask,
    pub forbidden: MorphFeatureMask,
    pub continuation: ContinuationState,
    pub terminal: bool,
}
