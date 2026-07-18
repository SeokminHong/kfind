use std::collections::BTreeSet;

mod parser;
mod user;

pub use parser::{
    LexiconSources, parse_lexicons, parse_modifiers_tsv, parse_nominals_tsv, parse_particles_tsv,
    parse_predicates_tsv,
};
pub use user::{UserLexicon, UserNominalRecord, UserPredicateRecord, parse_user_lexicon_toml};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum DataFinePos {
    Nng,
    Nnp,
    Nnb,
    Nr,
    Np,
    Vv,
    Va,
    Vx,
    Vcp,
    Vcn,
    Mm,
    Mag,
    Maj,
    Ic,
    Jks,
    Jkc,
    Jkg,
    Jko,
    Jkb,
    Jkv,
    Jkq,
    Jx,
    Jc,
}

impl DataFinePos {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "NNG" => Self::Nng,
            "NNP" => Self::Nnp,
            "NNB" => Self::Nnb,
            "NR" => Self::Nr,
            "NP" => Self::Np,
            "VV" => Self::Vv,
            "VA" => Self::Va,
            "VX" => Self::Vx,
            "VCP" => Self::Vcp,
            "VCN" => Self::Vcn,
            "MM" => Self::Mm,
            "MAG" => Self::Mag,
            "MAJ" => Self::Maj,
            "IC" => Self::Ic,
            "JKS" => Self::Jks,
            "JKC" => Self::Jkc,
            "JKG" => Self::Jkg,
            "JKO" => Self::Jko,
            "JKB" => Self::Jkb,
            "JKV" => Self::Jkv,
            "JKQ" => Self::Jkq,
            "JX" => Self::Jx,
            "JC" => Self::Jc,
            _ => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nng => "NNG",
            Self::Nnp => "NNP",
            Self::Nnb => "NNB",
            Self::Nr => "NR",
            Self::Np => "NP",
            Self::Vv => "VV",
            Self::Va => "VA",
            Self::Vx => "VX",
            Self::Vcp => "VCP",
            Self::Vcn => "VCN",
            Self::Mm => "MM",
            Self::Mag => "MAG",
            Self::Maj => "MAJ",
            Self::Ic => "IC",
            Self::Jks => "JKS",
            Self::Jkc => "JKC",
            Self::Jkg => "JKG",
            Self::Jko => "JKO",
            Self::Jkb => "JKB",
            Self::Jkv => "JKV",
            Self::Jkq => "JKQ",
            Self::Jx => "JX",
            Self::Jc => "JC",
        }
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Self> {
        const VALUES: &[DataFinePos] = &[
            DataFinePos::Nng,
            DataFinePos::Nnp,
            DataFinePos::Nnb,
            DataFinePos::Nr,
            DataFinePos::Np,
            DataFinePos::Vv,
            DataFinePos::Va,
            DataFinePos::Vx,
            DataFinePos::Vcp,
            DataFinePos::Vcn,
            DataFinePos::Mm,
            DataFinePos::Mag,
            DataFinePos::Maj,
            DataFinePos::Ic,
            DataFinePos::Jks,
            DataFinePos::Jkc,
            DataFinePos::Jkg,
            DataFinePos::Jko,
            DataFinePos::Jkb,
            DataFinePos::Jkv,
            DataFinePos::Jkq,
            DataFinePos::Jx,
            DataFinePos::Jc,
        ];
        VALUES.get(usize::from(code)).copied()
    }

    pub const fn is_predicate(self) -> bool {
        matches!(self, Self::Vv | Self::Va | Self::Vx | Self::Vcp | Self::Vcn)
    }

    pub const fn is_nominal(self) -> bool {
        matches!(
            self,
            Self::Nng | Self::Nnp | Self::Nnb | Self::Nr | Self::Np
        )
    }

    pub const fn is_modifier(self) -> bool {
        matches!(self, Self::Mm | Self::Mag | Self::Maj | Self::Ic)
    }

    pub const fn is_particle(self) -> bool {
        matches!(
            self,
            Self::Jks
                | Self::Jkc
                | Self::Jkg
                | Self::Jko
                | Self::Jkb
                | Self::Jkv
                | Self::Jkq
                | Self::Jx
                | Self::Jc
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DataAlternation {
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
    SurfaceOnly,
}

pub const DICTIONARY_CONJUGATION_RULE_ID: &str = "lexical.dictionary-conjugation";
pub const DICTIONARY_ADVERBIAL_I_RULE_ID: &str = "lexical.dictionary-adverbial-i";
pub const DICTIONARY_RELATED_ADVERB_RULE_ID: &str = "lexical.dictionary-related-adverb";
pub const DICTIONARY_VOICE_DERIVATION_RULE_ID: &str = "lexical.dictionary-voice";

pub fn is_dictionary_surface_rule(id: &str) -> bool {
    matches!(
        id,
        DICTIONARY_CONJUGATION_RULE_ID
            | DICTIONARY_ADVERBIAL_I_RULE_ID
            | DICTIONARY_RELATED_ADVERB_RULE_ID
    )
}

impl DataAlternation {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Regular" => Self::Regular,
            "DToL" => Self::DToL,
            "DropS" => Self::DropS,
            "BToWa" => Self::BToWa,
            "BToWo" => Self::BToWo,
            "DropH" => Self::DropH,
            "ReuDoubleL" => Self::ReuDoubleL,
            "Reo" => Self::Reo,
            "Ha" => Self::Ha,
            "UToEo" => Self::UToEo,
            "Copula" => Self::Copula,
            "Suppletive" => Self::Suppletive,
            "SurfaceOnly" => Self::SurfaceOnly,
            _ => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "Regular",
            Self::DToL => "DToL",
            Self::DropS => "DropS",
            Self::BToWa => "BToWa",
            Self::BToWo => "BToWo",
            Self::DropH => "DropH",
            Self::ReuDoubleL => "ReuDoubleL",
            Self::Reo => "Reo",
            Self::Ha => "Ha",
            Self::UToEo => "UToEo",
            Self::Copula => "Copula",
            Self::Suppletive => "Suppletive",
            Self::SurfaceOnly => "SurfaceOnly",
        }
    }

    pub const fn rule_id(self) -> &'static str {
        match self {
            Self::Regular => "lexical.regular",
            Self::DToL => "lexical.d-to-l",
            Self::DropS => "lexical.drop-s",
            Self::BToWa => "lexical.b-to-wa",
            Self::BToWo => "lexical.b-to-wo",
            Self::DropH => "lexical.drop-h",
            Self::ReuDoubleL => "lexical.reu-double-l",
            Self::Reo => "lexical.reo",
            Self::Ha => "lexical.ha",
            Self::UToEo => "lexical.u-to-eo",
            Self::Copula => "lexical.copula",
            Self::Suppletive => "lexical.suppletive",
            Self::SurfaceOnly => "lexical.surface-only",
        }
    }

    pub const fn fixture_feature(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::DToL => "d-irregular",
            Self::DropS => "s-irregular",
            Self::BToWa => "b-irregular-wa",
            Self::BToWo => "b-irregular-wo",
            Self::DropH => "h-irregular",
            Self::ReuDoubleL => "reu-irregular",
            Self::Reo => "reo-irregular",
            Self::Ha => "ha",
            Self::UToEo => "u-irregular",
            Self::Copula => "copula",
            Self::Suppletive => "suppletive",
            Self::SurfaceOnly => "surface-only",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SurfaceOverride {
    pub rule_id: String,
    pub surface: String,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PredicateDerivation {
    pub rule_id: String,
    pub target_lemma: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateRecord {
    pub lemma: String,
    pub pos: DataFinePos,
    pub alternation: DataAlternation,
    pub flags: BTreeSet<String>,
    pub overrides: Vec<SurfaceOverride>,
    pub derivations: Vec<PredicateDerivation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NominalRecord {
    pub lemma: String,
    pub pos: DataFinePos,
    pub flags: BTreeSet<String>,
    pub overrides: Vec<SurfaceOverride>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierRecord {
    pub lemma: String,
    pub pos: DataFinePos,
    pub flags: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticleRecord {
    pub lemma: String,
    pub pos: DataFinePos,
    pub rule_id: String,
    pub variants: Vec<String>,
    pub flags: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LexiconData {
    pub predicates: Vec<PredicateRecord>,
    pub nominals: Vec<NominalRecord>,
    pub modifiers: Vec<ModifierRecord>,
    pub particles: Vec<ParticleRecord>,
}

impl LexiconData {
    /// Returns every core predicate analysis for `lemma` without deduplicating
    /// same-POS alternations.
    pub fn predicate_analyses<'a>(
        &'a self,
        lemma: &'a str,
    ) -> impl Iterator<Item = &'a PredicateRecord> + 'a {
        self.predicates
            .iter()
            .filter(move |record| record.lemma == lemma)
    }

    /// Applies user entries while preserving every user analysis for the same
    /// lemma. If any user entry for a lemma has `replace = true`, all built-in
    /// entries for that lemma are removed before the user group is appended.
    pub fn apply_user_lexicon(&mut self, user: UserLexicon) {
        let replaced_predicates = user
            .predicates
            .iter()
            .filter(|record| record.replace)
            .map(|record| record.entry.lemma.as_str())
            .collect::<BTreeSet<_>>();
        let replaced_nominals = user
            .nominals
            .iter()
            .filter(|record| record.replace)
            .map(|record| record.entry.lemma.as_str())
            .collect::<BTreeSet<_>>();

        self.predicates
            .retain(|record| !replaced_predicates.contains(record.lemma.as_str()));
        self.nominals
            .retain(|record| !replaced_nominals.contains(record.lemma.as_str()));
        self.predicates
            .extend(user.predicates.into_iter().map(|record| record.entry));
        self.nominals
            .extend(user.nominals.into_iter().map(|record| record.entry));
    }
}
