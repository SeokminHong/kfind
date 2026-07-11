use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use kfind_matcher::{AnchorBuildError, MorphMatcherBuildError};
use kfind_morph::{CoarsePos, GenerateError};
use kfind_query::{
    AnalyzeError, CompileError, CompileErrorKind, CompileOptionError, ExpandMode, QueryError,
    QueryErrorKind,
};
use kfind_search::{InputSearchError, SearchRunError, WalkConfigError};

use crate::{CliError, Language, OutputError};

mod data;

pub struct LocalizedCliError<'a> {
    error: &'a CliError,
    language: Language,
}

impl<'a> LocalizedCliError<'a> {
    pub(crate) const fn new(error: &'a CliError, language: Language) -> Self {
        Self { error, language }
    }
}

impl Display for LocalizedCliError<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.error {
            CliError::Options(error) => write_compile_option(error, self.language, formatter),
            CliError::Data(error) => data::write_error(error, self.language, formatter),
            CliError::Compile(error) => write_compile_error(error, self.language, formatter),
            CliError::Matcher(error) => write_matcher_error(error, self.language, formatter),
            CliError::Search(error) => write_search_error(error, self.language, formatter),
            CliError::Output(error) => write_output_error(error, self.language, formatter),
            CliError::Read { path, source } => write!(
                formatter,
                "{} {}: {source}",
                self.language.select("failed to read", "읽을 수 없습니다:"),
                path.display()
            ),
            CliError::MissingData(path) => write!(
                formatter,
                "{}: {}",
                self.language
                    .select("full POS lexicon is missing", "full POS lexicon이 없습니다"),
                path.display()
            ),
            CliError::Stderr(error) => write!(
                formatter,
                "{}: {error}",
                self.language.select(
                    "failed to write diagnostics",
                    "진단 메시지를 출력할 수 없습니다"
                )
            ),
        }
    }
}

impl CliError {
    #[must_use]
    pub const fn localized(&self, language: Language) -> LocalizedCliError<'_> {
        LocalizedCliError::new(self, language)
    }
}

pub fn write_cli_error(
    writer: &mut impl Write,
    error: &CliError,
    language: Language,
) -> io::Result<()> {
    writer.write_all(b"kfind: ")?;
    crate::output::write_safe_text(writer, &error.localized(language).to_string())?;
    writer.write_all(b"\n")
}

fn write_compile_option(
    error: &CompileOptionError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        CompileOptionError::LiteralExpandConflict { expand } => write!(
            formatter,
            "{} --expand {}",
            language.select(
                "--literal conflicts with",
                "--literal은 다음 옵션과 함께 사용할 수 없습니다:"
            ),
            expand_label(*expand)
        ),
        CompileOptionError::LiteralPosConflict { pos } => write!(
            formatter,
            "{} --pos {}",
            language.select(
                "--literal conflicts with",
                "--literal은 다음 옵션과 함께 사용할 수 없습니다:"
            ),
            pos_label(*pos)
        ),
    }
}

fn write_compile_error(
    error: &CompileError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    if let Some(index) = error.atom_index {
        write!(formatter, "atom[{index}]: ")?;
    }
    match error.kind.as_ref() {
        CompileErrorKind::Query(error) => write_query_error(error, language, formatter),
        CompileErrorKind::Analyze(error) => write_analyze_error(error, language, formatter),
        CompileErrorKind::Generate(error) => write_generate_error(error, language, formatter),
        CompileErrorKind::TooManyAnalyses { actual, limit } => write!(
            formatter,
            "{} {actual} {} {limit}",
            language.select("analysis count", "분석 수"),
            language.select("exceeds limit", "이 제한을 초과했습니다:")
        ),
        CompileErrorKind::TooManyBranches { actual, limit } => write!(
            formatter,
            "{} {actual} {} {limit}",
            language.select("branch count", "branch 수"),
            language.select("exceeds limit", "이 제한을 초과했습니다:")
        ),
        CompileErrorKind::MatcherMemoryExceeded { estimated, limit } => write!(
            formatter,
            "{} {estimated} bytes {} {limit}",
            language.select("estimated matcher memory", "예상 matcher 메모리"),
            language.select("exceeds limit", "가 제한을 초과했습니다:")
        ),
        CompileErrorKind::ContinuationDepthExceeded { actual, limit } => write!(
            formatter,
            "{} {actual} {} {limit}",
            language.select("continuation depth", "continuation 깊이"),
            language.select("exceeds limit", "가 제한을 초과했습니다:")
        ),
        CompileErrorKind::NoSearchableBranches => formatter.write_str(language.select(
            "query has no searchable branches",
            "쿼리에 검색 가능한 branch가 없습니다",
        )),
        CompileErrorKind::InvalidCoreMapping => formatter.write_str(language.select(
            "morphology core mapping is not on a UTF-8 boundary",
            "형태소 core mapping이 UTF-8 경계에 있지 않습니다",
        )),
    }
}

fn write_query_error(
    error: &QueryError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match &error.kind {
        QueryErrorKind::EmptyQuery => formatter.write_str(language.select(
            "query must contain at least one atom",
            "쿼리에는 하나 이상의 atom이 있어야 합니다",
        ))?,
        QueryErrorKind::EmptyAtom => formatter.write_str(language.select(
            "query atom must not be empty",
            "쿼리 atom은 비어 있을 수 없습니다",
        ))?,
        QueryErrorKind::DanglingEscape => formatter.write_str(language.select(
            "backslash must escape another character",
            "역슬래시는 다음 문자를 escape해야 합니다",
        ))?,
        QueryErrorKind::UnterminatedQuote { quote } => write!(
            formatter,
            "{} {quote}",
            language.select("unterminated quote", "닫히지 않은 따옴표")
        )?,
        QueryErrorKind::QueryTooLong { actual, limit } => write!(
            formatter,
            "{} {actual}; {} {limit}",
            language.select("query Unicode scalar count", "쿼리 Unicode scalar 수"),
            language.select("limit is", "제한은")
        )?,
        QueryErrorKind::TooManyAtoms { actual, limit } => write!(
            formatter,
            "{} {actual}; {} {limit}",
            language.select("query atom count", "쿼리 atom 수"),
            language.select("limit is", "제한은")
        )?,
        QueryErrorKind::ConflictingPos { global, tagged } => write!(
            formatter,
            "{} {} / {}",
            language.select(
                "global part of speech conflicts with atom tag",
                "전역 품사와 atom 태그가 충돌합니다"
            ),
            pos_label(*global),
            pos_label(*tagged)
        )?,
    }
    write!(
        formatter,
        " {} {}..{}",
        language.select("at bytes", "byte 위치"),
        error.span.start,
        error.span.end
    )
}

fn write_analyze_error(
    error: &AnalyzeError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        AnalyzeError::InvalidForcedPredicateLemma { lemma, pos } => write!(
            formatter,
            "{} {} `{lemma}` {}",
            language.select("forced", "강제 지정한"),
            pos_label(*pos),
            language.select(
                "must be a non-empty -다 lemma",
                "는 비어 있지 않은 `다` 표제어여야 합니다"
            )
        ),
    }
}

fn write_generate_error(
    error: &GenerateError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        GenerateError::InvalidLemma(lemma) => write!(
            formatter,
            "{}: {lemma}",
            language.select(
                "predicate lemma must be a non-empty -다 form",
                "용언 표제어는 비어 있지 않은 `다` 기본형이어야 합니다"
            )
        ),
        GenerateError::AlternationMismatch { lemma, alternation } => write!(
            formatter,
            "{} {alternation:?}: {lemma}",
            language.select(
                "predicate stem does not satisfy alternation",
                "용언 어간이 alternation 조건을 만족하지 않습니다"
            )
        ),
        GenerateError::InvalidOverride { lemma, surface } => write!(
            formatter,
            "{} {lemma}: {surface}",
            language.select(
                "override core length is invalid for",
                "override core 길이가 올바르지 않습니다"
            )
        ),
    }
}

fn write_matcher_error(
    error: &MorphMatcherBuildError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        MorphMatcherBuildError::EmptyPlan => formatter.write_str(language.select(
            "a morphology matcher requires at least one atom",
            "형태 matcher에는 하나 이상의 atom이 필요합니다",
        )),
        MorphMatcherBuildError::EmptyAtom { atom_index } => write!(
            formatter,
            "{} {atom_index} {}",
            language.select("query atom", "쿼리 atom"),
            language.select("has no search branches", "에 검색 branch가 없습니다")
        ),
        MorphMatcherBuildError::Anchor(error) => write_anchor_error(error, language, formatter),
    }
}

fn write_anchor_error(
    error: &AnchorBuildError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        AnchorBuildError::EmptySet => formatter.write_str(language.select(
            "an anchor engine requires at least one anchor",
            "anchor engine에는 하나 이상의 anchor가 필요합니다",
        )),
        AnchorBuildError::EmptyAnchor(index) => write!(
            formatter,
            "anchor {index} {}",
            language.select("is empty", "가 비어 있습니다")
        ),
        AnchorBuildError::TooManyAnchors { actual, limit } => write!(
            formatter,
            "anchor {} {actual}; {} {limit}",
            language.select("count", "수"),
            language.select("limit is", "제한은")
        ),
        AnchorBuildError::MemoryLimit { estimated, limit } => write!(
            formatter,
            "anchor matcher: {estimated} bytes; {} {limit}",
            language.select("limit is", "제한은")
        ),
        AnchorBuildError::DuplicateAnchor { first, duplicate } => write!(
            formatter,
            "anchor {duplicate} {} anchor {first}",
            language.select("duplicates", "가 다음과 중복됩니다:")
        ),
        AnchorBuildError::Build(message) => write!(
            formatter,
            "{}: {message}",
            language.select("failed to build anchor matcher", "anchor matcher 생성 실패")
        ),
    }
}

fn write_search_error(
    error: &SearchRunError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        SearchRunError::Walk(error) => write_walk_error(error, language, formatter),
        SearchRunError::Input(error) => write_input_error(error, language, formatter),
        SearchRunError::Output(error) => write!(
            formatter,
            "{}: {error}",
            language.select("failed to write search output", "검색 결과 출력 실패")
        ),
        SearchRunError::CallbackPanic(message) => write!(
            formatter,
            "{}: {message}",
            language.select("output callback panicked", "출력 callback이 중단됨")
        ),
        SearchRunError::WriterPanic(message) => write!(
            formatter,
            "{}: {message}",
            language.select("output writer panicked", "출력 writer가 중단됨")
        ),
    }
}

fn write_walk_error(
    error: &WalkConfigError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        WalkConfigError::NoPaths => formatter.write_str(language.select(
            "at least one search path is required",
            "하나 이상의 검색 경로가 필요합니다",
        )),
        WalkConfigError::CurrentDir(error) => write!(
            formatter,
            "{}: {error}",
            language.select(
                "failed to read current directory",
                "현재 디렉터리를 읽을 수 없습니다"
            )
        ),
        WalkConfigError::Glob(error) => write!(
            formatter,
            "{}: {error}",
            language.select("invalid glob", "glob이 올바르지 않습니다")
        ),
        WalkConfigError::FileType(error) => write!(
            formatter,
            "{}: {error}",
            language.select("invalid file type", "파일 유형이 올바르지 않습니다")
        ),
    }
}

fn write_input_error(
    error: &InputSearchError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        InputSearchError::Encoding(error) => write!(
            formatter,
            "{}: {error}",
            language.select("invalid input encoding", "입력 인코딩이 올바르지 않습니다")
        ),
        InputSearchError::MatchLimitExceeded { limit } => write!(
            formatter,
            "{} {limit}",
            language.select(
                "matches per line exceed limit",
                "줄별 match 수가 제한을 초과했습니다:"
            )
        ),
        InputSearchError::Io(error) => write!(
            formatter,
            "{}: {error}",
            language.select("failed to read input", "입력 읽기 실패")
        ),
    }
}

fn write_output_error(
    error: &OutputError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    match error {
        OutputError::Io(error) => write!(
            formatter,
            "{}: {error}",
            language.select("failed to write output", "출력 실패")
        ),
        OutputError::Json(error) => write!(
            formatter,
            "{}: {error}",
            language.select("failed to serialize JSON output", "JSON 결과 직렬화 실패")
        ),
    }
}

const fn expand_label(expand: ExpandMode) -> &'static str {
    match expand {
        ExpandMode::Literal => "literal",
        ExpandMode::Inflection => "inflection",
        ExpandMode::Derivation => "derivation",
    }
}

const fn pos_label(pos: CoarsePos) -> &'static str {
    match pos {
        CoarsePos::Noun => "noun",
        CoarsePos::Pronoun => "pronoun",
        CoarsePos::Numeral => "numeral",
        CoarsePos::Verb => "verb",
        CoarsePos::Adjective => "adjective",
        CoarsePos::Determiner => "determiner",
        CoarsePos::Adverb => "adverb",
        CoarsePos::Particle => "particle",
        CoarsePos::Interjection => "interjection",
        CoarsePos::Literal => "literal",
    }
}

#[cfg(test)]
mod tests;
