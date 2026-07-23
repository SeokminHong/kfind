use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read, Write};

use serde::Deserialize;
use serde_json::json;

const MAX_HOOK_INPUT_BYTES: u64 = 1024 * 1024;
const DENIAL_REASON: &str = "Korean search patterns must use kfind instead of literal rg/grep. Retry with kfind, or use kfind --literal when exact surface matching is intentional.";

#[derive(Debug, Deserialize)]
struct HookInput {
    hook_event_name: String,
    tool_name: String,
    tool_input: ToolInput,
}

#[derive(Debug, Deserialize)]
struct ToolInput {
    command: Option<String>,
}

pub fn run_agent_hook_with_io(
    reader: &mut impl Read,
    writer: &mut impl Write,
) -> Result<(), AgentHookError> {
    let input = read_hook_input(reader)?;
    if !matches!(input.tool_name.as_str(), "Bash" | "run_shell_command") {
        return Ok(());
    }
    let command = input
        .tool_input
        .command
        .ok_or(AgentHookError::MissingCommand)?;
    let blocked = contains_korean_literal_search(&command);

    match input.hook_event_name.as_str() {
        "PreToolUse" if blocked => write_json(
            writer,
            &json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": DENIAL_REASON,
                }
            }),
        ),
        "PreToolUse" => Ok(()),
        "BeforeTool" if blocked => write_json(
            writer,
            &json!({
                "decision": "deny",
                "reason": DENIAL_REASON,
            }),
        ),
        "BeforeTool" => write_json(writer, &json!({ "decision": "allow" })),
        event => Err(AgentHookError::UnsupportedEvent(event.to_owned())),
    }
}

fn read_hook_input(reader: &mut impl Read) -> Result<HookInput, AgentHookError> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_HOOK_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(AgentHookError::Read)?;
    if bytes.len() as u64 > MAX_HOOK_INPUT_BYTES {
        return Err(AgentHookError::InputTooLarge);
    }
    serde_json::from_slice(&bytes).map_err(AgentHookError::Json)
}

fn write_json(writer: &mut impl Write, value: &serde_json::Value) -> Result<(), AgentHookError> {
    serde_json::to_writer(&mut *writer, value).map_err(AgentHookError::WriteJson)?;
    writer.write_all(b"\n").map_err(AgentHookError::Write)?;
    writer.flush().map_err(AgentHookError::Write)
}

fn contains_korean_literal_search(command: &str) -> bool {
    let Some(tokens) = tokenize_shell(command) else {
        return false;
    };
    let mut words = Vec::new();
    for token in tokens {
        match token {
            ShellToken::Word(word) => words.push(word),
            ShellToken::Separator => {
                if segment_contains_korean_literal_search(&words) {
                    return true;
                }
                words.clear();
            }
        }
    }
    segment_contains_korean_literal_search(&words)
}

fn segment_contains_korean_literal_search(words: &[String]) -> bool {
    let Some(command_index) = wrapped_command_index(words) else {
        return false;
    };
    let command = executable_name(&words[command_index]);
    let arguments = &words[command_index + 1..];
    match command {
        "rg" => arguments_have_korean_pattern(arguments, SearchTool::Ripgrep),
        "grep" | "egrep" | "fgrep" => arguments_have_korean_pattern(arguments, SearchTool::Grep),
        "git" => git_grep_has_korean_pattern(arguments),
        _ => false,
    }
}

fn wrapped_command_index(words: &[String]) -> Option<usize> {
    let mut index = 0;
    while index < words.len() {
        let word = words[index].as_str();
        if is_assignment(word) || matches!(word, "if" | "then" | "elif" | "do" | "else") {
            index += 1;
            continue;
        }
        match executable_name(word) {
            "command" | "builtin" | "exec" | "nohup" | "time" => {
                index += 1;
                while words
                    .get(index)
                    .is_some_and(|argument| argument.starts_with('-'))
                {
                    index += 1;
                }
            }
            "env" => {
                index += 1;
                while let Some(argument) = words.get(index) {
                    if is_assignment(argument) || argument.starts_with('-') {
                        index += 1;
                    } else {
                        break;
                    }
                }
            }
            "sudo" => {
                index += 1;
                index = skip_sudo_options(words, index);
            }
            _ => return Some(index),
        }
    }
    None
}

fn skip_sudo_options(words: &[String], mut index: usize) -> usize {
    while let Some(argument) = words.get(index) {
        if argument == "--" {
            return index + 1;
        }
        if !argument.starts_with('-') || argument == "-" {
            break;
        }
        let takes_value = matches!(
            argument.as_str(),
            "-C" | "-D" | "-g" | "-h" | "-p" | "-R" | "-T" | "-u"
        );
        index += 1;
        if takes_value {
            index += 1;
        }
    }
    index
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn executable_name(command: &str) -> &str {
    command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .trim_matches(['(', ')', '{', '}'])
}

fn git_grep_has_korean_pattern(arguments: &[String]) -> bool {
    let mut index = 0;
    while let Some(argument) = arguments.get(index) {
        if argument == "grep" {
            return arguments_have_korean_pattern(&arguments[index + 1..], SearchTool::Grep);
        }
        if argument == "--" || !argument.starts_with('-') {
            return false;
        }
        let takes_value = matches!(
            argument.as_str(),
            "-C" | "-c" | "--git-dir" | "--namespace" | "--super-prefix" | "--work-tree"
        );
        index += 1;
        if takes_value {
            index += 1;
        }
    }
    false
}

#[derive(Clone, Copy)]
enum SearchTool {
    Ripgrep,
    Grep,
}

fn arguments_have_korean_pattern(arguments: &[String], tool: SearchTool) -> bool {
    if arguments.iter().any(|argument| {
        matches!(
            argument.as_str(),
            "--help" | "--version" | "--files" | "--type-list"
        )
    }) {
        return false;
    }
    let mut explicit_patterns = Vec::new();
    let mut uses_pattern_file = false;
    let mut positional_pattern = None;
    let mut options_ended = false;
    let mut index = 0;

    while let Some(argument) = arguments.get(index) {
        if !options_ended && argument == "--" {
            options_ended = true;
            index += 1;
            continue;
        }
        if !options_ended && argument.starts_with("--") {
            let option = long_option(argument, tool);
            match option {
                LongOption::Pattern(Some(pattern)) => explicit_patterns.push(pattern),
                LongOption::Pattern(None) => {
                    if let Some(pattern) = arguments.get(index + 1) {
                        explicit_patterns.push(pattern);
                        index += 1;
                    }
                }
                LongOption::PatternFile { inline: false } => {
                    uses_pattern_file = true;
                    index += 1;
                }
                LongOption::Value { inline: false } => index += 1,
                LongOption::PatternFile { inline: true } => uses_pattern_file = true,
                LongOption::Value { inline: true } | LongOption::Flag => {}
            }
            index += 1;
            continue;
        }
        if !options_ended && argument.starts_with('-') && argument != "-" {
            let outcome = short_options(argument, arguments.get(index + 1), tool);
            explicit_patterns.extend(outcome.patterns);
            uses_pattern_file |= outcome.uses_pattern_file;
            if outcome.consumes_next {
                index += 1;
            }
            index += 1;
            continue;
        }
        if explicit_patterns.is_empty() && !uses_pattern_file {
            positional_pattern = Some(argument);
        }
        break;
    }

    explicit_patterns
        .iter()
        .any(|pattern| contains_hangul(pattern))
        || positional_pattern.is_some_and(|pattern| contains_hangul(pattern))
}

enum LongOption<'a> {
    Pattern(Option<&'a str>),
    PatternFile { inline: bool },
    Value { inline: bool },
    Flag,
}

fn long_option<'a>(argument: &'a str, tool: SearchTool) -> LongOption<'a> {
    let (name, inline_value) = argument
        .split_once('=')
        .map_or((argument, None), |(name, value)| (name, Some(value)));
    match name {
        "--regexp" => LongOption::Pattern(inline_value),
        "--file" => LongOption::PatternFile {
            inline: inline_value.is_some(),
        },
        _ if long_option_takes_value(name, tool) => LongOption::Value {
            inline: inline_value.is_some(),
        },
        _ => LongOption::Flag,
    }
}

fn long_option_takes_value(name: &str, tool: SearchTool) -> bool {
    match tool {
        SearchTool::Ripgrep => matches!(
            name,
            "--after-context"
                | "--before-context"
                | "--colors"
                | "--context"
                | "--context-separator"
                | "--dfa-size-limit"
                | "--encoding"
                | "--engine"
                | "--field-context-separator"
                | "--field-match-separator"
                | "--glob"
                | "--hostname-bin"
                | "--iglob"
                | "--ignore-file"
                | "--max-columns"
                | "--max-count"
                | "--max-depth"
                | "--max-filesize"
                | "--path-separator"
                | "--pre"
                | "--pre-glob"
                | "--regex-size-limit"
                | "--replace"
                | "--sort"
                | "--sortr"
                | "--threads"
                | "--type"
                | "--type-add"
                | "--type-clear"
                | "--type-not"
        ),
        SearchTool::Grep => matches!(
            name,
            "--after-context"
                | "--before-context"
                | "--binary-files"
                | "--context"
                | "--devices"
                | "--directories"
                | "--exclude"
                | "--exclude-dir"
                | "--exclude-from"
                | "--include"
                | "--label"
                | "--max-count"
        ),
    }
}

struct ShortOptionOutcome<'a> {
    patterns: Vec<&'a str>,
    uses_pattern_file: bool,
    consumes_next: bool,
}

fn short_options<'a>(
    argument: &'a str,
    next: Option<&'a String>,
    tool: SearchTool,
) -> ShortOptionOutcome<'a> {
    let body = argument.trim_start_matches('-');
    let mut patterns = Vec::new();
    let mut uses_pattern_file = false;
    let mut consumes_next = false;

    for (offset, option) in body.char_indices() {
        let value_start = offset + option.len_utf8();
        if option == 'e' {
            if value_start < body.len() {
                patterns.push(&body[value_start..]);
            } else if let Some(next) = next {
                patterns.push(next);
                consumes_next = true;
            }
            break;
        }
        if option == 'f' {
            uses_pattern_file = true;
            consumes_next = value_start == body.len() && next.is_some();
            break;
        }
        if short_option_takes_value(option, tool) {
            consumes_next = value_start == body.len() && next.is_some();
            break;
        }
    }

    ShortOptionOutcome {
        patterns,
        uses_pattern_file,
        consumes_next,
    }
}

const fn short_option_takes_value(option: char, tool: SearchTool) -> bool {
    match tool {
        SearchTool::Ripgrep => matches!(
            option,
            'A' | 'B' | 'C' | 'E' | 'g' | 'j' | 'm' | 'M' | 'r' | 't' | 'T'
        ),
        SearchTool::Grep => matches!(option, 'A' | 'B' | 'C' | 'm'),
    }
}

fn contains_hangul(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '\u{1100}'..='\u{11ff}'
                | '\u{3130}'..='\u{318f}'
                | '\u{a960}'..='\u{a97f}'
                | '\u{ac00}'..='\u{d7a3}'
                | '\u{d7b0}'..='\u{d7ff}'
                | '\u{ffa0}'..='\u{ffdc}'
        )
    })
}

enum ShellToken {
    Word(String),
    Separator,
}

fn tokenize_shell(command: &str) -> Option<Vec<ShellToken>> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut characters = command.chars().peekable();
    let mut quote = None;

    while let Some(character) = characters.next() {
        match quote {
            Some('\'') if character == '\'' => quote = None,
            Some('\'') => word.push(character),
            Some('"') if character == '"' => quote = None,
            Some('"') if character == '\\' => word.push(characters.next()?),
            Some('"') => word.push(character),
            Some(_) => unreachable!("only shell quote characters are stored"),
            None if matches!(character, '\'' | '"') => quote = Some(character),
            None if character == '\\' => word.push(characters.next()?),
            None if matches!(character, ';' | '|' | '&' | '\n') => {
                push_word(&mut tokens, &mut word);
                if !matches!(tokens.last(), Some(ShellToken::Separator)) {
                    tokens.push(ShellToken::Separator);
                }
            }
            None if character.is_whitespace() => push_word(&mut tokens, &mut word),
            None if matches!(character, '<' | '>') => {
                push_word(&mut tokens, &mut word);
                tokens.push(ShellToken::Separator);
            }
            None => word.push(character),
        }
    }
    if quote.is_some() {
        return None;
    }
    push_word(&mut tokens, &mut word);
    Some(tokens)
}

fn push_word(tokens: &mut Vec<ShellToken>, word: &mut String) {
    if !word.is_empty() {
        tokens.push(ShellToken::Word(std::mem::take(word)));
    }
}

#[derive(Debug)]
pub enum AgentHookError {
    Read(io::Error),
    InputTooLarge,
    Json(serde_json::Error),
    MissingCommand,
    UnsupportedEvent(String),
    WriteJson(serde_json::Error),
    Write(io::Error),
}

impl Display for AgentHookError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "failed to read agent hook input: {error}"),
            Self::InputTooLarge => {
                formatter.write_str("agent hook input exceeds the 1048576-byte size limit")
            }
            Self::Json(error) => write!(formatter, "agent hook input is not valid JSON: {error}"),
            Self::MissingCommand => {
                formatter.write_str("agent hook input is missing tool_input.command")
            }
            Self::UnsupportedEvent(event) => {
                write!(formatter, "unsupported agent hook event: {event}")
            }
            Self::WriteJson(error) => {
                write!(formatter, "failed to encode agent hook output: {error}")
            }
            Self::Write(error) => write!(formatter, "failed to write agent hook output: {error}"),
        }
    }
}

impl Error for AgentHookError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) | Self::Write(error) => Some(error),
            Self::Json(error) | Self::WriteJson(error) => Some(error),
            Self::InputTooLarge | Self::MissingCommand | Self::UnsupportedEvent(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
