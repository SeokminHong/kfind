use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::protocol::{ColumnRange, MatchLine, decode_match_line};

const HIGHLIGHT_START: &str = "\x1b[1;31m";
const HIGHLIGHT_END: &str = "\x1b[0m";
const ELLIPSIS: &str = "…";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RowKey {
    pub source: usize,
    pub target: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Layout {
    pub rows: Vec<RowKey>,
    pub truncated: bool,
    pub width: usize,
}

impl Layout {
    pub(super) fn locate(&self, key: RowKey) -> usize {
        self.rows
            .iter()
            .position(|row| *row == key)
            .or_else(|| self.rows.iter().position(|row| row.source == key.source))
            .unwrap_or(0)
    }

    #[cfg(feature = "pager-memory-benchmark")]
    pub(super) fn index_stats(&self) -> (usize, usize, usize) {
        (
            self.rows.len(),
            self.rows.capacity(),
            std::mem::size_of::<RowKey>(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceIndex {
    offset: u64,
    length: usize,
}

enum SourceLine {
    Plain(String),
    Match(MatchLine),
}

pub(super) struct Document {
    file: File,
    lines: Vec<SourceIndex>,
    indexed_end: u64,
}

impl Document {
    pub(super) fn open(mut file: File) -> io::Result<Self> {
        file.seek(SeekFrom::Start(0))?;
        let mut document = Self {
            file,
            lines: Vec::new(),
            indexed_end: 0,
        };
        document.refresh()?;
        Ok(document)
    }

    pub(super) fn source_count(&self) -> usize {
        self.lines.len()
    }

    #[cfg(feature = "pager-memory-benchmark")]
    pub(super) fn index_stats(&self) -> (usize, usize, usize) {
        (
            self.lines.len(),
            self.lines.capacity(),
            std::mem::size_of::<SourceIndex>(),
        )
    }

    pub(super) fn refresh(&mut self) -> io::Result<std::ops::Range<usize>> {
        let first_new = self.lines.len();
        let mut reader = BufReader::new(self.file.try_clone()?);
        reader.seek(SeekFrom::Start(self.indexed_end))?;
        let mut offset = self.indexed_end;
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            let length = reader.read_until(b'\n', &mut bytes)?;
            if length == 0 || bytes.last() != Some(&b'\n') {
                break;
            }
            self.lines.push(SourceIndex { offset, length });
            offset = offset.saturating_add(length as u64);
        }
        self.indexed_end = offset;
        Ok(first_new..self.lines.len())
    }

    pub(super) fn layout(&mut self, width: usize) -> io::Result<Layout> {
        let mut rows = Vec::new();
        let mut truncated = false;
        for source in 0..self.lines.len() {
            match self.read_source(source)? {
                SourceLine::Plain(text) => {
                    truncated |= display_width(&text) > width;
                    rows.push(RowKey {
                        source,
                        target: None,
                    });
                }
                SourceLine::Match(line) => {
                    let line_layout = match_layout(&line, source, width);
                    truncated |= line_layout.truncated;
                    rows.extend(line_layout.rows);
                }
            }
        }
        Ok(Layout {
            rows,
            truncated,
            width,
        })
    }

    pub(super) fn extend_layout(
        &mut self,
        layout: &mut Layout,
        sources: std::ops::Range<usize>,
    ) -> io::Result<()> {
        for source in sources {
            match self.read_source(source)? {
                SourceLine::Plain(text) => {
                    layout.truncated |= display_width(&text) > layout.width;
                    layout.rows.push(RowKey {
                        source,
                        target: None,
                    });
                }
                SourceLine::Match(line) => {
                    let line_layout = match_layout(&line, source, layout.width);
                    layout.truncated |= line_layout.truncated;
                    layout.rows.extend(line_layout.rows);
                }
            }
        }
        Ok(())
    }

    pub(super) fn render_row(&mut self, key: RowKey, width: usize) -> io::Result<String> {
        match self.read_source(key.source)? {
            SourceLine::Plain(text) => Ok(truncate_end(&text, width).0),
            SourceLine::Match(line) => render_match_row(&line, key.target, width),
        }
    }

    pub(super) fn full_row(&mut self, source: usize) -> io::Result<String> {
        match self.read_source(source)? {
            SourceLine::Plain(text) => Ok(text),
            SourceLine::Match(line) => {
                let mut rendered = line.prefixes[0].clone();
                let highlights = if line.color {
                    all_tokens(&line)
                } else {
                    Vec::new()
                };
                render_slice(
                    &mut rendered,
                    &line.content,
                    0,
                    display_width(&line.content),
                    &highlights,
                );
                Ok(rendered)
            }
        }
    }

    fn read_source(&mut self, source: usize) -> io::Result<SourceLine> {
        let index = *self
            .lines
            .get(source)
            .ok_or_else(|| invalid_document("source line is outside the document"))?;
        self.file.seek(SeekFrom::Start(index.offset))?;
        let mut bytes = vec![0; index.length];
        self.file.read_exact(&mut bytes)?;
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        if let Some(line) = decode_match_line(&bytes)? {
            Ok(SourceLine::Match(line))
        } else {
            String::from_utf8(bytes)
                .map(SourceLine::Plain)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.utf8_error()))
        }
    }
}

fn match_layout(line: &MatchLine, source: usize, width: usize) -> Layout {
    let (_, prefix_width, prefix_truncated) = fit_prefix(&line.prefixes[0], width);
    let content_width = width.saturating_sub(prefix_width);
    if display_width(&line.content) <= content_width {
        return Layout {
            rows: vec![RowKey {
                source,
                target: None,
            }],
            truncated: prefix_truncated,
            width,
        };
    }

    Layout {
        rows: (0..line.matches.len())
            .map(|target| RowKey {
                source,
                target: Some(target),
            })
            .collect(),
        truncated: true,
        width,
    }
}

fn render_match_row(line: &MatchLine, target: Option<usize>, width: usize) -> io::Result<String> {
    let total = display_width(&line.content);
    if let Some(target) = target {
        let matched = line
            .matches
            .get(target)
            .ok_or_else(|| invalid_document("target match is outside the source line"))?;
        let prefix = line
            .prefixes
            .get(target)
            .ok_or_else(|| invalid_document("target prefix is outside the source line"))?;
        let (mut text, prefix_width, _) = fit_prefix(prefix, width);
        let window = balanced_window(total, matched.span, width.saturating_sub(prefix_width));
        if window.left_ellipsis {
            text.push_str(ELLIPSIS);
        }
        let highlights = if line.color {
            matched.tokens.clone()
        } else {
            Vec::new()
        };
        render_slice(
            &mut text,
            &line.content,
            window.start,
            window.end,
            &highlights,
        );
        if window.right_ellipsis {
            text.push_str(ELLIPSIS);
        }
        return Ok(text);
    }

    let (mut text, _, _) = fit_prefix(&line.prefixes[0], width);
    let highlights = if line.color {
        all_tokens(line)
    } else {
        Vec::new()
    };
    render_slice(&mut text, &line.content, 0, total, &highlights);
    Ok(text)
}

fn all_tokens(line: &MatchLine) -> Vec<ColumnRange> {
    line.matches
        .iter()
        .flat_map(|matched| matched.tokens.iter().copied())
        .collect()
}

fn fit_prefix(prefix: &str, terminal_width: usize) -> (String, usize, bool) {
    let prefix_width = display_width(prefix);
    let budget = terminal_width.saturating_mul(2) / 5;
    if prefix_width <= budget {
        return (prefix.to_owned(), prefix_width, false);
    }
    if budget == 0 {
        return (String::new(), 0, !prefix.is_empty());
    }
    if budget == 1 {
        return (ELLIPSIS.to_owned(), 1, true);
    }
    let mut fitted = String::from(ELLIPSIS);
    fitted.push_str(&slice_columns_from_end(prefix, budget - 1));
    let fitted_width = display_width(&fitted);
    (fitted, fitted_width, true)
}

pub(super) fn truncate_end(text: &str, width: usize) -> (String, bool) {
    if display_width(text) <= width {
        return (text.to_owned(), false);
    }
    if width == 0 {
        return (String::new(), true);
    }
    if width == 1 {
        return (ELLIPSIS.to_owned(), true);
    }
    let mut truncated = slice_columns(text, 0, width - 1);
    truncated.push_str(ELLIPSIS);
    (truncated, true)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Window {
    start: usize,
    end: usize,
    left_ellipsis: bool,
    right_ellipsis: bool,
}

fn balanced_window(total: usize, target: ColumnRange, width: usize) -> Window {
    if total <= width {
        return Window {
            start: 0,
            end: total,
            left_ellipsis: false,
            right_ellipsis: false,
        };
    }
    if width == 0 {
        return Window {
            start: target.start.min(total),
            end: target.start.min(total),
            left_ellipsis: target.start > 0,
            right_ellipsis: target.end < total,
        };
    }

    let target = ColumnRange::new(target.start.min(total), target.end.min(total));
    let mut left_ellipsis = target.start > 0;
    let mut right_ellipsis = target.end < total;
    let mut start = 0;
    let mut end = 0;

    for _ in 0..4 {
        let indicators = usize::from(left_ellipsis) + usize::from(right_ellipsis);
        let inner = width.saturating_sub(indicators);
        if inner == 0 {
            start = target.start;
            end = start;
        } else {
            let target_width = target.end.saturating_sub(target.start);
            if target_width >= inner {
                let center = target.start.saturating_add(target_width / 2);
                start = center.saturating_sub(inner / 2).min(total - inner);
            } else {
                let free_context = inner - target_width;
                let before = target.start;
                let after = total.saturating_sub(target.end);
                let context = before.saturating_add(after);
                let mut desired_before = free_context
                    .saturating_mul(before)
                    .saturating_add(context / 2)
                    .checked_div(context)
                    .unwrap_or(free_context / 2);
                if before > 0 && after > 0 && free_context >= 2 {
                    let minimum = free_context.div_ceil(5);
                    desired_before = desired_before.clamp(minimum, free_context - minimum);
                }
                start = target.start.saturating_sub(desired_before);
                if target.end > start.saturating_add(inner) {
                    start = target.end - inner;
                }
                start = start.min(total - inner);
            }
            end = start.saturating_add(inner).min(total);
            if end - start < inner {
                start = end.saturating_sub(inner);
            }
        }
        let new_left = start > 0;
        let new_right = end < total;
        if new_left == left_ellipsis && new_right == right_ellipsis {
            break;
        }
        left_ellipsis = new_left;
        right_ellipsis = new_right;
    }

    Window {
        start,
        end,
        left_ellipsis,
        right_ellipsis,
    }
}

fn render_slice(
    output: &mut String,
    text: &str,
    start: usize,
    end: usize,
    highlights: &[ColumnRange],
) {
    let mut column = 0_usize;
    let mut highlighting = false;
    for character in text.chars() {
        let width = UnicodeWidthChar::width(character).unwrap_or(0);
        let next = column.saturating_add(width);
        let included = if width == 0 {
            column >= start && column <= end
        } else {
            column >= start && next <= end
        };
        if included {
            let highlighted = highlights
                .iter()
                .any(|range| range.start < next && column < range.end);
            if highlighted != highlighting {
                output.push_str(if highlighted {
                    HIGHLIGHT_START
                } else {
                    HIGHLIGHT_END
                });
                highlighting = highlighted;
            }
            output.push(character);
        }
        column = next;
        if column > end {
            break;
        }
    }
    if highlighting {
        output.push_str(HIGHLIGHT_END);
    }
}

fn slice_columns(text: &str, start: usize, end: usize) -> String {
    let mut sliced = String::new();
    render_slice(&mut sliced, text, start, end, &[]);
    sliced
}

fn slice_columns_from_end(text: &str, width: usize) -> String {
    let total = display_width(text);
    slice_columns(text, total.saturating_sub(width), total)
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn invalid_document(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, Write};

    use tempfile::{NamedTempFile, tempfile};

    use super::super::protocol::{PagerMatch, write_match_line};
    use super::*;

    fn match_line(content: &str, spans: &[(usize, usize)]) -> MatchLine {
        MatchLine {
            content: content.to_owned(),
            prefixes: spans
                .iter()
                .enumerate()
                .map(|(index, _)| format!("sample.txt:1:{}: ", index + 1))
                .collect(),
            matches: spans
                .iter()
                .map(|&(start, end)| PagerMatch {
                    span: ColumnRange::new(start, end),
                    tokens: vec![ColumnRange::new(start, end)],
                })
                .collect(),
            color: true,
        }
    }

    fn document(bytes: &[u8]) -> Document {
        let mut file = tempfile().unwrap();
        file.write_all(bytes).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        Document::open(file).unwrap()
    }

    fn visible(text: &str) -> String {
        text.replace(HIGHLIGHT_START, "").replace(HIGHLIGHT_END, "")
    }

    #[test]
    fn document_indexes_plain_and_structured_lines() {
        let expected = match_line("before MATCH after", &[(7, 12)]);
        let mut bytes = b"query: test\n".to_vec();
        write_match_line(&mut bytes, &expected).unwrap();
        let mut document = document(&bytes);

        assert_eq!(document.source_count(), 2);
        assert_eq!(document.full_row(0).unwrap(), "query: test");
        assert_eq!(
            visible(&document.full_row(1).unwrap()),
            "sample.txt:1:1: before MATCH after"
        );
    }

    #[test]
    fn document_refresh_indexes_only_complete_appended_lines() {
        let file = NamedTempFile::new().unwrap();
        let mut writer = file.reopen().unwrap();
        let mut document = Document::open(file.reopen().unwrap()).unwrap();

        writer.write_all(b"first").unwrap();
        writer.flush().unwrap();
        assert_eq!(document.refresh().unwrap(), 0..0);

        writer.write_all(b" line\nsecond line\n").unwrap();
        writer.flush().unwrap();
        let appended = document.refresh().unwrap();

        assert_eq!(appended, 0..2);
        assert_eq!(document.full_row(0).unwrap(), "first line");
        assert_eq!(document.full_row(1).unwrap(), "second line");
        assert_eq!(document.refresh().unwrap(), 2..2);
    }

    #[test]
    fn a_truncated_source_line_expands_to_one_row_per_match() {
        let line = match_line(
            "aaaaaaaaaaFIRSTbbbbbbbbbbbbbbbbbbbbSECONDcccccccccc",
            &[(10, 15), (35, 41)],
        );
        let layout = match_layout(&line, 0, 28);

        assert!(layout.truncated);
        assert_eq!(layout.rows.len(), 2);
        assert!(
            visible(&render_match_row(&line, layout.rows[0].target, 28).unwrap()).contains("FIRST")
        );
        assert!(
            visible(&render_match_row(&line, layout.rows[1].target, 28).unwrap())
                .contains("SECOND")
        );
        assert_eq!(layout.rows[0].target, Some(0));
        assert_eq!(layout.rows[1].target, Some(1));
    }

    #[test]
    fn resize_collapses_and_expands_match_rows() {
        let line = match_line(
            "aaaaaaaaaaFIRSTbbbbbbbbbbbbbbbbbbbbSECONDcccccccccc",
            &[(10, 15), (35, 41)],
        );
        let mut bytes = Vec::new();
        write_match_line(&mut bytes, &line).unwrap();
        let mut document = document(&bytes);

        let narrow = document.layout(28).unwrap();
        let wide = document.layout(100).unwrap();

        assert_eq!(narrow.rows.len(), 2);
        assert_eq!(wide.rows.len(), 1);
        assert!(!wide.truncated);
        assert_eq!(wide.locate(narrow.rows[1]), 0);
    }

    #[test]
    fn target_position_uses_ratio_with_a_twenty_percent_margin() {
        let content = format!("{}MATCH{}", "a".repeat(10), "b".repeat(90));
        let line = match_line(&content, &[(10, 15)]);
        let layout = match_layout(&line, 0, 40);
        let text = visible(&render_match_row(&line, layout.rows[0].target, 40).unwrap());
        let content = text.split_once(": ").unwrap().1;
        let target_column = display_width(content.split("MATCH").next().unwrap());

        assert!(content.starts_with("…"));
        assert!(content.ends_with("…"));
        assert!(target_column >= 4, "target column was {target_column}");
        assert!(target_column <= 19, "target column was {target_column}");
    }

    #[test]
    fn long_prefix_is_left_truncated_before_match_content() {
        let mut line = match_line("left MATCH right and more text", &[(5, 10)]);
        line.prefixes[0] = "/a/very/long/path/to/sample.txt:99:12: ".to_owned();
        let layout = match_layout(&line, 0, 30);
        let text = visible(&render_match_row(&line, layout.rows[0].target, 30).unwrap());

        assert!(text.starts_with('…'));
        assert!(text.contains("MATCH"));
        assert!(display_width(&text) <= 30);
    }

    #[test]
    fn plain_lines_keep_the_start_and_truncate_the_end() {
        let mut document = document(b"0123456789\n");
        let layout = document.layout(6).unwrap();

        assert_eq!(document.render_row(layout.rows[0], 6).unwrap(), "01234…");
    }

    #[test]
    fn column_slicing_counts_wide_korean_characters() {
        let line = match_line("앞앞MATCH뒤뒤뒤뒤뒤", &[(4, 9)]);
        let layout = match_layout(&line, 0, 15);
        let text = visible(&render_match_row(&line, layout.rows[0].target, 15).unwrap());

        assert!(text.contains("MATCH"));
        assert!(display_width(&text) <= 15);
    }
}
