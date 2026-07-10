use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::lexicon::{DataAlternation, DataFinePos};
use crate::{DataError, DataErrorKind, SourceLocation};

mod validation;

use validation::validate_rules;

const SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum EndingCategory {
    Final,
    Connective,
    Adnominal,
    Adverbial,
    Prefinal,
    Nominalizer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum EndingInitial {
    Consonant,
    AOrEo,
    Eu,
    AttachNieun,
    AttachRieul,
    AttachBieup,
    AttachMieum,
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EndingRule {
    pub id: String,
    pub category: EndingCategory,
    pub initial: EndingInitial,
    pub forms: Vec<String>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
    #[serde(default)]
    pub next: Vec<String>,
    pub terminal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlternationRule {
    pub id: String,
    pub kind: DataAlternation,
    pub flags: Vec<String>,
    pub ending_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContractionRule {
    pub id: String,
    pub kind: String,
    pub left: String,
    pub right: String,
    pub result: String,
    #[serde(default)]
    pub ending_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationRule {
    pub id: String,
    pub suffix: String,
    pub source_pos: Vec<DataFinePos>,
    pub result_pos: DataFinePos,
    pub alternation_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ParticleSelection {
    Literal,
    FinalPair,
    EuroRo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParticleTransitionRule {
    pub id: String,
    pub forms: Vec<String>,
    pub selection: ParticleSelection,
    #[serde(default)]
    pub next: Vec<String>,
    pub terminal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleSet {
    pub max_continuation_depth: u8,
    pub endings: Vec<EndingRule>,
    pub alternations: Vec<AlternationRule>,
    pub contractions: Vec<ContractionRule>,
    pub derivations: Vec<DerivationRule>,
    pub particles: Vec<ParticleTransitionRule>,
}

impl RuleSet {
    pub fn all_ids(&self) -> impl Iterator<Item = &str> {
        self.endings
            .iter()
            .map(|rule| rule.id.as_str())
            .chain(self.alternations.iter().map(|rule| rule.id.as_str()))
            .chain(self.contractions.iter().map(|rule| rule.id.as_str()))
            .chain(self.derivations.iter().map(|rule| rule.id.as_str()))
            .chain(self.particles.iter().map(|rule| rule.id.as_str()))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RuleSources<'a> {
    pub endings: &'a str,
    pub alternations: &'a str,
    pub contractions: &'a str,
    pub derivations: &'a str,
    pub particles: &'a str,
}

#[derive(Debug, Default)]
pub(super) struct RuleLocations {
    by_id: BTreeMap<(String, String), SourceLocation>,
}

impl RuleLocations {
    fn from_sources(sources: RuleSources<'_>) -> Self {
        let mut locations = Self::default();
        for (source, input) in [
            ("data/rules/endings.toml", sources.endings),
            ("data/rules/alternations.toml", sources.alternations),
            ("data/rules/contractions.toml", sources.contractions),
            ("data/rules/derivations.toml", sources.derivations),
            ("data/rules/particles.toml", sources.particles),
        ] {
            for (index, line) in input.lines().enumerate() {
                let trimmed = line.trim();
                let Some(id) = trimmed
                    .strip_prefix("id = \"")
                    .and_then(|value| value.strip_suffix('"'))
                else {
                    continue;
                };
                locations
                    .by_id
                    .entry((source.to_owned(), id.to_owned()))
                    .or_insert_with(|| SourceLocation::at_line(source, index + 1));
            }
        }
        locations
    }

    pub(super) fn get(&self, source: &str, id: &str) -> SourceLocation {
        self.by_id
            .get(&(source.to_owned(), id.to_owned()))
            .cloned()
            .unwrap_or_else(|| SourceLocation::new(source))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingsFile {
    schema_version: u16,
    max_continuation_depth: u8,
    #[serde(default, rename = "ending")]
    endings: Vec<EndingRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlternationsFile {
    schema_version: u16,
    #[serde(default, rename = "alternation")]
    alternations: Vec<RawAlternationRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAlternationRule {
    id: String,
    kind: String,
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    ending_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractionsFile {
    schema_version: u16,
    #[serde(default, rename = "contraction")]
    contractions: Vec<ContractionRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DerivationsFile {
    schema_version: u16,
    #[serde(default, rename = "derivation")]
    derivations: Vec<RawDerivationRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDerivationRule {
    id: String,
    suffix: String,
    source_pos: Vec<String>,
    result_pos: String,
    alternation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParticlesFile {
    schema_version: u16,
    #[serde(default, rename = "particle")]
    particles: Vec<ParticleTransitionRule>,
}

pub fn parse_rule_set(sources: RuleSources<'_>) -> Result<RuleSet, DataError> {
    let locations = RuleLocations::from_sources(sources);
    let endings_file: EndingsFile = parse_toml("data/rules/endings.toml", sources.endings)?;
    require_schema("data/rules/endings.toml", endings_file.schema_version)?;
    if endings_file.max_continuation_depth == 0 || endings_file.max_continuation_depth > 4 {
        return Err(invalid_value(
            "data/rules/endings.toml",
            "max_continuation_depth",
            endings_file.max_continuation_depth.to_string(),
            "1..=4 범위여야 합니다",
        ));
    }

    let alternations_file: AlternationsFile =
        parse_toml("data/rules/alternations.toml", sources.alternations)?;
    require_schema(
        "data/rules/alternations.toml",
        alternations_file.schema_version,
    )?;
    let alternations = alternations_file
        .alternations
        .into_iter()
        .map(|raw| {
            let kind = DataAlternation::parse(&raw.kind).ok_or_else(|| {
                invalid_value(
                    "data/rules/alternations.toml",
                    "kind",
                    raw.kind.clone(),
                    "알려진 lexical alternation이 아닙니다",
                )
            })?;
            Ok(AlternationRule {
                id: raw.id,
                kind,
                flags: raw.flags,
                ending_ids: raw.ending_ids,
            })
        })
        .collect::<Result<Vec<_>, DataError>>()?;

    let contractions_file: ContractionsFile =
        parse_toml("data/rules/contractions.toml", sources.contractions)?;
    require_schema(
        "data/rules/contractions.toml",
        contractions_file.schema_version,
    )?;

    let derivations_file: DerivationsFile =
        parse_toml("data/rules/derivations.toml", sources.derivations)?;
    require_schema(
        "data/rules/derivations.toml",
        derivations_file.schema_version,
    )?;
    let derivations = derivations_file
        .derivations
        .into_iter()
        .map(|raw| {
            let source_pos = raw
                .source_pos
                .iter()
                .map(|pos| parse_pos("data/rules/derivations.toml", "source_pos", pos))
                .collect::<Result<Vec<_>, _>>()?;
            let result_pos =
                parse_pos("data/rules/derivations.toml", "result_pos", &raw.result_pos)?;
            Ok(DerivationRule {
                id: raw.id,
                suffix: raw.suffix,
                source_pos,
                result_pos,
                alternation_id: raw.alternation_id,
            })
        })
        .collect::<Result<Vec<_>, DataError>>()?;

    let particles_file: ParticlesFile = parse_toml("data/rules/particles.toml", sources.particles)?;
    require_schema("data/rules/particles.toml", particles_file.schema_version)?;

    let rules = RuleSet {
        max_continuation_depth: endings_file.max_continuation_depth,
        endings: endings_file.endings,
        alternations,
        contractions: contractions_file.contractions,
        derivations,
        particles: particles_file.particles,
    };
    validate_rules(&rules, &locations)?;
    Ok(rules)
}

fn require_schema(source: &str, version: u16) -> Result<(), DataError> {
    if version == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(invalid_value(
            source,
            "schema_version",
            version.to_string(),
            "지원 버전은 1입니다",
        ))
    }
}

fn parse_pos(source: &str, field: &str, value: &str) -> Result<DataFinePos, DataError> {
    DataFinePos::parse(value)
        .ok_or_else(|| invalid_value(source, field, value, "지원하는 세부 품사가 아닙니다"))
}

fn parse_toml<T: DeserializeOwned>(source: &str, input: &str) -> Result<T, DataError> {
    toml::from_str(input).map_err(|error| {
        let mut location = SourceLocation::new(source);
        if let Some(span) = error.span() {
            let prefix = &input[..span.start];
            location.line = Some(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1);
            location.column = Some(
                prefix
                    .rsplit_once('\n')
                    .map_or(prefix.len(), |(_, tail)| tail.len())
                    + 1,
            );
        }
        DataError::new(location, DataErrorKind::Toml(error.message().to_owned()))
    })
}

fn invalid_value(source: &str, field: &str, value: impl Into<String>, reason: &str) -> DataError {
    DataError::new(
        SourceLocation::new(source),
        DataErrorKind::InvalidValue {
            field: field.to_owned(),
            value: value.into(),
            reason: reason.to_owned(),
        },
    )
}
