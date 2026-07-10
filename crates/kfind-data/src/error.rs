use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub source: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl SourceLocation {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            line: None,
            column: None,
        }
    }

    pub fn at_line(source: impl Into<String>, line: usize) -> Self {
        Self {
            source: source.into(),
            line: Some(line),
            column: None,
        }
    }
}

impl Display for SourceLocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.source)?;
        if let Some(line) = self.line {
            write!(formatter, ":{line}")?;
        }
        if let Some(column) = self.column {
            write!(formatter, ":{column}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataErrorKind {
    Io(String),
    InvalidHeader {
        expected: String,
        actual: String,
    },
    InvalidFieldCount {
        expected: usize,
        actual: usize,
    },
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
    NonNfc {
        field: String,
        value: String,
    },
    InvalidPredicateLemma(String),
    DuplicateRuleId(String),
    UnknownRuleId(String),
    OverrideConflict {
        lemma: String,
        rule_id: String,
        first: String,
        second: String,
    },
    Toml(String),
    Binary(String),
    MissingFixtureCoverage(String),
}

impl Display for DataErrorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "I/O 오류: {message}"),
            Self::InvalidHeader { expected, actual } => {
                write!(
                    formatter,
                    "TSV 헤더가 올바르지 않습니다: expected `{expected}`, got `{actual}`"
                )
            }
            Self::InvalidFieldCount { expected, actual } => {
                write!(
                    formatter,
                    "필드 수가 올바르지 않습니다: expected {expected}, got {actual}"
                )
            }
            Self::InvalidValue {
                field,
                value,
                reason,
            } => write!(
                formatter,
                "`{field}` 값 `{value}`가 올바르지 않습니다: {reason}"
            ),
            Self::NonNfc { field, value } => {
                write!(formatter, "`{field}` 값 `{value}`가 NFC가 아닙니다")
            }
            Self::InvalidPredicateLemma(lemma) => {
                write!(
                    formatter,
                    "용언 표제어 `{lemma}`는 비어 있지 않은 `다` 기본형이어야 합니다"
                )
            }
            Self::DuplicateRuleId(id) => write!(formatter, "규칙 ID `{id}`가 중복되었습니다"),
            Self::UnknownRuleId(id) => {
                write!(formatter, "존재하지 않는 규칙 ID `{id}`를 참조합니다")
            }
            Self::OverrideConflict {
                lemma,
                rule_id,
                first,
                second,
            } => write!(
                formatter,
                "표제어 `{lemma}`의 `{rule_id}` override가 `{first}`와 `{second}`로 충돌합니다"
            ),
            Self::Toml(message) => write!(formatter, "TOML 스키마 오류: {message}"),
            Self::Binary(message) => write!(formatter, "POS lexicon binary 오류: {message}"),
            Self::MissingFixtureCoverage(feature) => {
                write!(
                    formatter,
                    "사전 class `{feature}`를 사용하는 morphology fixture가 없습니다"
                )
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataError {
    pub location: SourceLocation,
    pub kind: Box<DataErrorKind>,
}

impl DataError {
    pub(crate) fn new(location: SourceLocation, kind: DataErrorKind) -> Self {
        Self {
            location,
            kind: Box::new(kind),
        }
    }

    pub(crate) fn line(source: &str, line: usize, kind: DataErrorKind) -> Self {
        Self::new(SourceLocation::at_line(source, line), kind)
    }
}

impl Display for DataError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.location, self.kind)
    }
}

impl Error for DataError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataWarning {
    DuplicateRow {
        location: SourceLocation,
        first_line: usize,
    },
}
