use std::collections::BTreeSet;
use std::io::BufRead;

use unicode_normalization::UnicodeNormalization;

use crate::binary::PosLexiconEntry;
use crate::lexicon::DataFinePos;
use crate::{DataError, DataErrorKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MecabExtraction {
    entries: Vec<PosLexiconEntry>,
    pub rows_read: usize,
    pub skipped_analysis_rows: usize,
    pub skipped_unsupported_pos: usize,
    pub duplicate_entries: usize,
    pub predicate_candidates_requiring_gold: usize,
    pub normalized_headwords: usize,
}

impl MecabExtraction {
    pub fn candidates(&self) -> &[PosLexiconEntry] {
        &self.entries
    }

    pub fn retain_gold_approved_predicates(
        &self,
        approved: &BTreeSet<PosLexiconEntry>,
    ) -> Vec<PosLexiconEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.pos.is_predicate() || approved.contains(*entry))
            .cloned()
            .collect()
    }
}

pub fn extract_mecab_ko_dic(
    source: &str,
    reader: impl BufRead,
) -> Result<MecabExtraction, DataError> {
    let mut entries = Vec::new();
    let mut rows_read = 0;
    let mut skipped_analysis_rows = 0;
    let mut skipped_unsupported_pos = 0;
    let mut normalized_headwords = 0;

    for (line_index, line) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.map_err(|error| {
            DataError::line(source, line_number, DataErrorKind::Io(error.to_string()))
        })?;
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        rows_read += 1;
        let fields = parse_csv_record(source, line_number, line)?;
        if fields.len() < 12 {
            return Err(DataError::line(
                source,
                line_number,
                DataErrorKind::InvalidFieldCount {
                    expected: 12,
                    actual: fields.len(),
                },
            ));
        }
        let Some(pos) = DataFinePos::parse(&fields[4]) else {
            skipped_unsupported_pos += 1;
            continue;
        };
        if matches!(fields[8].as_str(), "Inflect" | "Preanalysis") {
            skipped_analysis_rows += 1;
            continue;
        }
        let original = fields[0].as_str();
        if original.is_empty() {
            return Err(invalid_value(source, line_number, "surface", original));
        }
        let mut lemma = original.nfc().collect::<String>();
        normalized_headwords += usize::from(lemma != original);
        if pos.is_predicate() && !lemma.ends_with('다') {
            lemma.push('다');
        }
        entries.push(PosLexiconEntry { lemma, pos });
    }

    entries.sort_unstable();
    let original_count = entries.len();
    entries.dedup();
    let duplicate_entries = original_count - entries.len();
    let predicate_candidates_requiring_gold = entries
        .iter()
        .filter(|entry| entry.pos.is_predicate())
        .count();
    Ok(MecabExtraction {
        entries,
        rows_read,
        skipped_analysis_rows,
        skipped_unsupported_pos,
        duplicate_entries,
        predicate_candidates_requiring_gold,
        normalized_headwords,
    })
}

fn parse_csv_record(source: &str, line: usize, input: &str) -> Result<Vec<String>, DataError> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut quoted = false;
    let mut after_quote = false;

    while let Some(character) = chars.next() {
        if quoted {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    after_quote = true;
                }
            } else {
                field.push(character);
            }
            continue;
        }
        if after_quote {
            if character == ',' {
                fields.push(std::mem::take(&mut field));
                after_quote = false;
            } else {
                return Err(invalid_value(source, line, "csv", input));
            }
            continue;
        }
        match character {
            ',' => fields.push(std::mem::take(&mut field)),
            '"' if field.is_empty() => quoted = true,
            '"' => return Err(invalid_value(source, line, "csv", input)),
            _ => field.push(character),
        }
    }
    if quoted {
        return Err(invalid_value(source, line, "csv", input));
    }
    fields.push(field);
    Ok(fields)
}

fn invalid_value(source: &str, line: usize, field: &str, value: &str) -> DataError {
    DataError::line(
        source,
        line,
        DataErrorKind::InvalidValue {
            field: field.to_owned(),
            value: value.to_owned(),
            reason: "mecab-ko-dic CSV 스키마에 맞지 않습니다".to_owned(),
        },
    )
}
