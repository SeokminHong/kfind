use std::borrow::Cow;
use std::io::{self, Write};
use std::ops::Range;
use std::path::Path;

use kfind_query::QueryPlan;
use kfind_search::{FileSearchResult, SearchLine, SearchLineKind, SearchRecord};

use super::{OutputOptions, ResolvedColor, explain};

const MATCH_COLOR: &[u8] = b"\x1b[1;31m";
const RESET_COLOR: &[u8] = b"\x1b[0m";

pub(super) fn write_standard(
    writer: &mut impl Write,
    result: &FileSearchResult,
    plan: &QueryPlan,
    options: OutputOptions,
) -> io::Result<()> {
    for record in &result.records {
        match record {
            SearchRecord::ContextBreak => writer.write_all(b"--\n")?,
            SearchRecord::Line(line) => {
                write_line(writer, &result.path, line, options)?;
                if options.explain_match && line.kind == SearchLineKind::Match {
                    explain::write_match_explanations(writer, line, plan, options.language)?;
                }
            }
        }
    }
    Ok(())
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
) -> io::Result<()> {
    let is_match = line.kind == SearchLineKind::Match;
    let delimiter = if is_match { b':' } else { b'-' };
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

    if options.column && is_match {
        match first_scalar_column(line) {
            Some(column) => write!(writer, "{column}")?,
            None => writer.write_all(b"?")?,
        }
        writer.write_all(&[delimiter])?;
        has_prefix = true;
    }

    if has_prefix {
        writer.write_all(b" ")?;
    }
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

pub(super) fn line_content(bytes: &[u8]) -> &[u8] {
    let without_lf = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    without_lf.strip_suffix(b"\r").unwrap_or(without_lf)
}

pub(super) fn first_scalar_column(line: &SearchLine) -> Option<usize> {
    let start = line.matches.first()?.span.start;
    let prefix = line.bytes.get(..start)?;
    Some(std::str::from_utf8(prefix).ok()?.chars().count() + 1)
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
