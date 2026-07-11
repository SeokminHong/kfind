use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use grep_searcher::{
    BinaryDetection, Encoding, Searcher, SearcherBuilder, Sink, SinkContext, SinkContextKind,
    SinkError, SinkMatch,
};
use kfind_matcher::MorphMatcher;
use kfind_query::PhraseMatch;

const MAX_MATCHES_PER_LINE: usize = 65_536;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputEncoding {
    #[default]
    Auto,
    Utf8,
    Utf16Le,
    Utf16Be,
    EucKr,
}

impl InputEncoding {
    fn label(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Utf8 => Some("utf-8"),
            Self::Utf16Le => Some("utf-16le"),
            Self::Utf16Be => Some("utf-16be"),
            Self::EucKr => Some("euc-kr"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputOptions {
    pub encoding: InputEncoding,
    pub before_context: usize,
    pub after_context: usize,
    pub capture_records: bool,
    pub stop_after_first_match: bool,
}

impl Default for InputOptions {
    fn default() -> Self {
        Self {
            encoding: InputEncoding::Auto,
            before_context: 0,
            after_context: 0,
            capture_records: true,
            stop_after_first_match: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchLineKind {
    Match,
    BeforeContext,
    AfterContext,
    OtherContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchLine {
    pub kind: SearchLineKind,
    pub line_number: Option<u64>,
    pub absolute_byte_offset: u64,
    pub bytes: Vec<u8>,
    pub matches: Vec<PhraseMatch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchRecord {
    Line(SearchLine),
    ContextBreak,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSearchResult {
    pub path: PathBuf,
    pub records: Vec<SearchRecord>,
    pub matching_lines: u64,
    pub matched_spans: Option<u64>,
    pub binary_byte_offset: Option<u64>,
}

impl FileSearchResult {
    #[must_use]
    pub const fn has_match(&self) -> bool {
        self.matching_lines > 0
    }
}

/// Reusable, single-threaded file search state.
///
/// Create one value per traversal worker so that `grep-searcher` can reuse its
/// scratch buffers without sharing mutable state between workers.
pub struct InputSearcher {
    searcher: Searcher,
    options: InputOptions,
}

impl InputSearcher {
    pub fn new(options: InputOptions) -> Result<Self, InputSearchError> {
        Ok(Self {
            searcher: build_searcher(options)?,
            options,
        })
    }

    pub fn search_path(
        &mut self,
        matcher: &MorphMatcher,
        path: &Path,
    ) -> Result<FileSearchResult, InputSearchError> {
        let mut records = Vec::new();
        let mut result = self.search_path_stream(matcher, path, |record| {
            records.push(record);
            true
        })?;
        result.records = records;
        Ok(result)
    }

    pub fn search_reader(
        &mut self,
        matcher: &MorphMatcher,
        display_path: PathBuf,
        reader: impl Read,
    ) -> Result<FileSearchResult, InputSearchError> {
        let mut records = Vec::new();
        let mut result = self.search_reader_stream(matcher, display_path, reader, |record| {
            records.push(record);
            true
        })?;
        result.records = records;
        Ok(result)
    }

    pub(crate) fn search_path_stream<F>(
        &mut self,
        matcher: &MorphMatcher,
        path: &Path,
        emit: F,
    ) -> Result<FileSearchResult, InputSearchError>
    where
        F: FnMut(SearchRecord) -> bool,
    {
        let mut sink = MatchSink::new(
            path.to_path_buf(),
            matcher,
            self.options.capture_records,
            emit,
        );
        self.searcher.search_path(matcher, path, &mut sink)?;
        Ok(sink.finish())
    }

    pub(crate) fn search_reader_stream<F>(
        &mut self,
        matcher: &MorphMatcher,
        display_path: PathBuf,
        reader: impl Read,
        emit: F,
    ) -> Result<FileSearchResult, InputSearchError>
    where
        F: FnMut(SearchRecord) -> bool,
    {
        let mut sink = MatchSink::new(display_path, matcher, self.options.capture_records, emit);
        self.searcher.search_reader(matcher, reader, &mut sink)?;
        Ok(sink.finish())
    }
}

pub fn search_path(
    matcher: &MorphMatcher,
    path: &Path,
    options: InputOptions,
) -> Result<FileSearchResult, InputSearchError> {
    InputSearcher::new(options)?.search_path(matcher, path)
}

pub fn search_reader(
    matcher: &MorphMatcher,
    display_path: PathBuf,
    reader: impl Read,
    options: InputOptions,
) -> Result<FileSearchResult, InputSearchError> {
    InputSearcher::new(options)?.search_reader(matcher, display_path, reader)
}

fn build_searcher(options: InputOptions) -> Result<Searcher, InputSearchError> {
    let encoding = options
        .encoding
        .label()
        .map(Encoding::new)
        .transpose()
        .map_err(|error| InputSearchError::Encoding(error.to_string()))?;
    let mut builder = SearcherBuilder::new();
    builder
        .line_number(true)
        .multi_line(true)
        .before_context(options.before_context)
        .after_context(options.after_context)
        .binary_detection(BinaryDetection::quit(b'\0'))
        .encoding(encoding)
        .bom_sniffing(options.encoding == InputEncoding::Auto)
        .max_matches(options.stop_after_first_match.then_some(1));
    Ok(builder.build())
}

struct MatchSink<'a, F> {
    result: FileSearchResult,
    matcher: &'a MorphMatcher,
    capture_records: bool,
    emit: F,
}

impl<'a, F> MatchSink<'a, F>
where
    F: FnMut(SearchRecord) -> bool,
{
    fn new(path: PathBuf, matcher: &'a MorphMatcher, capture_records: bool, emit: F) -> Self {
        Self {
            result: FileSearchResult {
                path,
                records: Vec::new(),
                matching_lines: 0,
                matched_spans: capture_records.then_some(0),
                binary_byte_offset: None,
            },
            matcher,
            capture_records,
            emit,
        }
    }

    fn finish(self) -> FileSearchResult {
        self.result
    }

    fn emit_context(&mut self, context: &SinkContext<'_>) -> bool {
        if !self.capture_records {
            return true;
        }
        let kind = match context.kind() {
            SinkContextKind::Before => SearchLineKind::BeforeContext,
            SinkContextKind::After => SearchLineKind::AfterContext,
            SinkContextKind::Other => SearchLineKind::OtherContext,
        };
        (self.emit)(SearchRecord::Line(SearchLine {
            kind,
            line_number: context.line_number(),
            absolute_byte_offset: context.absolute_byte_offset(),
            bytes: context.bytes().to_vec(),
            matches: Vec::new(),
        }))
    }
}

impl<F> Sink for MatchSink<'_, F>
where
    F: FnMut(SearchRecord) -> bool,
{
    type Error = InputSearchError;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        matched: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        self.result.matching_lines += 1;
        let matches = if self.capture_records {
            collect_line_matches(self.matcher, matched.bytes(), MAX_MATCHES_PER_LINE)?
        } else {
            Vec::new()
        };
        if let Some(count) = &mut self.result.matched_spans {
            *count += matches.len() as u64;
        }
        if self.capture_records {
            return Ok((self.emit)(SearchRecord::Line(SearchLine {
                kind: SearchLineKind::Match,
                line_number: matched.line_number(),
                absolute_byte_offset: matched.absolute_byte_offset(),
                bytes: matched.bytes().to_vec(),
                matches,
            })));
        }
        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        Ok(self.emit_context(context))
    }

    fn context_break(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        if self.capture_records {
            return Ok((self.emit)(SearchRecord::ContextBreak));
        }
        Ok(true)
    }

    fn binary_data(
        &mut self,
        _searcher: &Searcher,
        binary_byte_offset: u64,
    ) -> Result<bool, Self::Error> {
        self.result
            .binary_byte_offset
            .get_or_insert(binary_byte_offset);
        Ok(false)
    }
}

fn collect_line_matches(
    matcher: &MorphMatcher,
    bytes: &[u8],
    limit: usize,
) -> Result<Vec<PhraseMatch>, InputSearchError> {
    let mut matches = Vec::new();
    let mut at = 0;
    while let Some(matched) = matcher.find_at_with_meta(bytes, at) {
        if matches.len() == limit {
            return Err(InputSearchError::MatchLimitExceeded { limit });
        }
        at = matched.span.end;
        matches.push(matched);
    }
    Ok(matches)
}

#[derive(Debug)]
pub enum InputSearchError {
    Encoding(String),
    MatchLimitExceeded { limit: usize },
    Io(io::Error),
}

impl Display for InputSearchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(error) => write!(formatter, "invalid input encoding: {error}"),
            Self::MatchLimitExceeded { limit } => {
                write!(formatter, "matches per line exceed limit {limit}")
            }
            Self::Io(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for InputSearchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Encoding(_) | Self::MatchLimitExceeded { .. } => None,
        }
    }
}

impl SinkError for InputSearchError {
    fn error_message<T: Display>(message: T) -> Self {
        Self::Io(io::Error::other(message.to_string()))
    }

    fn error_io(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<io::Error> for InputSearchError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};

    use super::*;

    fn matcher(query: &str) -> MorphMatcher {
        let lexicons = Arc::new(Lexicons::embedded().unwrap());
        let analyzer = LexiconQueryAnalyzer::new(lexicons);
        let plan = compile_query(query, &CompileOptions::default(), &analyzer).unwrap();
        MorphMatcher::new(Arc::new(plan)).unwrap()
    }

    #[test]
    fn reader_reports_lines_and_morphology_metadata() {
        let result = search_reader(
            &matcher("걷다"),
            PathBuf::from("<memory>"),
            "길을 걸어 갔다.\n멈췄다.\n".as_bytes(),
            InputOptions::default(),
        )
        .unwrap();

        assert_eq!(result.matching_lines, 1);
        assert_eq!(result.matched_spans, Some(1));
        let SearchRecord::Line(line) = &result.records[0] else {
            panic!("expected matching line")
        };
        assert_eq!(line.line_number, Some(1));
        assert_eq!(&line.bytes[line.matches[0].span.clone()], "걸어".as_bytes());
    }

    #[test]
    fn auto_encoding_detects_utf16_bom() {
        let text = "길을 걸어 갔다.\n";
        let mut encoded = vec![0xff, 0xfe];
        encoded.extend(text.encode_utf16().flat_map(u16::to_le_bytes));

        let result = search_reader(
            &matcher("걷다"),
            PathBuf::from("utf16.txt"),
            encoded.as_slice(),
            InputOptions::default(),
        )
        .unwrap();
        assert_eq!(result.matching_lines, 1);
    }

    #[test]
    fn raw_anchor_candidates_are_verified_within_their_line() {
        let result = search_reader(
            &matcher("권한"),
            PathBuf::from("candidates.txt"),
            "사용자권한\n권한은 있다.\n".as_bytes(),
            InputOptions::default(),
        )
        .unwrap();

        assert_eq!(result.matching_lines, 1);
        let SearchRecord::Line(line) = &result.records[0] else {
            panic!("expected matching line")
        };
        assert_eq!(line.line_number, Some(2));
        assert_eq!(
            &line.bytes[line.matches[0].span.clone()],
            "권한은".as_bytes()
        );
    }

    #[test]
    fn quoted_literal_can_match_across_lines() {
        let result = search_reader(
            &matcher("\"권한\n검증\""),
            PathBuf::from("multiline.txt"),
            "앞\n권한\n검증\n뒤\n".as_bytes(),
            InputOptions::default(),
        )
        .unwrap();

        assert_eq!(result.matching_lines, 1);
        let SearchRecord::Line(line) = &result.records[0] else {
            panic!("expected multiline match")
        };
        assert_eq!(line.line_number, Some(2));
        assert_eq!(
            &line.bytes[line.matches[0].span.clone()],
            "권한\n검증".as_bytes()
        );
    }

    #[test]
    fn binary_input_stops_at_nul() {
        let result = search_reader(
            &matcher("걷다"),
            PathBuf::from("binary"),
            b"\0\x01\x02\xea\xb1\xb8\xec\x96\xb4".as_slice(),
            InputOptions::default(),
        )
        .unwrap();

        assert!(!result.has_match());
        assert_eq!(result.binary_byte_offset, Some(0));
    }

    #[test]
    fn context_is_reported_without_counting_as_a_match() {
        let result = search_reader(
            &matcher("걷다"),
            PathBuf::from("context.txt"),
            "앞줄\n길을 걸어 갔다.\n뒷줄\n".as_bytes(),
            InputOptions {
                before_context: 1,
                after_context: 1,
                ..InputOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.matching_lines, 1);
        assert_eq!(result.records.len(), 3);
        assert!(matches!(
            &result.records[0],
            SearchRecord::Line(SearchLine {
                kind: SearchLineKind::BeforeContext,
                ..
            })
        ));
        assert!(matches!(
            &result.records[2],
            SearchRecord::Line(SearchLine {
                kind: SearchLineKind::AfterContext,
                ..
            })
        ));
    }

    #[test]
    fn line_match_metadata_stops_at_the_configured_limit() {
        let matcher = matcher("권한");
        let error = collect_line_matches(&matcher, "권한 권한 권한".as_bytes(), 2).unwrap_err();

        assert!(matches!(
            error,
            InputSearchError::MatchLimitExceeded { limit: 2 }
        ));
    }
}
