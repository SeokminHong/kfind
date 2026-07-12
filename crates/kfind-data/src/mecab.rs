use std::io::BufRead;

use unicode_normalization::UnicodeNormalization;

use crate::binary::{ApprovedPosLexicon, PosLexiconEntry};
use crate::lexicon::DataFinePos;
use crate::{DataError, DataErrorKind};

const MAX_CONNECTION_COSTS: usize = 1 << 24;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MecabConnectionMatrix {
    right_contexts: u16,
    left_contexts: u16,
    costs: Vec<i16>,
}

impl MecabConnectionMatrix {
    #[must_use]
    pub fn right_contexts(&self) -> u16 {
        self.right_contexts
    }

    #[must_use]
    pub fn left_contexts(&self) -> u16 {
        self.left_contexts
    }

    #[must_use]
    pub fn costs(&self) -> &[i16] {
        &self.costs
    }

    #[must_use]
    pub fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        let index = usize::from(right_id)
            .checked_mul(usize::from(self.left_contexts))?
            .checked_add(usize::from(left_id))?;
        self.costs.get(index).copied()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MecabMorphologyEntry {
    pub surface: String,
    pub pos: DataFinePos,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MecabMorphologyExtraction {
    entries: Vec<MecabMorphologyEntry>,
    pub rows_read: usize,
    pub skipped_unsupported_pos: usize,
    pub duplicate_entries: usize,
    pub normalized_surfaces: usize,
}

impl MecabMorphologyExtraction {
    pub fn entries(&self) -> &[MecabMorphologyEntry] {
        &self.entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Normalized POS-only extraction output.
pub struct MecabExtraction {
    entries: Vec<PosLexiconEntry>,
    pub rows_read: usize,
    pub skipped_analysis_rows: usize,
    pub skipped_unsupported_pos: usize,
    pub skipped_noncanonical_copula_rows: usize,
    pub duplicate_entries: usize,
    pub predicate_candidates: usize,
    pub normalized_headwords: usize,
}

impl MecabExtraction {
    pub fn candidates(&self) -> &[PosLexiconEntry] {
        &self.entries
    }

    pub fn into_pos_lexicon(self) -> ApprovedPosLexicon {
        ApprovedPosLexicon::from_entries(self.entries)
    }

    /// Merges extraction output from multiple mecab-ko-dic CSV files.
    pub fn merge(mut self, other: Self) -> Self {
        let entries_before_deduplication = self.entries.len() + other.entries.len();
        self.entries.extend(other.entries);
        self.entries.sort_unstable();
        self.entries.dedup();

        self.rows_read += other.rows_read;
        self.skipped_analysis_rows += other.skipped_analysis_rows;
        self.skipped_unsupported_pos += other.skipped_unsupported_pos;
        self.skipped_noncanonical_copula_rows += other.skipped_noncanonical_copula_rows;
        self.duplicate_entries +=
            other.duplicate_entries + entries_before_deduplication - self.entries.len();
        self.predicate_candidates = self
            .entries
            .iter()
            .filter(|entry| entry.pos.is_predicate())
            .count();
        self.normalized_headwords += other.normalized_headwords;
        self
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
    let mut skipped_noncanonical_copula_rows = 0;
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
        if !is_canonical_copula_stem(pos, original) {
            skipped_noncanonical_copula_rows += 1;
            continue;
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
    let predicate_candidates = entries
        .iter()
        .filter(|entry| entry.pos.is_predicate())
        .count();
    Ok(MecabExtraction {
        entries,
        rows_read,
        skipped_analysis_rows,
        skipped_unsupported_pos,
        skipped_noncanonical_copula_rows,
        duplicate_entries,
        predicate_candidates,
        normalized_headwords,
    })
}

pub fn extract_mecab_morphology(
    source: &str,
    reader: impl BufRead,
) -> Result<MecabMorphologyExtraction, DataError> {
    let mut entries = Vec::new();
    let mut rows_read = 0;
    let mut skipped_unsupported_pos = 0;
    let mut normalized_surfaces = 0;

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
        let original = fields[0].as_str();
        if original.is_empty() {
            return Err(invalid_value(source, line_number, "surface", original));
        }
        let surface = original.nfc().collect::<String>();
        normalized_surfaces += usize::from(surface != original);
        entries.push(MecabMorphologyEntry {
            surface,
            pos,
            left_id: parse_integer(source, line_number, "left_id", &fields[1])?,
            right_id: parse_integer(source, line_number, "right_id", &fields[2])?,
            word_cost: parse_integer(source, line_number, "word_cost", &fields[3])?,
        });
    }

    entries.sort_unstable();
    let original_count = entries.len();
    entries.dedup();
    Ok(MecabMorphologyExtraction {
        duplicate_entries: original_count - entries.len(),
        entries,
        rows_read,
        skipped_unsupported_pos,
        normalized_surfaces,
    })
}

pub fn parse_mecab_connection_matrix(
    source: &str,
    reader: impl BufRead,
) -> Result<MecabConnectionMatrix, DataError> {
    let mut lines = reader.lines().enumerate();
    let (header_line, header) = next_matrix_line(source, &mut lines)?.ok_or_else(|| {
        DataError::line(
            source,
            1,
            DataErrorKind::InvalidHeader {
                expected: "RIGHT_CONTEXTS LEFT_CONTEXTS".to_owned(),
                actual: String::new(),
            },
        )
    })?;
    let header_fields = header.split_whitespace().collect::<Vec<_>>();
    if header_fields.len() != 2 {
        return Err(DataError::line(
            source,
            header_line,
            DataErrorKind::InvalidFieldCount {
                expected: 2,
                actual: header_fields.len(),
            },
        ));
    }
    let right_contexts =
        parse_integer::<u16>(source, header_line, "right_contexts", header_fields[0])?;
    let left_contexts =
        parse_integer::<u16>(source, header_line, "left_contexts", header_fields[1])?;
    let cost_count = usize::from(right_contexts)
        .checked_mul(usize::from(left_contexts))
        .filter(|count| *count > 0 && *count <= MAX_CONNECTION_COSTS)
        .ok_or_else(|| invalid_value(source, header_line, "matrix_dimensions", &header))?;
    let mut costs = vec![0_i16; cost_count];
    let mut seen = vec![0_u8; cost_count.div_ceil(8)];
    let mut parsed_costs = 0;

    while let Some((line_number, line)) = next_matrix_line(source, &mut lines)? {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(DataError::line(
                source,
                line_number,
                DataErrorKind::InvalidFieldCount {
                    expected: 3,
                    actual: fields.len(),
                },
            ));
        }
        let right_id = parse_integer::<u16>(source, line_number, "right_id", fields[0])?;
        let left_id = parse_integer::<u16>(source, line_number, "left_id", fields[1])?;
        if right_id >= right_contexts || left_id >= left_contexts {
            return Err(invalid_value(source, line_number, "context_id", &line));
        }
        let index = usize::from(right_id) * usize::from(left_contexts) + usize::from(left_id);
        let mask = 1_u8 << (index % 8);
        if seen[index / 8] & mask != 0 {
            return Err(invalid_value(
                source,
                line_number,
                "duplicate_context",
                &line,
            ));
        }
        seen[index / 8] |= mask;
        costs[index] = parse_integer(source, line_number, "cost", fields[2])?;
        parsed_costs += 1;
    }
    if parsed_costs != cost_count {
        return Err(invalid_value(
            source,
            header_line,
            "matrix_entries",
            &parsed_costs.to_string(),
        ));
    }
    Ok(MecabConnectionMatrix {
        right_contexts,
        left_contexts,
        costs,
    })
}

fn next_matrix_line(
    source: &str,
    lines: &mut impl Iterator<Item = (usize, std::io::Result<String>)>,
) -> Result<Option<(usize, String)>, DataError> {
    for (line_index, line) in lines {
        let line_number = line_index + 1;
        let line = line.map_err(|error| {
            DataError::line(source, line_number, DataErrorKind::Io(error.to_string()))
        })?;
        let content = line
            .split_once('#')
            .map_or(line.as_str(), |(value, _)| value)
            .trim();
        if !content.is_empty() {
            return Ok(Some((line_number, content.to_owned())));
        }
    }
    Ok(None)
}

fn is_canonical_copula_stem(pos: DataFinePos, surface: &str) -> bool {
    match pos {
        DataFinePos::Vcp => surface == "이",
        DataFinePos::Vcn => surface == "아니",
        _ => true,
    }
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
            reason: "mecab-ko-dic 스키마에 맞지 않습니다".to_owned(),
        },
    )
}

fn parse_integer<T>(source: &str, line: usize, field: &str, value: &str) -> Result<T, DataError>
where
    T: std::str::FromStr,
{
    value
        .parse()
        .map_err(|_| invalid_value(source, line, field, value))
}
