use std::error::Error;
use std::fmt;

use kfind_morph::CoarsePos;
use kfind_morph::GenerateError;

use crate::AnalyzeError;

/// Half-open UTF-8 byte offsets into the original query source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A query lexer or validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    pub kind: QueryErrorKind,
    pub span: SourceSpan,
}

impl QueryError {
    pub(crate) const fn new(kind: QueryErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at bytes {}..{}",
            self.kind, self.span.start, self.span.end
        )
    }
}

impl Error for QueryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryErrorKind {
    EmptyQuery,
    EmptyAtom,
    DanglingEscape,
    UnterminatedQuote {
        quote: char,
    },
    QueryTooLong {
        actual: usize,
        limit: usize,
    },
    TooManyAtoms {
        actual: usize,
        limit: usize,
    },
    ConflictingPos {
        global: CoarsePos,
        tagged: CoarsePos,
    },
}

impl fmt::Display for QueryErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyQuery => formatter.write_str("query must contain at least one atom"),
            Self::EmptyAtom => formatter.write_str("query atom must not be empty"),
            Self::DanglingEscape => formatter.write_str("backslash must escape another character"),
            Self::UnterminatedQuote { quote } => {
                write!(formatter, "unterminated {quote} quote")
            }
            Self::QueryTooLong { actual, limit } => {
                write!(
                    formatter,
                    "query has {actual} Unicode scalars; limit is {limit}"
                )
            }
            Self::TooManyAtoms { actual, limit } => {
                write!(formatter, "query has {actual} atoms; limit is {limit}")
            }
            Self::ConflictingPos { global, tagged } => write!(
                formatter,
                "global part of speech {global:?} conflicts with atom tag {tagged:?}"
            ),
        }
    }
}

#[derive(Debug)]
pub struct CompileError {
    pub atom_index: Option<usize>,
    pub kind: Box<CompileErrorKind>,
}

impl CompileError {
    pub(crate) fn new(atom_index: Option<usize>, kind: CompileErrorKind) -> Self {
        Self {
            atom_index,
            kind: Box::new(kind),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = self.atom_index {
            write!(formatter, "atom[{index}]: ")?;
        }
        self.kind.fmt(formatter)
    }
}

impl Error for CompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.kind.as_ref() {
            CompileErrorKind::Query(error) => Some(error),
            CompileErrorKind::Analyze(error) => Some(error),
            CompileErrorKind::Generate(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum CompileErrorKind {
    Query(QueryError),
    Analyze(AnalyzeError),
    Generate(GenerateError),
    TooManyAnalyses { actual: usize, limit: usize },
    TooManyBranches { actual: usize, limit: usize },
    MatcherMemoryExceeded { estimated: usize, limit: usize },
    ContinuationDepthExceeded { actual: usize, limit: usize },
    NoSearchableBranches,
    InvalidCoreMapping,
}

impl fmt::Display for CompileErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query(error) => error.fmt(formatter),
            Self::Analyze(error) => error.fmt(formatter),
            Self::Generate(error) => error.fmt(formatter),
            Self::TooManyAnalyses { actual, limit } => {
                write!(formatter, "analysis count {actual} exceeds limit {limit}")
            }
            Self::TooManyBranches { actual, limit } => {
                write!(formatter, "branch count {actual} exceeds limit {limit}")
            }
            Self::MatcherMemoryExceeded { estimated, limit } => write!(
                formatter,
                "estimated matcher memory {estimated} bytes exceeds limit {limit}"
            ),
            Self::ContinuationDepthExceeded { actual, limit } => write!(
                formatter,
                "continuation depth {actual} exceeds limit {limit}"
            ),
            Self::NoSearchableBranches => formatter.write_str("query has no searchable branches"),
            Self::InvalidCoreMapping => {
                formatter.write_str("morphology core mapping is not on a UTF-8 boundary")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhraseJoinError {
    NoAtoms,
    InvalidSpan {
        atom_index: usize,
        start: usize,
        end: usize,
        text_len: usize,
    },
}

impl fmt::Display for PhraseJoinError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAtoms => formatter.write_str("phrase join requires at least one atom"),
            Self::InvalidSpan {
                atom_index,
                start,
                end,
                text_len,
            } => write!(
                formatter,
                "atom[{atom_index}] span {start}..{end} is invalid for {text_len} byte text"
            ),
        }
    }
}

impl Error for PhraseJoinError {}
