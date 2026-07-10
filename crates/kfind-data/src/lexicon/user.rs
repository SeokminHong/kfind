use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::rules::RuleSet;
use crate::validation::{require_nfc, require_rule_id};
use crate::{DataError, DataErrorKind, SourceLocation};

use super::{DataAlternation, DataFinePos, NominalRecord, PredicateRecord, SurfaceOverride};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserLexicon {
    pub predicates: Vec<UserPredicateRecord>,
    pub nominals: Vec<UserNominalRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPredicateRecord {
    pub entry: PredicateRecord,
    pub replace: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserNominalRecord {
    pub entry: NominalRecord,
    pub replace: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUserLexicon {
    #[serde(default, rename = "predicate")]
    predicates: Vec<RawPredicate>,
    #[serde(default, rename = "nominal")]
    nominals: Vec<RawNominal>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPredicate {
    lemma: String,
    pos: String,
    alternation: String,
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default, rename = "override")]
    overrides: Vec<RawOverride>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNominal {
    surface: String,
    #[serde(default = "default_nominal_pos")]
    pos: String,
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default, rename = "override")]
    overrides: Vec<RawOverride>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOverride {
    rule_id: String,
    surface: String,
}

pub fn parse_user_lexicon_toml(
    source: &str,
    input: &str,
    rules: &RuleSet,
) -> Result<UserLexicon, DataError> {
    let raw: RawUserLexicon = toml::from_str(input).map_err(|error| {
        let mut location = SourceLocation::new(source);
        if let Some(span) = error.span() {
            set_span_location(&mut location, input, span.start);
        }
        DataError::new(location, DataErrorKind::Toml(error.message().to_owned()))
    })?;
    let known_rules = rules.all_ids().collect::<BTreeSet<_>>();

    let predicates = raw
        .predicates
        .into_iter()
        .map(|raw| parse_predicate(source, input, raw, rules, &known_rules))
        .collect::<Result<Vec<_>, _>>()?;
    let nominals = raw
        .nominals
        .into_iter()
        .map(|raw| parse_nominal(source, input, raw, &known_rules))
        .collect::<Result<Vec<_>, _>>()?;
    validate_override_conflicts(source, &predicates, &nominals)?;
    Ok(UserLexicon {
        predicates,
        nominals,
    })
}

fn parse_predicate(
    source: &str,
    input: &str,
    raw: RawPredicate,
    rules: &RuleSet,
    known_rules: &BTreeSet<&str>,
) -> Result<UserPredicateRecord, DataError> {
    let line = value_line(input, &raw.lemma);
    require_nfc(source, line, "lemma", &raw.lemma)?;
    if raw.lemma.strip_suffix('다').is_none_or(str::is_empty) {
        return Err(semantic_error(
            source,
            line,
            DataErrorKind::InvalidPredicateLemma(raw.lemma),
        ));
    }
    let pos = parse_predicate_pos(source, line, &raw.pos)?;
    let alternation = DataAlternation::parse(&raw.alternation).ok_or_else(|| {
        invalid_value(
            source,
            line,
            "alternation",
            &raw.alternation,
            "알려진 lexical alternation이 아닙니다",
        )
    })?;
    require_known_rule(source, line, alternation.rule_id(), known_rules)?;
    let flags = parse_flags(source, line, raw.flags)?;
    let allowed_flags = rules
        .alternations
        .iter()
        .find(|rule| rule.kind == alternation)
        .map(|rule| {
            rule.flags
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    if let Some(flag) = flags
        .iter()
        .find(|flag| !allowed_flags.contains(flag.as_str()))
    {
        return Err(invalid_value(
            source,
            line,
            "flags",
            flag,
            "alternation 규칙에 선언되지 않은 flag입니다",
        ));
    }
    let overrides = parse_overrides(source, line, raw.overrides, known_rules)?;
    Ok(UserPredicateRecord {
        entry: PredicateRecord {
            lemma: raw.lemma,
            pos,
            alternation,
            flags,
            overrides,
        },
        replace: raw.replace,
    })
}

fn validate_override_conflicts(
    source: &str,
    predicates: &[UserPredicateRecord],
    nominals: &[UserNominalRecord],
) -> Result<(), DataError> {
    let mut seen = BTreeMap::<(&str, &str), &str>::new();
    let entries = predicates
        .iter()
        .flat_map(|record| {
            record
                .entry
                .overrides
                .iter()
                .map(move |entry| (record.entry.lemma.as_str(), entry))
        })
        .chain(nominals.iter().flat_map(|record| {
            record
                .entry
                .overrides
                .iter()
                .map(move |entry| (record.entry.lemma.as_str(), entry))
        }));
    for (lemma, entry) in entries {
        let key = (lemma, entry.rule_id.as_str());
        if let Some(first) = seen.insert(key, &entry.surface) {
            if first != entry.surface {
                return Err(semantic_error(
                    source,
                    None,
                    DataErrorKind::OverrideConflict {
                        lemma: lemma.to_owned(),
                        rule_id: entry.rule_id.clone(),
                        first: first.to_owned(),
                        second: entry.surface.clone(),
                    },
                ));
            }
        }
    }
    Ok(())
}

fn parse_nominal(
    source: &str,
    input: &str,
    raw: RawNominal,
    known_rules: &BTreeSet<&str>,
) -> Result<UserNominalRecord, DataError> {
    let line = value_line(input, &raw.surface);
    require_nfc(source, line, "surface", &raw.surface)?;
    if raw.surface.is_empty() {
        return Err(invalid_value(source, line, "surface", "", "비어 있습니다"));
    }
    let pos = parse_nominal_pos(source, line, &raw.pos)?;
    let flags = parse_flags(source, line, raw.flags)?;
    let overrides = parse_overrides(source, line, raw.overrides, known_rules)?;
    Ok(UserNominalRecord {
        entry: NominalRecord {
            lemma: raw.surface,
            pos,
            flags,
            overrides,
        },
        replace: raw.replace,
    })
}

fn parse_overrides(
    source: &str,
    line: Option<usize>,
    raw: Vec<RawOverride>,
    known_rules: &BTreeSet<&str>,
) -> Result<Vec<SurfaceOverride>, DataError> {
    let mut overrides = Vec::with_capacity(raw.len());
    let mut seen = BTreeMap::<String, String>::new();
    for entry in raw {
        require_rule_id(source, &entry.rule_id)?;
        require_known_rule(source, line, &entry.rule_id, known_rules)?;
        require_nfc(source, line, "override surface", &entry.surface)?;
        if entry.surface.is_empty() {
            return Err(invalid_value(
                source,
                line,
                "override surface",
                "",
                "비어 있습니다",
            ));
        }
        if let Some(first) = seen.get(&entry.rule_id) {
            if first != &entry.surface {
                return Err(semantic_error(
                    source,
                    line,
                    DataErrorKind::OverrideConflict {
                        lemma: "사용자 사전 항목".to_owned(),
                        rule_id: entry.rule_id,
                        first: first.clone(),
                        second: entry.surface,
                    },
                ));
            }
            continue;
        }
        seen.insert(entry.rule_id.clone(), entry.surface.clone());
        overrides.push(SurfaceOverride {
            rule_id: entry.rule_id,
            surface: entry.surface,
        });
    }
    Ok(overrides)
}

fn parse_flags(
    source: &str,
    line: Option<usize>,
    flags: Vec<String>,
) -> Result<BTreeSet<String>, DataError> {
    let mut parsed = BTreeSet::new();
    for flag in flags {
        if flag.is_empty()
            || !flag
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(invalid_value(
                source,
                line,
                "flags",
                &flag,
                "대문자 ASCII identifier여야 합니다",
            ));
        }
        parsed.insert(flag);
    }
    Ok(parsed)
}

fn parse_predicate_pos(
    source: &str,
    line: Option<usize>,
    value: &str,
) -> Result<DataFinePos, DataError> {
    match value {
        "verb" => Ok(DataFinePos::Vv),
        "adjective" => Ok(DataFinePos::Va),
        "auxiliary-verb" | "auxiliary-adjective" => Ok(DataFinePos::Vx),
        "copula" => Ok(DataFinePos::Vcp),
        "negative-copula" => Ok(DataFinePos::Vcn),
        _ => Err(invalid_value(
            source,
            line,
            "pos",
            value,
            "사용자 사전 predicate POS가 아닙니다",
        )),
    }
}

fn parse_nominal_pos(
    source: &str,
    line: Option<usize>,
    value: &str,
) -> Result<DataFinePos, DataError> {
    match value {
        "noun" => Ok(DataFinePos::Nng),
        "proper-noun" => Ok(DataFinePos::Nnp),
        "dependent-noun" => Ok(DataFinePos::Nnb),
        "pronoun" => Ok(DataFinePos::Np),
        "numeral" => Ok(DataFinePos::Nr),
        _ => Err(invalid_value(
            source,
            line,
            "pos",
            value,
            "사용자 사전 nominal POS가 아닙니다",
        )),
    }
}

fn require_known_rule(
    source: &str,
    line: Option<usize>,
    rule_id: &str,
    known_rules: &BTreeSet<&str>,
) -> Result<(), DataError> {
    if known_rules.contains(rule_id) {
        Ok(())
    } else {
        Err(semantic_error(
            source,
            line,
            DataErrorKind::UnknownRuleId(rule_id.to_owned()),
        ))
    }
}

fn invalid_value(
    source: &str,
    line: Option<usize>,
    field: &str,
    value: &str,
    reason: &str,
) -> DataError {
    semantic_error(
        source,
        line,
        DataErrorKind::InvalidValue {
            field: field.to_owned(),
            value: value.to_owned(),
            reason: reason.to_owned(),
        },
    )
}

fn semantic_error(source: &str, line: Option<usize>, kind: DataErrorKind) -> DataError {
    let mut location = SourceLocation::new(source);
    location.line = line;
    DataError::new(location, kind)
}

fn value_line(input: &str, value: &str) -> Option<usize> {
    input
        .lines()
        .position(|line| line.contains(value))
        .map(|index| index + 1)
}

fn set_span_location(location: &mut SourceLocation, input: &str, offset: usize) {
    let prefix = &input[..offset];
    location.line = Some(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1);
    location.column = Some(
        prefix
            .rsplit_once('\n')
            .map_or(prefix.len(), |(_, tail)| tail.len())
            + 1,
    );
}

fn default_nominal_pos() -> String {
    "noun".to_owned()
}
