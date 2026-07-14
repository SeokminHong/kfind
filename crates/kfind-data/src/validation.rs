use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use unicode_normalization::is_nfc;

use crate::fixture::MorphologyCase;
use crate::lexicon::{LexiconData, LexiconSources};
use crate::rules::{RuleSet, RuleSources};
use crate::{DataError, DataErrorKind, DataWarning, SourceLocation};

pub(crate) fn require_nfc(
    source: &str,
    line: Option<usize>,
    field: &str,
    value: &str,
) -> Result<(), DataError> {
    if is_nfc(value) {
        return Ok(());
    }
    let mut location = SourceLocation::new(source);
    location.line = line;
    Err(DataError::new(
        location,
        DataErrorKind::NonNfc {
            field: field.to_owned(),
            value: value.to_owned(),
        },
    ))
}

pub(crate) fn require_rule_id(source: &str, id: &str) -> Result<(), DataError> {
    let valid = !id.is_empty()
        && id.split('.').count() >= 2
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::InvalidValue {
                field: "id".to_owned(),
                value: id.to_owned(),
                reason: "소문자 ASCII namespace 형식이어야 합니다".to_owned(),
            },
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedData {
    pub lexicon: LexiconData,
    pub rules: RuleSet,
    pub fixtures: Vec<MorphologyCase>,
    pub warnings: Vec<DataWarning>,
}

pub fn validate_data(
    lexicon: LexiconData,
    rules: RuleSet,
    fixtures: Vec<MorphologyCase>,
    warnings: Vec<DataWarning>,
) -> Result<ValidatedData, DataError> {
    validate_lexicon_rule_references(&lexicon, &rules)?;
    validate_override_conflicts(&lexicon)?;
    validate_fixture_coverage(&lexicon, &fixtures)?;
    Ok(ValidatedData {
        lexicon,
        rules,
        fixtures,
        warnings,
    })
}

fn validate_lexicon_rule_references(
    lexicon: &LexiconData,
    rules: &RuleSet,
) -> Result<(), DataError> {
    validate_predicates("data/lexicon/predicates.tsv", &lexicon.predicates, rules)?;
    let ids = rules.all_ids().collect::<BTreeSet<_>>();
    for nominal in &lexicon.nominals {
        for entry in &nominal.overrides {
            if !ids.contains(entry.rule_id.as_str()) {
                return Err(DataError::new(
                    SourceLocation::new("data/lexicon/nominals.tsv"),
                    DataErrorKind::UnknownRuleId(entry.rule_id.clone()),
                ));
            }
        }
    }
    for particle in &lexicon.particles {
        let Some(rule) = rules
            .particles
            .iter()
            .find(|rule| rule.id == particle.rule_id)
        else {
            return Err(DataError::new(
                SourceLocation::new("data/lexicon/particles.tsv"),
                DataErrorKind::UnknownRuleId(particle.rule_id.clone()),
            ));
        };
        let variants = particle
            .variants
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let forms = rule
            .forms
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if variants != forms {
            return Err(DataError::new(
                SourceLocation::new("data/lexicon/particles.tsv"),
                DataErrorKind::InvalidValue {
                    field: "variants".to_owned(),
                    value: particle.variants.join("|"),
                    reason: format!("{} 규칙의 forms와 정확히 일치해야 합니다", particle.rule_id),
                },
            ));
        }
    }
    Ok(())
}

/// Validates predicate records against the active alternation rules.
pub fn validate_predicates(
    source: &str,
    predicates: &[crate::PredicateRecord],
    rules: &RuleSet,
) -> Result<(), DataError> {
    let ids = rules.all_ids().collect::<BTreeSet<_>>();
    for predicate in predicates {
        let alternation_id = predicate.alternation.rule_id();
        let alternation_rule = rules
            .alternations
            .iter()
            .find(|rule| rule.id == alternation_id && rule.kind == predicate.alternation);
        if alternation_rule.is_none() {
            return Err(DataError::new(
                SourceLocation::new(source),
                DataErrorKind::UnknownRuleId(alternation_id.to_owned()),
            ));
        }
        let allowed_flags = alternation_rule
            .map(|rule| {
                rule.flags
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        if let Some(flag) = predicate
            .flags
            .iter()
            .find(|flag| !allowed_flags.contains(flag.as_str()))
        {
            return Err(DataError::new(
                SourceLocation::new(source),
                DataErrorKind::InvalidValue {
                    field: "flags".to_owned(),
                    value: flag.clone(),
                    reason: format!(
                        "{} 규칙에 선언되지 않은 predicate flag입니다",
                        predicate.alternation.as_str()
                    ),
                },
            ));
        }
        if predicate.alternation == crate::DataAlternation::SurfaceOnly
            && (predicate.overrides.len() != 1
                || !predicate
                    .overrides
                    .iter()
                    .all(|entry| crate::is_dictionary_surface_rule(&entry.rule_id)))
        {
            return Err(DataError::new(
                SourceLocation::new(source),
                DataErrorKind::InvalidValue {
                    field: "overrides".to_owned(),
                    value: predicate
                        .overrides
                        .iter()
                        .map(|entry| entry.rule_id.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    reason: "SurfaceOnly는 사전 provenance override 하나만 가져야 합니다"
                        .to_owned(),
                },
            ));
        }
        for entry in &predicate.overrides {
            if !ids.contains(entry.rule_id.as_str())
                && !(predicate.alternation == crate::DataAlternation::SurfaceOnly
                    && crate::is_dictionary_surface_rule(&entry.rule_id))
            {
                return Err(DataError::new(
                    SourceLocation::new(source),
                    DataErrorKind::UnknownRuleId(entry.rule_id.clone()),
                ));
            }
        }
    }
    Ok(())
}

fn validate_override_conflicts(lexicon: &LexiconData) -> Result<(), DataError> {
    let mut values = BTreeMap::<(String, String), String>::new();
    let all_overrides = lexicon
        .predicates
        .iter()
        .filter(|record| record.alternation != crate::DataAlternation::SurfaceOnly)
        .flat_map(|record| {
            record
                .overrides
                .iter()
                .map(move |entry| (record.lemma.as_str(), entry))
        })
        .chain(lexicon.nominals.iter().flat_map(|record| {
            record
                .overrides
                .iter()
                .map(move |entry| (record.lemma.as_str(), entry))
        }));

    for (lemma, entry) in all_overrides {
        let key = (lemma.to_owned(), entry.rule_id.clone());
        if let Some(first) = values.get(&key) {
            if first != &entry.surface {
                return Err(DataError::new(
                    SourceLocation::new("lexicon overrides"),
                    DataErrorKind::OverrideConflict {
                        lemma: lemma.to_owned(),
                        rule_id: entry.rule_id.clone(),
                        first: first.clone(),
                        second: entry.surface.clone(),
                    },
                ));
            }
        } else {
            values.insert(key, entry.surface.clone());
        }
    }
    Ok(())
}

fn validate_fixture_coverage(
    lexicon: &LexiconData,
    fixtures: &[MorphologyCase],
) -> Result<(), DataError> {
    let covered = fixtures
        .iter()
        .map(|case| case.feature.as_str())
        .collect::<BTreeSet<_>>();
    for feature in lexicon
        .predicates
        .iter()
        .map(|record| record.alternation.fixture_feature())
        .collect::<BTreeSet<_>>()
    {
        if !covered.contains(feature) {
            return Err(DataError::new(
                SourceLocation::new("data/fixtures/morphology_cases.tsv"),
                DataErrorKind::MissingFixtureCoverage(feature.to_owned()),
            ));
        }
    }
    for flag in lexicon
        .predicates
        .iter()
        .flat_map(|record| record.flags.iter())
        .collect::<BTreeSet<_>>()
    {
        let feature = flag.to_ascii_lowercase().replace('_', "-");
        if !covered.contains(feature.as_str()) {
            return Err(DataError::new(
                SourceLocation::new("data/fixtures/morphology_cases.tsv"),
                DataErrorKind::MissingFixtureCoverage(feature),
            ));
        }
    }
    Ok(())
}

pub fn load_data_dir(root: impl AsRef<Path>) -> Result<ValidatedData, DataError> {
    let root = root.as_ref();
    let read = |relative: &str| -> Result<String, DataError> {
        fs::read_to_string(root.join(relative)).map_err(|error| {
            DataError::new(
                SourceLocation::new(root.join(relative).display().to_string()),
                DataErrorKind::Io(error.to_string()),
            )
        })
    };

    let predicates = read("lexicon/predicates.tsv")?;
    let nominals = read("lexicon/nominals.tsv")?;
    let modifiers = read("lexicon/modifiers.tsv")?;
    let particles = read("lexicon/particles.tsv")?;
    let endings = read("rules/endings.toml")?;
    let alternations = read("rules/alternations.toml")?;
    let contractions = read("rules/contractions.toml")?;
    let derivations = read("rules/derivations.toml")?;
    let particle_transitions = read("rules/particles.toml")?;
    let fixture_source = read("fixtures/morphology_cases.tsv")?;

    let (lexicon, mut warnings) = crate::parse_lexicons(LexiconSources {
        predicates: &predicates,
        nominals: &nominals,
        modifiers: &modifiers,
        particles: &particles,
    })?;
    let rules = crate::parse_rule_set(RuleSources {
        endings: &endings,
        alternations: &alternations,
        contractions: &contractions,
        derivations: &derivations,
        particles: &particle_transitions,
    })?;
    let (fixtures, fixture_warnings) =
        crate::parse_morphology_cases_tsv("data/fixtures/morphology_cases.tsv", &fixture_source)?;
    warnings.extend(fixture_warnings);
    validate_data(lexicon, rules, fixtures, warnings)
}
