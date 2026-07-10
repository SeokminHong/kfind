use std::collections::HashMap;

use crate::{DataError, DataErrorKind, DataWarning, SourceLocation};

pub(crate) struct TsvRow<'a> {
    pub line: usize,
    pub fields: Vec<&'a str>,
}

pub(crate) struct ParsedRows<'a> {
    pub rows: Vec<TsvRow<'a>>,
    pub warnings: Vec<DataWarning>,
}

pub(crate) fn parse_rows<'a>(
    source: &str,
    input: &'a str,
    expected_header: &[&str],
) -> Result<ParsedRows<'a>, DataError> {
    let mut meaningful_lines = input
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
    let Some((header_index, raw_header)) = meaningful_lines.next() else {
        return Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::InvalidHeader {
                expected: expected_header.join("\t"),
                actual: String::new(),
            },
        ));
    };
    let header = raw_header.trim_end_matches('\r');
    let expected = expected_header.join("\t");
    if header != expected {
        return Err(DataError::line(
            source,
            header_index + 1,
            DataErrorKind::InvalidHeader {
                expected,
                actual: header.to_owned(),
            },
        ));
    }

    let mut seen = HashMap::<&str, usize>::new();
    let mut rows = Vec::new();
    let mut warnings = Vec::new();
    for (line_index, raw_line) in meaningful_lines {
        let line_number = line_index + 1;
        let line = raw_line.trim_end_matches('\r');
        if let Some(first_line) = seen.get(line).copied() {
            warnings.push(DataWarning::DuplicateRow {
                location: SourceLocation::at_line(source, line_number),
                first_line,
            });
            continue;
        }
        seen.insert(line, line_number);

        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != expected_header.len() {
            return Err(DataError::line(
                source,
                line_number,
                DataErrorKind::InvalidFieldCount {
                    expected: expected_header.len(),
                    actual: fields.len(),
                },
            ));
        }
        rows.push(TsvRow {
            line: line_number,
            fields,
        });
    }
    Ok(ParsedRows { rows, warnings })
}
