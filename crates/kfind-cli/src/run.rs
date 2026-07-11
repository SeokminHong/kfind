use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kfind_data::{DataError, parse_user_lexicon_toml};
use kfind_matcher::{MorphMatcher, MorphMatcherBuildError};
use kfind_query::{
    CompileError, CompileOptionError, LexiconQueryAnalyzer, Lexicons, compile_query,
};
use kfind_search::{
    ExecutionOptions, InputEncoding, InputOptions, ResultOrder, SearchConfig, SearchEvent,
    SearchRunError, SearchSummary, WalkOptions, execute_search_with_stdin, resolve_search_paths,
};

use crate::output::{FullPosStatus, write_safe_path, write_safe_text};
use crate::{Args, EncodingArg, Language, OutputError, OutputOptions, OutputWriter, SortArg};

const FULL_POS_FILE: &str = "lexicon.bin";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExitStatus {
    Match = 0,
    NoMatch = 1,
    Error = 2,
}

impl ExitStatus {
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

pub fn run_with_io<R, W, E>(
    args: &Args,
    language: Language,
    stdin: R,
    stdout: &mut W,
    stderr: &mut E,
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> Result<ExitStatus, CliError>
where
    R: Read,
    W: Write + Send,
    E: Write + Send,
{
    let options = args.compile_options().map_err(CliError::Options)?;
    let loaded_lexicons = load_lexicons(args)?;
    let full_pos_status = loaded_lexicons.full_pos;
    let lexicons = Arc::new(loaded_lexicons.lexicons);
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan =
        Arc::new(compile_query(&args.query, &options, &analyzer).map_err(CliError::Compile)?);
    let matcher = Arc::new(MorphMatcher::new(Arc::clone(&plan)).map_err(CliError::Matcher)?);
    let paths = resolve_search_paths(&args.paths, stdin_is_terminal);
    let output_options = OutputOptions::from_args_with_language(
        args,
        language,
        stdout_is_terminal,
        should_print_filenames(&paths),
    );
    let mut output = OutputWriter::new(stdout, output_options);

    if args.explain_query {
        if let Err(error) = output.write_query_plan_with_full_pos(&plan, &full_pos_status) {
            if error.is_broken_pipe() {
                return Ok(ExitStatus::Match);
            }
            return Err(CliError::Output(error));
        }
    }

    let config = search_config(args, paths);
    let summary = execute_search_with_stdin(matcher, config, stdin, |event| match event {
        SearchEvent::FileStart { .. } => Ok(()),
        SearchEvent::Record { path, record } => output
            .write_record(path, record, &plan)
            .map_err(output_error_as_io),
        SearchEvent::FileEnd(result) => {
            output.write_file(result, &plan).map_err(output_error_as_io)
        }
        SearchEvent::Issue(issue) => write_issue(stderr, issue, language),
    })
    .map_err(CliError::Search)?;

    if !summary.output_closed {
        if let Err(error) = output.flush() {
            if !error.is_broken_pipe() {
                return Err(CliError::Output(error));
            }
        }
        stderr.flush().map_err(CliError::Stderr)?;
    }
    Ok(status_from_summary(summary))
}

struct LoadedLexicons {
    lexicons: Lexicons,
    full_pos: FullPosStatus,
}

fn load_lexicons(args: &Args) -> Result<LoadedLexicons, CliError> {
    let mut lexicons = Lexicons::embedded().map_err(CliError::Data)?;
    let full_pos = resolve_full_pos(args)?;
    if let FullPosStatus::Loaded { path } = &full_pos {
        let bytes = fs::read(path).map_err(|source| CliError::Read {
            path: path.clone(),
            source,
        })?;
        lexicons.load_full_pos(&bytes).map_err(CliError::Data)?;
    }
    if let Some(path) = user_lexicon_path(args) {
        let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
            path: path.clone(),
            source,
        })?;
        let user = parse_user_lexicon_toml(&path.to_string_lossy(), &source, lexicons.rules())
            .map_err(CliError::Data)?;
        lexicons.merge_user(&user);
    }
    Ok(LoadedLexicons { lexicons, full_pos })
}

fn resolve_full_pos(args: &Args) -> Result<FullPosStatus, CliError> {
    if let Some(directory) = &args.data_dir {
        let path = directory.join(FULL_POS_FILE);
        return if path.is_file() {
            Ok(FullPosStatus::Loaded { path })
        } else {
            Err(CliError::MissingData(path))
        };
    }

    Ok(select_full_pos(auto_full_pos_candidates()))
}

fn auto_full_pos_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(directory) = env::var_os("KFIND_DATA_DIR") {
        push_candidate(
            &mut candidates,
            PathBuf::from(directory).join(FULL_POS_FILE),
        );
    }
    if let Ok(executable) = env::current_exe() {
        if let Some(prefix) = executable.parent().and_then(Path::parent) {
            push_candidate(
                &mut candidates,
                prefix.join("share/kfind").join(FULL_POS_FILE),
            );
        }
    }
    if let Some(directory) = env::var_os("XDG_DATA_HOME") {
        push_candidate(
            &mut candidates,
            PathBuf::from(directory).join("kfind").join(FULL_POS_FILE),
        );
    } else if let Some(home) = env::var_os("HOME") {
        push_candidate(
            &mut candidates,
            PathBuf::from(home)
                .join(".local/share/kfind")
                .join(FULL_POS_FILE),
        );
    }
    for path in [
        PathBuf::from("data/generated/full-pos").join(FULL_POS_FILE),
        PathBuf::from("data/generated").join(FULL_POS_FILE),
        PathBuf::from("/opt/homebrew/share/kfind").join(FULL_POS_FILE),
        PathBuf::from("/usr/local/share/kfind").join(FULL_POS_FILE),
    ] {
        push_candidate(&mut candidates, path);
    }
    candidates
}

fn push_candidate(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.contains(&path) {
        candidates.push(path);
    }
}

fn select_full_pos(candidates: Vec<PathBuf>) -> FullPosStatus {
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .map_or_else(
            || FullPosStatus::Preview {
                candidate_paths: candidates.into_boxed_slice(),
            },
            |path| FullPosStatus::Loaded { path },
        )
}

fn user_lexicon_path(args: &Args) -> Option<PathBuf> {
    if let Some(path) = &args.user_lexicon {
        return Some(path.clone());
    }
    if let Some(path) = env::var_os("KFIND_USER_LEXICON") {
        return Some(PathBuf::from(path));
    }
    let path = if let Some(directory) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(directory).join("kfind/lexicon.toml")
    } else {
        PathBuf::from(env::var_os("HOME")?).join(".config/kfind/lexicon.toml")
    };
    path.is_file().then_some(path)
}

fn search_config(args: &Args, paths: Vec<PathBuf>) -> SearchConfig {
    let context = args.context.unwrap_or(0);
    let summary_mode = args.count || args.files_with_matches || args.quiet;
    SearchConfig {
        paths,
        walk: WalkOptions {
            hidden: args.hidden,
            no_ignore: args.no_ignore,
            threads: args.threads,
            globs: args.glob.clone(),
            selected_types: args.file_type.clone(),
            type_definitions: args.type_add.clone(),
            ..WalkOptions::default()
        },
        input: InputOptions {
            encoding: input_encoding(args.encoding),
            before_context: args.before_context.unwrap_or(context),
            after_context: args.after_context.unwrap_or(context),
            capture_records: !summary_mode,
            stop_after_first_match: args.files_with_matches || args.quiet,
        },
        execution: ExecutionOptions {
            quiet: args.quiet,
            order: match args.sort {
                Some(SortArg::Path) => ResultOrder::Path,
                None => ResultOrder::Unspecified,
            },
            ..ExecutionOptions::default()
        },
    }
}

const fn input_encoding(encoding: EncodingArg) -> InputEncoding {
    match encoding {
        EncodingArg::Auto => InputEncoding::Auto,
        EncodingArg::Utf8 => InputEncoding::Utf8,
        EncodingArg::Utf16le => InputEncoding::Utf16Le,
        EncodingArg::Utf16be => InputEncoding::Utf16Be,
        EncodingArg::EucKr => InputEncoding::EucKr,
    }
}

fn should_print_filenames(paths: &[PathBuf]) -> bool {
    paths.len() > 1
        || paths
            .iter()
            .any(|path| path != Path::new("-") && path.is_dir())
}

fn write_issue(
    writer: &mut impl Write,
    issue: &kfind_search::SearchIssue,
    language: Language,
) -> io::Result<()> {
    writer.write_all(b"kfind: ")?;
    if let Some(path) = &issue.path {
        write_safe_path(writer, path)?;
        writer.write_all(b": ")?;
    }
    write_safe_text(writer, search_issue_context(language, issue.kind))?;
    writer.write_all(b": ")?;
    write_safe_text(writer, &search_issue_detail(&issue.message, language))?;
    writer.write_all(b"\n")
}

const fn search_issue_context(
    language: Language,
    kind: kfind_search::SearchIssueKind,
) -> &'static str {
    match kind {
        kfind_search::SearchIssueKind::Walk => {
            language.select("file traversal failed", "파일 탐색 실패")
        }
        kfind_search::SearchIssueKind::Input => {
            language.select("input search failed", "입력 검색 실패")
        }
        kfind_search::SearchIssueKind::WorkerPanic => {
            language.select("search worker panicked", "검색 worker가 중단됨")
        }
    }
}

fn search_issue_detail(message: &str, language: Language) -> Cow<'_, str> {
    if language == Language::Korean {
        if let Some(detail) = message.strip_prefix("invalid input encoding: ") {
            return Cow::Owned(format!("입력 인코딩이 올바르지 않습니다: {detail}"));
        }
        if let Some(limit) = message.strip_prefix("matches per line exceed limit ") {
            return Cow::Owned(format!("줄별 match 수가 제한 {limit}을 초과했습니다"));
        }
        if message == "panic without a string payload" {
            return Cow::Borrowed("문자열 정보 없이 panic이 발생했습니다");
        }
        if message == "file search stream closed before completion" {
            return Cow::Borrowed("파일 검색 stream이 완료 전에 닫혔습니다");
        }
    }
    Cow::Borrowed(message)
}

fn output_error_as_io(error: OutputError) -> io::Error {
    match error {
        OutputError::Io(error) => error,
        OutputError::Json(error) => io::Error::other(error),
    }
}

const fn status_from_summary(summary: SearchSummary) -> ExitStatus {
    if summary.errors > 0 {
        ExitStatus::Error
    } else if summary.has_match || summary.output_closed {
        ExitStatus::Match
    } else {
        ExitStatus::NoMatch
    }
}

#[derive(Debug)]
pub enum CliError {
    Options(CompileOptionError),
    Data(DataError),
    Compile(CompileError),
    Matcher(MorphMatcherBuildError),
    Search(SearchRunError),
    Output(OutputError),
    Read { path: PathBuf, source: io::Error },
    MissingData(PathBuf),
    Stderr(io::Error),
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Options(error) => Display::fmt(error, formatter),
            Self::Data(error) => Display::fmt(error, formatter),
            Self::Compile(error) => Display::fmt(error, formatter),
            Self::Matcher(error) => Display::fmt(error, formatter),
            Self::Search(error) => Display::fmt(error, formatter),
            Self::Output(error) => Display::fmt(error, formatter),
            Self::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::MissingData(path) => {
                write!(formatter, "full POS lexicon is missing: {}", path.display())
            }
            Self::Stderr(error) => write!(formatter, "failed to write diagnostics: {error}"),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Options(error) => Some(error),
            Self::Data(error) => Some(error),
            Self::Compile(error) => Some(error),
            Self::Matcher(error) => Some(error),
            Self::Search(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::Read { source, .. } | Self::Stderr(source) => Some(source),
            Self::MissingData(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
