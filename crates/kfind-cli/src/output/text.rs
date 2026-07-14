use std::borrow::Cow;
use std::io::{self, Write};
use std::ops::Range;
use std::path::Path;

use kfind_query::QueryPlan;
use kfind_search::{FileSearchResult, SearchLine, SearchLineKind, SearchRecord};
use unicode_width::UnicodeWidthStr;

use super::pager::{ColumnRange, MatchLine, PagerMatch, write_match_line};
use super::{OutputOptions, ResolvedColor, explain};

const MATCH_COLOR: &[u8] = b"\x1b[1;31m";
const RESET_COLOR: &[u8] = b"\x1b[0m";

pub(super) fn write_standard(
    writer: &mut impl Write,
    result: &FileSearchResult,
    plan: &QueryPlan,
    options: OutputOptions,
    terminal_pager: bool,
) -> io::Result<()> {
    for record in &result.records {
        write_record(writer, &result.path, record, plan, options, terminal_pager)?;
    }
    Ok(())
}

pub(super) fn write_record(
    writer: &mut impl Write,
    path: &Path,
    record: &SearchRecord,
    plan: &QueryPlan,
    options: OutputOptions,
    terminal_pager: bool,
) -> io::Result<()> {
    match record {
        SearchRecord::ContextBreak => writer.write_all(b"--\n"),
        SearchRecord::Line(line) => {
            write_line(writer, path, line, options, terminal_pager)?;
            if options.explain_match && line.kind == SearchLineKind::Match {
                explain::write_match_explanations(writer, line, plan, options.language)?;
            }
            Ok(())
        }
    }
}

pub(super) fn write_count(
    writer: &mut impl Write,
    result: &FileSearchResult,
    with_filename: bool,
) -> io::Result<()> {
    if with_filename {
        write_safe_bytes(writer, &path_bytes(&result.path))?;
        writer.write_all(b":")?;
    }
    writeln!(writer, "{}", result.matching_lines)
}

pub(super) fn write_filename_if_matched(
    writer: &mut impl Write,
    result: &FileSearchResult,
) -> io::Result<()> {
    if result.has_match() {
        write_safe_bytes(writer, &path_bytes(&result.path))?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

fn write_line(
    writer: &mut impl Write,
    path: &Path,
    line: &SearchLine,
    options: OutputOptions,
    terminal_pager: bool,
) -> io::Result<()> {
    let is_match = line.kind == SearchLineKind::Match;
    if is_match
        && terminal_pager
        && let Some(match_line) = terminal_match_line(path, line, options)
    {
        return write_match_line(writer, &match_line);
    }

    let delimiter = if is_match { b':' } else { b'-' };
    write_prefix(
        writer,
        path,
        line,
        options,
        delimiter,
        line.matches.first().map(|matched| matched.span.start),
    )?;
    let content = line_content(&line.bytes);
    let ranges = token_ranges(line, content.len());
    write_highlighted(
        writer,
        content,
        &ranges,
        is_match && options.color == ResolvedColor::Enabled,
    )?;
    writer.write_all(b"\n")
}

fn terminal_match_line(
    path: &Path,
    line: &SearchLine,
    options: OutputOptions,
) -> Option<MatchLine> {
    let content = line_content(&line.bytes);
    let safe = SafeContent::new(content);
    let mut prefixes = Vec::with_capacity(line.matches.len());
    let mut matches = Vec::with_capacity(line.matches.len());

    for matched in &line.matches {
        let span = safe.range(&matched.span)?;
        if span.start == span.end {
            return None;
        }
        let mut prefix = Vec::new();
        write_prefix(
            &mut prefix,
            path,
            line,
            options,
            b':',
            Some(matched.span.start),
        )
        .ok()?;
        prefixes.push(String::from_utf8(prefix).ok()?);
        let tokens = matched
            .atoms
            .iter()
            .filter_map(|atom| safe.range(&atom.token))
            .filter(|range| range.start < range.end)
            .collect();
        matches.push(PagerMatch { span, tokens });
    }

    (!matches.is_empty()).then_some(MatchLine {
        content: safe.text,
        prefixes,
        matches,
        color: options.color == ResolvedColor::Enabled,
    })
}

fn write_prefix(
    writer: &mut impl Write,
    path: &Path,
    line: &SearchLine,
    options: OutputOptions,
    delimiter: u8,
    match_start: Option<usize>,
) -> io::Result<()> {
    let mut has_prefix = false;

    if options.with_filename() {
        write_safe_bytes(writer, &path_bytes(path))?;
        writer.write_all(&[delimiter])?;
        has_prefix = true;
    }

    if options.line_number || options.column {
        match line.line_number {
            Some(number) => write!(writer, "{number}")?,
            None => writer.write_all(b"?")?,
        }
        writer.write_all(&[delimiter])?;
        has_prefix = true;
    }

    if options.column && line.kind == SearchLineKind::Match {
        match match_start.and_then(|start| scalar_column(line, start)) {
            Some(column) => write!(writer, "{column}")?,
            None => writer.write_all(b"?")?,
        }
        writer.write_all(&[delimiter])?;
        has_prefix = true;
    }

    if has_prefix {
        writer.write_all(b" ")?;
    }
    Ok(())
}

pub(super) fn line_content(bytes: &[u8]) -> &[u8] {
    let without_lf = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    without_lf.strip_suffix(b"\r").unwrap_or(without_lf)
}

pub(super) fn first_scalar_column(line: &SearchLine) -> Option<usize> {
    let start = line.matches.first()?.span.start;
    scalar_column(line, start)
}

fn scalar_column(line: &SearchLine, start: usize) -> Option<usize> {
    let prefix = line.bytes.get(..start)?;
    Some(std::str::from_utf8(prefix).ok()?.chars().count() + 1)
}

struct SafeContent {
    text: String,
    columns: Vec<usize>,
}

impl SafeContent {
    fn new(bytes: &[u8]) -> Self {
        let mut text = String::new();
        let mut columns = vec![0; bytes.len() + 1];
        let mut offset = 0;
        let mut column = 0;

        while offset < bytes.len() {
            match std::str::from_utf8(&bytes[offset..]) {
                Ok(valid) => {
                    append_valid(valid, offset, &mut text, &mut columns, &mut column);
                    offset = bytes.len();
                }
                Err(error) => {
                    let valid_len = error.valid_up_to();
                    if valid_len > 0 {
                        let valid = std::str::from_utf8(&bytes[offset..offset + valid_len])
                            .expect("valid_up_to always identifies valid UTF-8");
                        append_valid(valid, offset, &mut text, &mut columns, &mut column);
                        offset += valid_len;
                    }
                    let invalid_len = error.error_len().unwrap_or(bytes.len() - offset);
                    for byte in &bytes[offset..offset + invalid_len] {
                        columns[offset] = column;
                        let escaped = format!("\\x{byte:02X}");
                        text.push_str(&escaped);
                        column += UnicodeWidthStr::width(escaped.as_str());
                        offset += 1;
                        columns[offset] = column;
                    }
                }
            }
        }
        columns[bytes.len()] = column;
        Self { text, columns }
    }

    fn range(&self, range: &Range<usize>) -> Option<ColumnRange> {
        let start = *self.columns.get(range.start)?;
        let end = *self.columns.get(range.end)?;
        (start <= end).then_some(ColumnRange::new(start, end))
    }
}

fn append_valid(
    valid: &str,
    base: usize,
    text: &mut String,
    columns: &mut [usize],
    column: &mut usize,
) {
    for (relative, character) in valid.char_indices() {
        let start = base + relative;
        let end = start + character.len_utf8();
        columns[start..end].fill(*column);
        let escaped = safe_character(character);
        text.push_str(&escaped);
        *column += UnicodeWidthStr::width(escaped.as_str());
        columns[end] = *column;
    }
}

fn safe_character(character: char) -> String {
    match character {
        '\n' => "\\n".to_owned(),
        '\r' => "\\r".to_owned(),
        '\t' => "\\t".to_owned(),
        character if character.is_control() => format!("\\u{{{:04X}}}", character as u32),
        character => character.to_string(),
    }
}

fn token_ranges(line: &SearchLine, content_len: usize) -> Vec<Range<usize>> {
    let mut ranges = line
        .matches
        .iter()
        .flat_map(|matched| matched.atoms.iter())
        .filter(|atom| atom.token.start < atom.token.end && atom.token.end <= content_len)
        .map(|atom| atom.token.clone())
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| (range.start, range.end));

    let mut merged = Vec::<Range<usize>>::new();
    for range in ranges {
        if merged
            .last()
            .is_some_and(|previous| range.start <= previous.end)
        {
            let previous = merged.last_mut().expect("the previous range exists");
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn write_highlighted(
    writer: &mut impl Write,
    bytes: &[u8],
    ranges: &[Range<usize>],
    color: bool,
) -> io::Result<()> {
    if !color || ranges.is_empty() {
        return write_safe_bytes(writer, bytes);
    }

    let mut cursor = 0;
    for range in ranges {
        write_safe_bytes(writer, &bytes[cursor..range.start])?;
        writer.write_all(MATCH_COLOR)?;
        write_safe_bytes(writer, &bytes[range.clone()])?;
        writer.write_all(RESET_COLOR)?;
        cursor = range.end;
    }
    write_safe_bytes(writer, &bytes[cursor..])
}

pub(super) fn write_safe_bytes(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    let mut remaining = bytes;
    while !remaining.is_empty() {
        match std::str::from_utf8(remaining) {
            Ok(text) => {
                write_safe_text(writer, text)?;
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                if valid > 0 {
                    let text = std::str::from_utf8(&remaining[..valid])
                        .expect("valid_up_to always identifies valid UTF-8");
                    write_safe_text(writer, text)?;
                }
                let invalid_len = error.error_len().unwrap_or(remaining.len() - valid);
                for byte in &remaining[valid..valid + invalid_len] {
                    write!(writer, "\\x{byte:02X}")?;
                }
                remaining = &remaining[valid + invalid_len..];
            }
        }
    }
    Ok(())
}

fn write_safe_text(writer: &mut impl Write, text: &str) -> io::Result<()> {
    for character in text.chars() {
        match character {
            '\n' => writer.write_all(b"\\n")?,
            '\r' => writer.write_all(b"\\r")?,
            '\t' => writer.write_all(b"\\t")?,
            character if character.is_control() => {
                write!(writer, "\\u{{{:04X}}}", character as u32)?;
            }
            character => write!(writer, "{character}")?,
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn path_bytes(path: &Path) -> Cow<'_, [u8]> {
    use std::os::unix::ffi::OsStrExt;

    Cow::Borrowed(path.as_os_str().as_bytes())
}

#[cfg(not(unix))]
pub(super) fn path_bytes(path: &Path) -> Cow<'_, [u8]> {
    Cow::Owned(path.to_string_lossy().into_owned().into_bytes())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kfind_query::{PhraseMatch, VerifiedSpan};

    use super::*;
    use crate::output::FilenameMode;

    #[test]
    fn safe_content_maps_source_bytes_to_display_columns() {
        let safe = SafeContent::new("앞\t걸어".as_bytes());

        assert_eq!(safe.text, "앞\\t걸어");
        assert_eq!(safe.range(&(4..10)), Some(ColumnRange::new(4, 8)));

        let invalid = SafeContent::new(b"\xFF\t");
        assert_eq!(invalid.text, "\\xFF\\t");
        assert_eq!(invalid.columns, vec![0, 4, 6]);
    }

    #[test]
    fn terminal_match_rows_keep_each_match_column_and_display_span() {
        let bytes = "앞 걸어 중간 걸어 뒤\n".as_bytes().to_vec();
        let spans = [
            "앞 ".len().."앞 걸어".len(),
            "앞 걸어 중간 ".len().."앞 걸어 중간 걸어".len(),
        ];
        let matches = spans
            .iter()
            .cloned()
            .map(|span| PhraseMatch {
                span: span.clone(),
                atoms: vec![VerifiedSpan {
                    core: span.clone(),
                    token: span,
                    origins: Vec::new(),
                }],
            })
            .collect();
        let line = SearchLine {
            kind: SearchLineKind::Match,
            line_number: Some(7),
            absolute_byte_offset: 0,
            bytes,
            matches,
        };
        let options = OutputOptions {
            filename: FilenameMode::Always,
            line_number: true,
            column: true,
            color: ResolvedColor::Enabled,
            ..OutputOptions::default()
        };

        let payload = terminal_match_line(&PathBuf::from("sample.txt"), &line, options).unwrap();

        assert_eq!(payload.content, "앞 걸어 중간 걸어 뒤");
        assert_eq!(payload.prefixes, ["sample.txt:7:3: ", "sample.txt:7:9: "]);
        assert_eq!(payload.matches[0].span, ColumnRange::new(3, 7));
        assert_eq!(payload.matches[1].span, ColumnRange::new(13, 17));
        assert_eq!(payload.matches[0].tokens, [ColumnRange::new(3, 7)]);
        assert_eq!(payload.matches[1].tokens, [ColumnRange::new(13, 17)]);
    }
}
