use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use kfind_data::PredicateRecord;
use kfind_morph::{LexicalAlternation, PredicateFlags};

pub const REQUIRED_SOURCES: [&str; 2] = ["krdict", "stdict"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Classification {
    DToL,
    RegularD,
    DropS,
    RegularS,
    BToWa,
    BToWo,
    RegularB,
    DropH,
    RegularH,
    ReuDoubleL,
    Reo,
    RegularEuDrop,
    UToEo,
}

impl Classification {
    pub const VALUES: [Self; 13] = [
        Self::DToL,
        Self::RegularD,
        Self::DropS,
        Self::RegularS,
        Self::BToWa,
        Self::BToWo,
        Self::RegularB,
        Self::DropH,
        Self::RegularH,
        Self::ReuDoubleL,
        Self::Reo,
        Self::RegularEuDrop,
        Self::UToEo,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::DToL => "DToL",
            Self::RegularD => "RegularD",
            Self::DropS => "DropS",
            Self::RegularS => "RegularS",
            Self::BToWa => "BToWa",
            Self::BToWo => "BToWo",
            Self::RegularB => "RegularB",
            Self::DropH => "DropH",
            Self::RegularH => "RegularH",
            Self::ReuDoubleL => "ReuDoubleL",
            Self::Reo => "Reo",
            Self::RegularEuDrop => "RegularEuDrop",
            Self::UToEo => "UToEo",
        }
    }

    pub const fn alternation(self) -> &'static str {
        match self {
            Self::RegularD
            | Self::RegularS
            | Self::RegularB
            | Self::RegularH
            | Self::RegularEuDrop => "Regular",
            Self::DToL => "DToL",
            Self::DropS => "DropS",
            Self::BToWa => "BToWa",
            Self::BToWo => "BToWo",
            Self::DropH => "DropH",
            Self::ReuDoubleL => "ReuDoubleL",
            Self::Reo => "Reo",
            Self::UToEo => "UToEo",
        }
    }

    pub const fn flags(self) -> &'static str {
        match self {
            Self::RegularEuDrop => "EU_DROP",
            _ => "",
        }
    }

    pub const fn diagnostic_rule_id(self) -> Option<&'static str> {
        match self {
            Self::DToL => Some("lexical.d-to-l"),
            Self::DropS => Some("lexical.drop-s"),
            Self::BToWa => Some("lexical.b-to-wa"),
            Self::BToWo => Some("lexical.b-to-wo"),
            Self::DropH => Some("lexical.drop-h"),
            Self::ReuDoubleL => Some("lexical.reu-double-l"),
            Self::Reo => Some("lexical.reo"),
            Self::RegularEuDrop => Some("contraction.eu-drop"),
            Self::UToEo => Some("lexical.u-to-eo"),
            Self::RegularD | Self::RegularS | Self::RegularB | Self::RegularH => None,
        }
    }

    pub const fn alternation_rule_id(self) -> &'static str {
        match self.lexical_alternation() {
            LexicalAlternation::Regular => "lexical.regular",
            LexicalAlternation::DToL => "lexical.d-to-l",
            LexicalAlternation::DropS => "lexical.drop-s",
            LexicalAlternation::BToWa => "lexical.b-to-wa",
            LexicalAlternation::BToWo => "lexical.b-to-wo",
            LexicalAlternation::DropH => "lexical.drop-h",
            LexicalAlternation::ReuDoubleL => "lexical.reu-double-l",
            LexicalAlternation::Reo => "lexical.reo",
            LexicalAlternation::UToEo => "lexical.u-to-eo",
            LexicalAlternation::Ha
            | LexicalAlternation::Copula
            | LexicalAlternation::Suppletive
            | LexicalAlternation::SurfaceOnly => {
                unreachable!()
            }
        }
    }

    pub const fn lexical_alternation(self) -> LexicalAlternation {
        match self {
            Self::DToL => LexicalAlternation::DToL,
            Self::DropS => LexicalAlternation::DropS,
            Self::BToWa => LexicalAlternation::BToWa,
            Self::BToWo => LexicalAlternation::BToWo,
            Self::DropH => LexicalAlternation::DropH,
            Self::ReuDoubleL => LexicalAlternation::ReuDoubleL,
            Self::Reo => LexicalAlternation::Reo,
            Self::UToEo => LexicalAlternation::UToEo,
            Self::RegularD
            | Self::RegularS
            | Self::RegularB
            | Self::RegularH
            | Self::RegularEuDrop => LexicalAlternation::Regular,
        }
    }

    pub const fn predicate_flags(self) -> PredicateFlags {
        match self {
            Self::RegularEuDrop => PredicateFlags::EU_DROP,
            _ => PredicateFlags::NONE,
        }
    }

    pub const fn is_enriched(self) -> bool {
        !matches!(
            self,
            Self::RegularD | Self::RegularS | Self::RegularB | Self::RegularH | Self::RegularEuDrop
        )
    }

    pub const fn competitors(self) -> &'static [Self] {
        match self {
            Self::DToL => &[Self::RegularD],
            Self::RegularD => &[Self::DToL],
            Self::DropS => &[Self::RegularS],
            Self::RegularS => &[Self::DropS],
            Self::BToWa => &[Self::BToWo, Self::RegularB],
            Self::BToWo => &[Self::BToWa, Self::RegularB],
            Self::RegularB => &[Self::BToWa, Self::BToWo],
            Self::DropH => &[Self::RegularH],
            Self::RegularH => &[Self::DropH],
            Self::ReuDoubleL => &[Self::Reo, Self::RegularEuDrop],
            Self::Reo => &[Self::ReuDoubleL, Self::RegularEuDrop],
            Self::RegularEuDrop => &[Self::ReuDoubleL, Self::Reo],
            Self::UToEo => &[],
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceRecord {
    pub source: String,
    pub source_id: String,
    pub lemma: String,
    pub pos: String,
    pub lexical_status: String,
    pub conjugations: BTreeSet<String>,
    pub related_adverbs: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateKey {
    pub lemma: String,
    pub pos: String,
    pub classification: Classification,
}

#[derive(Clone, Debug, Default)]
pub struct Evidence {
    pub source_ids: BTreeMap<String, BTreeSet<String>>,
}

impl Evidence {
    pub fn add(&mut self, source: &str, source_id: &str) {
        self.source_ids
            .entry(source.to_owned())
            .or_default()
            .insert(source_id.to_owned());
    }

    pub fn has_required_sources(&self) -> bool {
        REQUIRED_SOURCES
            .iter()
            .all(|source| self.source_ids.contains_key(*source))
    }

    pub fn ids(&self, source: &str) -> String {
        self.source_ids
            .get(source)
            .map(|values| values.iter().cloned().collect::<Vec<_>>().join("|"))
            .unwrap_or_default()
    }
}

pub type CoreEntries = BTreeSet<(String, String, String, String)>;

pub fn core_entries(records: &[PredicateRecord]) -> CoreEntries {
    records
        .iter()
        .map(|record| {
            (
                record.lemma.clone(),
                record.pos.as_str().to_owned(),
                record.alternation.as_str().to_owned(),
                record.flags.iter().cloned().collect::<Vec<_>>().join("|"),
            )
        })
        .collect()
}

pub fn core_key(candidate: &CandidateKey) -> (String, String, String, String) {
    (
        candidate.lemma.clone(),
        candidate.pos.clone(),
        candidate.classification.alternation().to_owned(),
        candidate.classification.flags().to_owned(),
    )
}

#[derive(Debug)]
pub struct UsageError(String);

impl UsageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl Display for UsageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for UsageError {}

pub fn parse_source_records(input: &str) -> Result<Vec<SourceRecord>, UsageError> {
    let mut lines = input.lines();
    let header = lines.next().unwrap_or_default();
    if header
        != "source\tsource_id\traw_homonym\tlemma\tpos\tlexical_status\tconjugations\trelated_adverbs"
    {
        return Err(UsageError::new("unexpected normalized records header"));
    }
    lines
        .enumerate()
        .map(|(index, line)| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 8 {
                return Err(UsageError::new(format!(
                    "normalized records line {} has {} fields",
                    index + 2,
                    fields.len()
                )));
            }
            Ok(SourceRecord {
                source: fields[0].to_owned(),
                source_id: fields[1].to_owned(),
                lemma: fields[3].to_owned(),
                pos: fields[4].to_owned(),
                lexical_status: fields[5].to_owned(),
                conjugations: fields[6]
                    .split('|')
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect(),
                related_adverbs: fields[7]
                    .split('|')
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect()
}
