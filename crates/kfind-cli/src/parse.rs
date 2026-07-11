use std::ffi::OsString;
use std::fmt::{self, Display, Formatter, Write as _};

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Command, CommandFactory, FromArgMatches};

use crate::{Args, Language};

const HELP_TEMPLATE_ENGLISH: &str =
    "{about-with-newline}\nUsage: {usage}\n\nArguments:\n{positionals}\nOptions:\n{options}";
const HELP_TEMPLATE_KOREAN: &str =
    "{about-with-newline}\n사용법: {usage}\n\n인수:\n{positionals}\n옵션:\n{options}";

const HELP_TEXT: &[(&str, &str, &str)] = &[
    (
        "query",
        "Korean lemma, short phrase, or tagged query.",
        "한국어 표제어, 짧은 구 또는 품사 태그 쿼리.",
    ),
    (
        "paths",
        "Files and directories to search. Uses stdin when piped, otherwise '.'.",
        "검색할 파일과 디렉터리. 파이프 입력이 있으면 stdin, 없으면 '.'을 사용합니다.",
    ),
    (
        "pos",
        "Force one part of speech (default: auto; values: auto, noun, pronoun, numeral, verb, adjective, determiner, adverb, particle, interjection, literal).",
        "품사를 강제합니다(기본값: auto; 값: auto, noun, pronoun, numeral, verb, adjective, determiner, adverb, particle, interjection, literal).",
    ),
    (
        "expand",
        "Choose the expansion level (default: inflection; values: literal, inflection, derivation).",
        "확장 수준을 선택합니다(기본값: inflection; 값: literal, inflection, derivation).",
    ),
    (
        "boundary",
        "Choose the token boundary policy (default: smart; values: smart, token, any).",
        "토큰 경계 정책을 선택합니다(기본값: smart; 값: smart, token, any).",
    ),
    (
        "literal",
        "Search only the literal query without morphology expansion.",
        "형태 확장 없이 쿼리 문자열만 검색합니다.",
    ),
    (
        "max_gap",
        "Set the maximum Unicode scalar gap between phrase atoms (default: 24).",
        "구를 구성하는 atom 사이의 최대 Unicode scalar 거리를 지정합니다(기본값: 24).",
    ),
    (
        "unicode_normalization",
        "Choose Unicode matching (default: nfc; values: nfc, canonical, none).",
        "Unicode 검색 방식을 선택합니다(기본값: nfc; 값: nfc, canonical, none).",
    ),
    (
        "encoding",
        "Choose the input encoding (default: auto; values: auto, utf-8, utf-16le, utf-16be, euc-kr).",
        "입력 인코딩을 선택합니다(기본값: auto; 값: auto, utf-8, utf-16le, utf-16be, euc-kr).",
    ),
    (
        "glob",
        "Add an include or exclude glob.",
        "포함 또는 제외 glob을 추가합니다.",
    ),
    (
        "file_type",
        "Search only a named file type.",
        "지정한 파일 유형만 검색합니다.",
    ),
    (
        "type_add",
        "Define a file type as NAME:GLOB.",
        "파일 유형을 NAME:GLOB 형식으로 정의합니다.",
    ),
    (
        "hidden",
        "Search hidden files and directories.",
        "숨김 파일과 디렉터리도 검색합니다.",
    ),
    (
        "no_ignore",
        "Search without applying .gitignore or other ignore files.",
        ".gitignore 등 ignore 파일을 적용하지 않습니다.",
    ),
    (
        "threads",
        "Set the number of search worker threads.",
        "검색 worker thread 수를 지정합니다.",
    ),
    (
        "line_number",
        "Print line numbers.",
        "줄 번호를 출력합니다.",
    ),
    (
        "with_filename",
        "Always print file names.",
        "파일 이름을 항상 출력합니다.",
    ),
    (
        "no_filename",
        "Never print file names.",
        "파일 이름을 출력하지 않습니다.",
    ),
    (
        "context",
        "Print NUM lines before and after each match.",
        "각 match 앞뒤로 NUM개 줄을 출력합니다.",
    ),
    (
        "before_context",
        "Print NUM lines before each match.",
        "각 match 앞에 NUM개 줄을 출력합니다.",
    ),
    (
        "after_context",
        "Print NUM lines after each match.",
        "각 match 뒤에 NUM개 줄을 출력합니다.",
    ),
    (
        "files_with_matches",
        "Print only file names containing a match.",
        "match가 있는 파일 이름만 출력합니다.",
    ),
    (
        "count",
        "Print the number of matching lines per file.",
        "파일별로 match가 있는 줄 수를 출력합니다.",
    ),
    (
        "quiet",
        "Print no matches and stop after the first match.",
        "match를 출력하지 않고 첫 match에서 검색을 멈춥니다.",
    ),
    (
        "json",
        "Write JSON Lines output.",
        "JSON Lines 형식으로 출력합니다.",
    ),
    (
        "color",
        "Choose color output (default: auto; values: auto, always, never).",
        "색상 출력을 선택합니다(기본값: auto; 값: auto, always, never).",
    ),
    (
        "column",
        "Print one-based Unicode scalar columns.",
        "1부터 시작하는 Unicode scalar 열 번호를 출력합니다.",
    ),
    (
        "explain_query",
        "Print the compiled query plan.",
        "컴파일된 쿼리 계획을 출력합니다.",
    ),
    (
        "explain_match",
        "Print the lemma and rules behind each match.",
        "각 match를 생성한 표제어와 규칙을 출력합니다.",
    ),
    (
        "sort",
        "Choose result ordering (values: path).",
        "결과 정렬 방식을 선택합니다(값: path).",
    ),
    (
        "data_dir",
        "Read the full POS lexicon from PATH.",
        "PATH에서 full POS lexicon을 읽습니다.",
    ),
    (
        "user_lexicon",
        "Read a user lexicon from PATH.",
        "PATH에서 사용자 사전을 읽습니다.",
    ),
    ("help", "Print help.", "도움말을 출력합니다."),
    ("version", "Print version.", "버전을 출력합니다."),
];

pub fn parse_args_from<I, T>(values: I, language: Language) -> Result<Args, CliParseError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = localized_command(language)
        .try_get_matches_from(values)
        .map_err(|error| CliParseError::new(&error, language))?;
    Args::from_arg_matches(&matches).map_err(|error| CliParseError::new(&error, language))
}

pub(crate) fn localized_command(language: Language) -> Command {
    let mut command = Args::command()
        .about(language.select(
            "Fast Korean lemma and inflection search for code and documents.",
            "한국어 표제어와 활용형을 빠르게 찾는 코드·문서 검색 CLI.",
        ))
        .help_template(language.select(HELP_TEMPLATE_ENGLISH, HELP_TEMPLATE_KOREAN));
    for &(id, english, korean) in HELP_TEXT {
        command = command.mut_arg(id, |argument| {
            argument.help(language.select(english, korean))
        });
    }
    for id in [
        "pos",
        "expand",
        "boundary",
        "unicode_normalization",
        "encoding",
        "color",
        "sort",
    ] {
        command = command.mut_arg(id, |argument| {
            argument.hide_default_value(true).hide_possible_values(true)
        });
    }
    command
}

#[derive(Debug)]
pub struct CliParseError {
    output: String,
    exit_code: u8,
    use_stderr: bool,
}

impl CliParseError {
    fn new(error: &clap::Error, language: Language) -> Self {
        let use_stderr = error.use_stderr();
        let output = if use_stderr {
            format_argument_error(error, language)
        } else {
            error.to_string()
        };
        Self {
            output,
            exit_code: u8::try_from(error.exit_code()).unwrap_or(2),
            use_stderr,
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.exit_code
    }

    #[must_use]
    pub const fn use_stderr(&self) -> bool {
        self.use_stderr
    }
}

impl Display for CliParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.output)
    }
}

fn format_argument_error(error: &clap::Error, language: Language) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "{}: {}",
        language.select("error", "오류"),
        error_detail(error, language)
    );
    if let Some(suggestion) = suggestion(error, language) {
        let _ = writeln!(output, "\n  {}:", language.select("tip", "해결 방법"));
        let _ = writeln!(output, "    {suggestion}");
    }
    let _ = writeln!(
        output,
        "\n{}: {}\n",
        language.select("Usage", "사용법"),
        rendered_usage(language)
    );
    let _ = writeln!(
        output,
        "{}",
        language.select(
            "For more information, try '--help'.",
            "자세한 내용은 '--help'를 실행하세요."
        )
    );
    output
}

fn error_detail(error: &clap::Error, language: Language) -> String {
    let invalid_arg = context(error, ContextKind::InvalidArg);
    let invalid_value = context(error, ContextKind::InvalidValue);
    match error.kind() {
        ErrorKind::ArgumentConflict => {
            let prior = context(error, ContextKind::PriorArg);
            match (invalid_arg, prior) {
                (Some(argument), Some(prior)) if argument == prior => language
                    .select(
                        &format!("the argument '{argument}' cannot be used multiple times"),
                        &format!("인수 '{argument}'는 여러 번 사용할 수 없습니다"),
                    )
                    .to_owned(),
                (Some(argument), Some(prior)) => language
                    .select(
                        &format!("the argument '{argument}' cannot be used with '{prior}'"),
                        &format!("인수 '{argument}'와 '{prior}'는 함께 사용할 수 없습니다"),
                    )
                    .to_owned(),
                _ => language
                    .select(
                        "conflicting arguments were provided",
                        "서로 충돌하는 인수가 입력되었습니다",
                    )
                    .to_owned(),
            }
        }
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
            match (invalid_value, invalid_arg) {
                (Some(value), Some(argument)) if value.is_empty() => language
                    .select(
                        &format!("a value is required for '{argument}'"),
                        &format!("'{argument}'에 값이 필요합니다"),
                    )
                    .to_owned(),
                (Some(value), Some(argument)) => {
                    let mut detail = language
                        .select(
                            &format!("invalid value '{value}' for '{argument}'"),
                            &format!("'{argument}'에 사용할 수 없는 값 '{value}'입니다"),
                        )
                        .to_owned();
                    if let Some(values) = context(error, ContextKind::ValidValue) {
                        let _ = write!(
                            detail,
                            "\n  {}: {values}",
                            language.select("possible values", "사용 가능한 값")
                        );
                    }
                    detail
                }
                _ => language
                    .select(
                        "an invalid value was provided",
                        "사용할 수 없는 값이 입력되었습니다",
                    )
                    .to_owned(),
            }
        }
        ErrorKind::UnknownArgument => invalid_arg.map_or_else(
            || {
                language
                    .select(
                        "an unknown argument was provided",
                        "알 수 없는 인수가 입력되었습니다",
                    )
                    .to_owned()
            },
            |argument| {
                language
                    .select(
                        &format!("unexpected argument '{argument}'"),
                        &format!("알 수 없는 인수 '{argument}'입니다"),
                    )
                    .to_owned()
            },
        ),
        ErrorKind::MissingRequiredArgument => {
            let arguments = invalid_arg.unwrap_or_else(|| "<QUERY>".to_owned());
            language
                .select(
                    &format!("required argument was not provided: {arguments}"),
                    &format!("필수 인수가 입력되지 않았습니다: {arguments}"),
                )
                .to_owned()
        }
        ErrorKind::TooManyValues => language
            .select(
                "too many argument values were provided",
                "인수 값이 너무 많이 입력되었습니다",
            )
            .to_owned(),
        ErrorKind::TooFewValues | ErrorKind::WrongNumberOfValues => language
            .select(
                "the wrong number of argument values was provided",
                "인수 값의 개수가 올바르지 않습니다",
            )
            .to_owned(),
        ErrorKind::NoEquals => language
            .select(
                "an equal sign is required when assigning this option",
                "이 옵션에 값을 지정할 때 등호가 필요합니다",
            )
            .to_owned(),
        ErrorKind::InvalidSubcommand | ErrorKind::MissingSubcommand => language
            .select(
                "the subcommand is invalid or missing",
                "하위 명령이 없거나 올바르지 않습니다",
            )
            .to_owned(),
        ErrorKind::InvalidUtf8 => language
            .select(
                "command-line arguments are not valid UTF-8",
                "명령줄 인수가 올바른 UTF-8이 아닙니다",
            )
            .to_owned(),
        _ => language
            .select(
                "invalid command-line arguments",
                "명령줄 인수가 올바르지 않습니다",
            )
            .to_owned(),
    }
}

fn context(error: &clap::Error, kind: ContextKind) -> Option<String> {
    error.get(kind).and_then(|value| match value {
        ContextValue::String(value) => Some(value.clone()),
        ContextValue::Strings(values) => Some(values.join(", ")),
        ContextValue::StyledStr(value) => Some(value.to_string()),
        ContextValue::StyledStrs(values) => Some(
            values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        ContextValue::Number(value) => Some(value.to_string()),
        ContextValue::None | ContextValue::Bool(_) => None,
        _ => None,
    })
}

fn suggestion(error: &clap::Error, language: Language) -> Option<String> {
    let suggested = [
        ContextKind::SuggestedArg,
        ContextKind::SuggestedValue,
        ContextKind::SuggestedSubcommand,
        ContextKind::SuggestedCommand,
    ]
    .into_iter()
    .find_map(|kind| context(error, kind));
    if let Some(suggested) = suggested {
        return Some(
            language
                .select(
                    &format!("use '{suggested}'"),
                    &format!("'{suggested}'를 사용하세요"),
                )
                .to_owned(),
        );
    }
    if let Some(trailing) = context(error, ContextKind::TrailingArg) {
        return Some(
            language
                .select(
                    &format!("to pass '{trailing}' as a value, use '-- {trailing}'"),
                    &format!("'{trailing}'를 값으로 전달하려면 '-- {trailing}'를 사용하세요"),
                )
                .to_owned(),
        );
    }
    let opaque = context(error, ContextKind::Suggested)?;
    if opaque.contains("to pass") {
        let argument = context(error, ContextKind::InvalidArg)?;
        return Some(
            language
                .select(
                    &format!("to pass '{argument}' as a value, use '-- {argument}'"),
                    &format!("'{argument}'를 값으로 전달하려면 '-- {argument}'를 사용하세요"),
                )
                .to_owned(),
        );
    }
    (language == Language::English).then_some(opaque)
}

fn rendered_usage(language: Language) -> String {
    let usage = localized_command(language).render_usage().to_string();
    usage
        .strip_prefix("Usage: ")
        .unwrap_or(usage.as_str())
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn help(language: Language) -> String {
        localized_command(language)
            .try_get_matches_from(["kfind", "--help"])
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn help_is_fully_localized_without_changing_cli_tokens() {
        let english = help(Language::English);
        assert!(english.contains("Usage:"));
        assert!(english.contains("Arguments:"));
        assert!(english.contains("Options:"));
        assert!(english.contains("Print version."));

        let korean = help(Language::Korean);
        assert!(korean.contains("한국어 표제어와 활용형"));
        assert!(korean.contains("사용법:"));
        assert!(korean.contains("인수:"));
        assert!(korean.contains("옵션:"));
        assert!(korean.contains("버전을 출력합니다."));
        assert!(!korean.contains("Usage:"));
        assert!(!korean.contains("[default:"));
        assert!(!korean.contains("[possible values:"));
        for token in ["--pos", "--expand", "auto", "verb", "--help"] {
            assert!(english.contains(token));
            assert!(korean.contains(token));
        }
    }

    #[test]
    fn argument_errors_are_localized() {
        let missing = parse_args_from(["kfind"], Language::Korean).unwrap_err();
        let missing = missing.to_string();
        assert!(missing.contains("오류: 필수 인수가 입력되지 않았습니다"));
        assert!(missing.contains("사용법: kfind"));
        assert!(missing.contains("'--help'를 실행하세요"));
        assert!(!missing.contains("error:"));
        assert!(!missing.contains("Usage:"));

        let invalid = parse_args_from(["kfind", "--pos", "unknown", "걷다"], Language::English)
            .unwrap_err()
            .to_string();
        assert!(invalid.contains("error: invalid value 'unknown'"));
        assert!(invalid.contains("possible values"));
        assert!(invalid.contains("Usage: kfind"));
    }

    #[test]
    fn unknown_and_conflicting_arguments_keep_details() {
        let unknown = parse_args_from(["kfind", "--unknown", "걷다"], Language::Korean)
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("--unknown"));
        assert!(unknown.contains("알 수 없는 인수"));
        assert!(unknown.contains("-- --unknown"));
        assert!(!unknown.contains("to pass"));

        let conflict = parse_args_from(["kfind", "--json", "--count", "걷다"], Language::Korean)
            .unwrap_err()
            .to_string();
        assert!(conflict.contains("--json"));
        assert!(conflict.contains("--count"));
        assert!(conflict.contains("함께 사용할 수 없습니다"));
    }
}
