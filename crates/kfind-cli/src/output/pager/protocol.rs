use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

const MATCH_LINE_PREFIX: &[u8] = b"\x1eKFIND_MATCH\t";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ColumnRange {
    pub start: usize,
    pub end: usize,
}

impl ColumnRange {
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PagerMatch {
    pub span: ColumnRange,
    pub tokens: Vec<ColumnRange>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct MatchLine {
    pub content: String,
    pub prefixes: Vec<String>,
    pub matches: Vec<PagerMatch>,
    pub color: bool,
}

impl MatchLine {
    fn validate(&self) -> io::Result<()> {
        if self.matches.is_empty() || self.prefixes.len() != self.matches.len() {
            return Err(invalid_protocol("match and prefix counts differ"));
        }
        let width = UnicodeWidthStr::width(self.content.as_str());
        for matched in &self.matches {
            validate_range(matched.span, width)?;
            for token in &matched.tokens {
                validate_range(*token, width)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn write_match_line(writer: &mut impl Write, line: &MatchLine) -> io::Result<()> {
    line.validate()?;
    writer.write_all(MATCH_LINE_PREFIX)?;
    serde_json::to_writer(&mut *writer, line).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

pub(crate) fn decode_match_line(bytes: &[u8]) -> io::Result<Option<MatchLine>> {
    let Some(payload) = bytes.strip_prefix(MATCH_LINE_PREFIX) else {
        return Ok(None);
    };
    let line = serde_json::from_slice::<MatchLine>(payload).map_err(io::Error::other)?;
    line.validate()?;
    Ok(Some(line))
}

fn validate_range(range: ColumnRange, width: usize) -> io::Result<()> {
    if range.start <= range.end && range.end <= width {
        Ok(())
    } else {
        Err(invalid_protocol("display range is outside the line"))
    }
}

fn invalid_protocol(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> MatchLine {
        MatchLine {
            content: "앞 걸어 뒤".to_owned(),
            prefixes: vec!["sample:1: ".to_owned()],
            matches: vec![PagerMatch {
                span: ColumnRange::new(3, 7),
                tokens: vec![ColumnRange::new(3, 7)],
            }],
            color: true,
        }
    }

    #[test]
    fn match_line_round_trips_through_the_internal_protocol() {
        let expected = line();
        let mut bytes = Vec::new();

        write_match_line(&mut bytes, &expected).unwrap();
        let decoded = decode_match_line(bytes.strip_suffix(b"\n").unwrap()).unwrap();

        assert_eq!(decoded, Some(expected));
    }

    #[test]
    fn protocol_rejects_ranges_outside_the_displayed_content() {
        let mut invalid = line();
        invalid.matches[0].span.end = 99;

        assert_eq!(
            write_match_line(&mut Vec::new(), &invalid)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
