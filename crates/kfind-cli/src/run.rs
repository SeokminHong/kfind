use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, ComponentResource, DataError, decode_component_resource,
    parse_user_lexicon_toml,
};
use kfind_matcher::{MorphMatcher, MorphMatcherBuildError};
use kfind_morph::LocalComponentEvaluator;
use kfind_query::{
    CompileError, CompileOptionError, LexiconQueryAnalyzer, Lexicons, compile_query,
};
use kfind_search::{
    ExecutionOptions, InputEncoding, InputOptions, ResultOrder, SearchConfig, SearchEvent,
    SearchRunError, SearchSummary, WalkOptions, execute_search_with_stdin, resolve_search_paths,
};

use crate::output::{FullPosNotRequiredReason, FullPosStatus, write_safe_path, write_safe_text};
use crate::{Args, EncodingArg, Language, OutputError, OutputOptions, OutputWriter, SortArg};

const FULL_POS_FILE: &str = "lexicon.bin";
const COMPONENT_RESOURCE_FILE: &str = "morphology-component-compact.kfc";

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
    let full_pos_mode = if args.embedded {
        FullPosMode::Disabled(FullPosNotRequiredReason::EmbeddedMode)
    } else if options.requires_full_pos_lexicon() {
        FullPosMode::Auto
    } else {
        FullPosMode::Disabled(FullPosNotRequiredReason::LiteralQuery)
    };
    let loaded_lexicons = load_lexicons(args, full_pos_mode)?;
    let full_pos_status = loaded_lexicons.full_pos;
    let lexicons = Arc::new(loaded_lexicons.lexicons);
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan =
        Arc::new(compile_query(&args.query, &options, &analyzer).map_err(CliError::Compile)?);
    let matcher = if plan.requires_component_resource() {
        let resource = Arc::new(load_component_resource(args)?);
        let evaluator = Arc::new(LocalComponentEvaluator::new(resource));
        MorphMatcher::with_component_evaluator(Arc::clone(&plan), evaluator)
    } else {
        MorphMatcher::new(Arc::clone(&plan))
    }
    .map(Arc::new)
    .map_err(CliError::Matcher)?;
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

#[derive(Clone, Copy)]
enum FullPosMode {
    Auto,
    Disabled(FullPosNotRequiredReason),
}

fn load_lexicons(args: &Args, full_pos_mode: FullPosMode) -> Result<LoadedLexicons, CliError> {
    let mut lexicons = Lexicons::embedded().map_err(CliError::Data)?;
    let full_pos = match full_pos_mode {
        FullPosMode::Auto => {
            let resolved_full_pos = resolve_full_pos(args)?;
            if let FullPosStatus::Loaded { path } = &resolved_full_pos {
                let bytes = fs::read(path).map_err(|source| CliError::Read {
                    path: path.clone(),
                    source,
                })?;
                lexicons.load_full_pos(&bytes).map_err(CliError::Data)?;
            }
            resolved_full_pos
        }
        FullPosMode::Disabled(reason) => FullPosStatus::NotRequired(reason),
    };
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

fn load_component_resource(args: &Args) -> Result<ComponentResource, CliError> {
    let path = resolve_component_resource(args)?;
    let bytes = fs::read(&path).map_err(|source| CliError::Read {
        path: path.clone(),
        source,
    })?;
    decode_component_resource(
        &path.to_string_lossy(),
        bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .map_err(CliError::Data)
}

fn resolve_component_resource(args: &Args) -> Result<PathBuf, CliError> {
    if let Some(directory) = &args.data_dir {
        return require_component_candidate(vec![directory.join(COMPONENT_RESOURCE_FILE)]);
    }
    if let Some(directory) = env::var_os("KFIND_DATA_DIR") {
        return require_component_candidate(vec![
            PathBuf::from(directory).join(COMPONENT_RESOURCE_FILE),
        ]);
    }
    require_component_candidate(auto_component_candidates())
}

fn require_component_candidate(candidates: Vec<PathBuf>) -> Result<PathBuf, CliError> {
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .ok_or_else(|| CliError::MissingComponent(candidates.into_boxed_slice()))
}

fn auto_component_candidates() -> Vec<PathBuf> {
    auto_data_candidates(COMPONENT_RESOURCE_FILE)
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

fn auto_data_candidates(file: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = env::current_exe() {
        if let Some(prefix) = executable.parent().and_then(Path::parent) {
            push_candidate(&mut candidates, prefix.join("share/kfind").join(file));
        }
    }
    if let Some(directory) = env::var_os("XDG_DATA_HOME") {
        push_candidate(
            &mut candidates,
            PathBuf::from(directory).join("kfind").join(file),
        );
    } else if let Some(home) = env::var_os("HOME") {
        push_candidate(
            &mut candidates,
            PathBuf::from(home).join(".local/share/kfind").join(file),
        );
    }
    for path in [
        PathBuf::from("data/generated/component").join(file),
        PathBuf::from("data/generated").join(file),
        PathBuf::from("/opt/homebrew/share/kfind").join(file),
        PathBuf::from("/usr/local/share/kfind").join(file),
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
    MissingComponent(Box<[PathBuf]>),
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
            Self::MissingComponent(paths) => {
                formatter.write_str("component resource is missing")?;
                for path in paths {
                    write!(formatter, ": {}", path.display())?;
                }
                Ok(())
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
            Self::MissingData(_) | Self::MissingComponent(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
