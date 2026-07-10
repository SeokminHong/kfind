use crate::tsv::parse_rows;
use crate::validation::require_nfc;
use crate::{DataError, DataErrorKind, DataWarning};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FixturePos {
    Noun,
    Pronoun,
    Numeral,
    Verb,
    Adjective,
    Copula,
    Determiner,
    Adverb,
    Particle,
    Interjection,
    Literal,
}

impl FixturePos {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "noun" => Self::Noun,
            "pronoun" => Self::Pronoun,
            "numeral" => Self::Numeral,
            "verb" => Self::Verb,
            "adjective" => Self::Adjective,
            "copula" => Self::Copula,
            "determiner" => Self::Determiner,
            "adverb" => Self::Adverb,
            "particle" => Self::Particle,
            "interjection" => Self::Interjection,
            "literal" => Self::Literal,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExpectedMatch {
    Match,
    NoMatch,
}

impl ExpectedMatch {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "match" => Self::Match,
            "no-match" => Self::NoMatch,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyCase {
    pub query: String,
    pub pos: FixturePos,
    pub text: String,
    pub expected: ExpectedMatch,
    pub feature: String,
}

pub fn parse_morphology_cases_tsv(
    source: &str,
    input: &str,
) -> Result<(Vec<MorphologyCase>, Vec<DataWarning>), DataError> {
    let parsed = parse_rows(
        source,
        input,
        &["query", "pos", "text", "expected", "feature"],
    )?;
    let mut cases = Vec::with_capacity(parsed.rows.len());
    for row in parsed.rows {
        for (field, value) in [("query", row.fields[0]), ("text", row.fields[2])] {
            require_nfc(source, Some(row.line), field, value)?;
            if value.is_empty() {
                return Err(invalid_value(source, row.line, field, value));
            }
        }
        let pos = FixturePos::parse(row.fields[1])
            .ok_or_else(|| invalid_value(source, row.line, "pos", row.fields[1]))?;
        let expected = ExpectedMatch::parse(row.fields[3])
            .ok_or_else(|| invalid_value(source, row.line, "expected", row.fields[3]))?;
        let feature = row.fields[4];
        if feature.is_empty()
            || !feature.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
            })
        {
            return Err(invalid_value(source, row.line, "feature", feature));
        }
        cases.push(MorphologyCase {
            query: row.fields[0].to_owned(),
            pos,
            text: row.fields[2].to_owned(),
            expected,
            feature: feature.to_owned(),
        });
    }
    Ok((cases, parsed.warnings))
}

fn invalid_value(source: &str, line: usize, field: &str, value: &str) -> DataError {
    DataError::line(
        source,
        line,
        DataErrorKind::InvalidValue {
            field: field.to_owned(),
            value: value.to_owned(),
            reason: "morphology fixture 스키마에 맞지 않습니다".to_owned(),
        },
    )
}
