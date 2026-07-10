use std::collections::BTreeSet;

use crate::tsv::parse_rows;
use crate::validation::{require_nfc, require_rule_id};
use crate::{DataError, DataErrorKind, DataWarning};

mod user;

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
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SurfaceOverride {
    pub rule_id: String,
    pub surface: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateRecord {
    pub lemma: String,
    pub pos: DataFinePos,
    pub alternation: DataAlternation,
    pub flags: BTreeSet<String>,
    pub overrides: Vec<SurfaceOverride>,
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

#[derive(Clone, Copy, Debug)]
pub struct LexiconSources<'a> {
    pub predicates: &'a str,
    pub nominals: &'a str,
    pub modifiers: &'a str,
    pub particles: &'a str,
}

pub fn parse_lexicons(
    sources: LexiconSources<'_>,
) -> Result<(LexiconData, Vec<DataWarning>), DataError> {
    let (predicates, mut warnings) =
        parse_predicates_tsv("data/lexicon/predicates.tsv", sources.predicates)?;
    let (nominals, nominal_warnings) =
        parse_nominals_tsv("data/lexicon/nominals.tsv", sources.nominals)?;
    warnings.extend(nominal_warnings);
    let (modifiers, modifier_warnings) =
        parse_modifiers_tsv("data/lexicon/modifiers.tsv", sources.modifiers)?;
    warnings.extend(modifier_warnings);
    let (particles, particle_warnings) =
        parse_particles_tsv("data/lexicon/particles.tsv", sources.particles)?;
    warnings.extend(particle_warnings);
    Ok((
        LexiconData {
            predicates,
            nominals,
            modifiers,
            particles,
        },
        warnings,
    ))
}

pub fn parse_predicates_tsv(
    source: &str,
    input: &str,
) -> Result<(Vec<PredicateRecord>, Vec<DataWarning>), DataError> {
    let parsed = parse_rows(
        source,
        input,
        &["lemma", "pos", "alternation", "flags", "overrides"],
    )?;
    let mut records = Vec::with_capacity(parsed.rows.len());
    for row in parsed.rows {
        let lemma = parse_nfc(source, row.line, "lemma", row.fields[0])?;
        if lemma.strip_suffix('다').is_none_or(str::is_empty) {
            return Err(DataError::line(
                source,
                row.line,
                DataErrorKind::InvalidPredicateLemma(lemma),
            ));
        }
        let pos = parse_pos(source, row.line, row.fields[1])?;
        if !pos.is_predicate() {
            return Err(invalid_value(
                source,
                row.line,
                "pos",
                row.fields[1],
                "predicate POS가 아닙니다",
            ));
        }
        let alternation = DataAlternation::parse(row.fields[2]).ok_or_else(|| {
            invalid_value(
                source,
                row.line,
                "alternation",
                row.fields[2],
                "알려진 lexical alternation이 아닙니다",
            )
        })?;
        records.push(PredicateRecord {
            lemma,
            pos,
            alternation,
            flags: parse_flags(source, row.line, row.fields[3])?,
            overrides: parse_overrides(source, row.line, row.fields[4])?,
        });
    }
    Ok((records, parsed.warnings))
}

pub fn parse_nominals_tsv(
    source: &str,
    input: &str,
) -> Result<(Vec<NominalRecord>, Vec<DataWarning>), DataError> {
    let parsed = parse_rows(source, input, &["lemma", "pos", "flags", "overrides"])?;
    let mut records = Vec::with_capacity(parsed.rows.len());
    for row in parsed.rows {
        let pos = parse_pos(source, row.line, row.fields[1])?;
        if !pos.is_nominal() {
            return Err(invalid_value(
                source,
                row.line,
                "pos",
                row.fields[1],
                "nominal POS가 아닙니다",
            ));
        }
        records.push(NominalRecord {
            lemma: parse_nfc(source, row.line, "lemma", row.fields[0])?,
            pos,
            flags: parse_flags(source, row.line, row.fields[2])?,
            overrides: parse_overrides(source, row.line, row.fields[3])?,
        });
    }
    Ok((records, parsed.warnings))
}

pub fn parse_modifiers_tsv(
    source: &str,
    input: &str,
) -> Result<(Vec<ModifierRecord>, Vec<DataWarning>), DataError> {
    let parsed = parse_rows(source, input, &["lemma", "pos", "flags"])?;
    let mut records = Vec::with_capacity(parsed.rows.len());
    for row in parsed.rows {
        let pos = parse_pos(source, row.line, row.fields[1])?;
        if !pos.is_modifier() {
            return Err(invalid_value(
                source,
                row.line,
                "pos",
                row.fields[1],
                "modifier POS가 아닙니다",
            ));
        }
        records.push(ModifierRecord {
            lemma: parse_nfc(source, row.line, "lemma", row.fields[0])?,
            pos,
            flags: parse_flags(source, row.line, row.fields[2])?,
        });
    }
    Ok((records, parsed.warnings))
}

pub fn parse_particles_tsv(
    source: &str,
    input: &str,
) -> Result<(Vec<ParticleRecord>, Vec<DataWarning>), DataError> {
    let parsed = parse_rows(
        source,
        input,
        &["lemma", "pos", "rule_id", "variants", "flags"],
    )?;
    let mut records = Vec::with_capacity(parsed.rows.len());
    for row in parsed.rows {
        let pos = parse_pos(source, row.line, row.fields[1])?;
        if !pos.is_particle() {
            return Err(invalid_value(
                source,
                row.line,
                "pos",
                row.fields[1],
                "particle POS가 아닙니다",
            ));
        }
        require_rule_id(source, row.fields[2])?;
        let variants = parse_nfc_list(source, row.line, "variants", row.fields[3], '|')?;
        if variants.is_empty() {
            return Err(invalid_value(
                source,
                row.line,
                "variants",
                row.fields[3],
                "하나 이상의 표면형이 필요합니다",
            ));
        }
        records.push(ParticleRecord {
            lemma: parse_nfc(source, row.line, "lemma", row.fields[0])?,
            pos,
            rule_id: row.fields[2].to_owned(),
            variants,
            flags: parse_flags(source, row.line, row.fields[4])?,
        });
    }
    Ok((records, parsed.warnings))
}

fn parse_pos(source: &str, line: usize, value: &str) -> Result<DataFinePos, DataError> {
    DataFinePos::parse(value)
        .ok_or_else(|| invalid_value(source, line, "pos", value, "지원하는 세부 품사가 아닙니다"))
}

fn parse_flags(source: &str, line: usize, value: &str) -> Result<BTreeSet<String>, DataError> {
    let flags = value
        .split('|')
        .filter(|flag| !flag.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for flag in &flags {
        if !flag
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(invalid_value(
                source,
                line,
                "flags",
                flag,
                "대문자 ASCII identifier여야 합니다",
            ));
        }
    }
    Ok(flags)
}

fn parse_overrides(
    source: &str,
    line: usize,
    value: &str,
) -> Result<Vec<SurfaceOverride>, DataError> {
    let mut records = Vec::new();
    let mut seen = std::collections::BTreeMap::<&str, &str>::new();
    for item in value.split(';').filter(|item| !item.is_empty()) {
        let Some((rule_id, surface)) = item.split_once('=') else {
            return Err(invalid_value(
                source,
                line,
                "overrides",
                item,
                "`rule.id=surface` 형식이어야 합니다",
            ));
        };
        require_rule_id(source, rule_id)?;
        require_nfc(source, Some(line), "override surface", surface)?;
        if surface.is_empty() {
            return Err(invalid_value(
                source,
                line,
                "overrides",
                item,
                "surface가 비어 있습니다",
            ));
        }
        if let Some(first) = seen.insert(rule_id, surface) {
            if first != surface {
                return Err(DataError::line(
                    source,
                    line,
                    DataErrorKind::OverrideConflict {
                        lemma: "현재 행".to_owned(),
                        rule_id: rule_id.to_owned(),
                        first: first.to_owned(),
                        second: surface.to_owned(),
                    },
                ));
            }
            continue;
        }
        records.push(SurfaceOverride {
            rule_id: rule_id.to_owned(),
            surface: surface.to_owned(),
        });
    }
    Ok(records)
}

fn parse_nfc_list(
    source: &str,
    line: usize,
    field: &str,
    value: &str,
    separator: char,
) -> Result<Vec<String>, DataError> {
    value
        .split(separator)
        .filter(|entry| !entry.is_empty())
        .map(|entry| parse_nfc(source, line, field, entry))
        .collect()
}

fn parse_nfc(source: &str, line: usize, field: &str, value: &str) -> Result<String, DataError> {
    require_nfc(source, Some(line), field, value)?;
    if value.is_empty() {
        return Err(invalid_value(source, line, field, value, "비어 있습니다"));
    }
    Ok(value.to_owned())
}

fn invalid_value(source: &str, line: usize, field: &str, value: &str, reason: &str) -> DataError {
    DataError::line(
        source,
        line,
        DataErrorKind::InvalidValue {
            field: field.to_owned(),
            value: value.to_owned(),
            reason: reason.to_owned(),
        },
    )
}
