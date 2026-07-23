use std::io::Cursor;

use super::*;

fn codex_input(command: &str) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
    })
    .to_string()
}

fn gemini_input(command: &str) -> String {
    serde_json::json!({
        "hook_event_name": "BeforeTool",
        "tool_name": "run_shell_command",
        "tool_input": { "command": command },
    })
    .to_string()
}

fn run_hook(input: &str) -> Result<String, AgentHookError> {
    let mut output = Vec::new();
    run_agent_hook_with_io(&mut Cursor::new(input), &mut output)?;
    Ok(String::from_utf8(output).unwrap())
}

#[test]
fn blocks_korean_patterns_for_supported_search_commands() {
    for command in [
        "rg '사용자' crates",
        "/usr/local/bin/rg --regexp=검증하다 docs",
        "grep -Rn -e '권한' .",
        "egrep '검증' docs",
        "fgrep --regexp '한글' README.md",
        "git grep '검색'",
        "git -C repository grep -e사용자 -- '*.rs'",
        "env LC_ALL=C rg -n 사용자 crates",
        "sudo -u nobody grep 사용자 file",
        "printf data | rg 사용자 && echo done",
    ] {
        assert!(
            contains_korean_literal_search(command),
            "expected blocked command: {command}"
        );
    }
}

#[test]
fn permits_non_pattern_korean_arguments_and_kfind() {
    for command in [
        "rg TODO '한국어 문서'",
        "rg --glob '*한글*' TODO .",
        "rg --type-add '문서:*.한글' TODO .",
        "grep --include='*한글*' TODO .",
        "grep -f 한글-patterns.txt docs",
        "git grep TODO -- '한국어 문서'",
        "rg --files '한국어 문서'",
        "kfind --embedded --boundary any --json 사용자 crates",
        "rg 'user|사용자",
        "echo rg 사용자",
        "rg --files | grep README",
    ] {
        assert!(
            !contains_korean_literal_search(command),
            "expected allowed command: {command}"
        );
    }
}

#[test]
fn quoted_shell_operators_remain_part_of_the_pattern() {
    assert!(contains_korean_literal_search("rg 'user|사용자' crates"));
    assert!(contains_korean_literal_search(
        "echo ready; rg '검증 && 확인' docs"
    ));
}

#[test]
fn codex_and_claude_receive_structured_pre_tool_denials() {
    let output = run_hook(&codex_input("rg 사용자 crates")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("kfind")
    );
}

#[test]
fn gemini_receives_allow_and_deny_decisions() {
    let denied: serde_json::Value =
        serde_json::from_str(&run_hook(&gemini_input("grep 검색 docs")).unwrap()).unwrap();
    let allowed: serde_json::Value =
        serde_json::from_str(&run_hook(&gemini_input("rg TODO docs")).unwrap()).unwrap();

    assert_eq!(denied["decision"], "deny");
    assert_eq!(allowed["decision"], "allow");
}

#[test]
fn codex_allow_path_is_silent() {
    assert_eq!(run_hook(&codex_input("rg TODO crates")).unwrap(), "");
}

#[test]
fn rejects_invalid_hook_payloads() {
    let error = run_hook("{").unwrap_err();
    assert!(matches!(error, AgentHookError::Json(_)));

    let missing = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {},
    })
    .to_string();
    assert!(matches!(
        run_hook(&missing).unwrap_err(),
        AgentHookError::MissingCommand
    ));
}
