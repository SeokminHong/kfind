use std::collections::BTreeSet;

use crate::tsv::parse_rows;
use crate::validation::{require_nfc, require_rule_id};
use crate::{DataError, DataErrorKind, DataWarning};

use super::{
    DataAlternation, DataFinePos, LexiconData, ModifierRecord, NominalRecord, ParticleRecord,
    PredicateRecord, SurfaceOverride,
};

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
