use std::path::Path;
use std::process::{Command, Output, Stdio};

fn run(locale_variables: &[(&str, &str)], arguments: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kfind"));
    for name in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        command.env_remove(name);
    }
    for &(name, value) in locale_variables {
        command.env(name, value);
    }
    command.args(arguments).output().unwrap()
}

#[test]
fn help_uses_the_locale_and_success_stream() {
    let korean = run(&[("LANG", "ko_KR.UTF-8")], &["--help"]);
    assert!(korean.status.success());
    assert!(korean.stderr.is_empty());
    let korean = String::from_utf8(korean.stdout).unwrap();
    assert!(korean.contains("한국어 표제어와 활용형"));
    assert!(korean.contains("사용법:"));

    let english = run(&[("LANG", "C")], &["--help"]);
    assert!(english.status.success());
    assert!(english.stderr.is_empty());
    let english = String::from_utf8(english.stdout).unwrap();
    assert!(english.contains("Fast Korean lemma"));
    assert!(english.contains("Usage:"));
}

#[test]
fn argument_errors_use_the_locale_and_error_stream() {
    let output = run(&[("LC_MESSAGES", "ko")], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("오류: 필수 인수가 입력되지 않았습니다"));
    assert!(stderr.contains("사용법:"));
}

#[test]
fn higher_priority_non_korean_locale_selects_english() {
    let output = run(
        &[("LC_ALL", "C"), ("LC_MESSAGES", "ko"), ("LANG", "ko")],
        &["--help"],
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Usage:"));
    assert!(!stdout.contains("사용법:"));
}

#[test]
fn runtime_errors_are_localized_after_parsing() {
    let output = run(
        &[("LC_ALL", "ko_KR.UTF-8")],
        &["--literal", "--pos", "verb", "걸어"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("함께 사용할 수 없습니다")
    );
}

#[test]
fn initialization_errors_escape_terminal_control_characters() {
    let output = run(
        &[("LC_ALL", "C")],
        &["--data-dir", "bad\u{1b}[31m\npath", "걷다"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.contains(&0x1b));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(r"bad\u{001B}[31m\npath"));
}

#[test]
fn agent_init_uses_localized_errors_and_clean_custom_output() {
    let missing = run(&[("LC_ALL", "ko_KR.UTF-8")], &["--init"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert!(
        String::from_utf8(missing.stderr)
            .unwrap()
            .contains("agent가 입력되지 않았습니다")
    );

    let custom = run(&[("LC_ALL", "C")], &["--init", "--agent", "custom"]);
    assert!(custom.status.success());
    assert!(custom.stderr.is_empty());
    let stdout = String::from_utf8(custom.stdout).unwrap();
    assert!(stdout.starts_with("---\nname: kfind\n"));
    assert!(stdout.contains("managed by kfind init"));
}

#[test]
fn locale_reaches_explain_output() {
    let output = run(&[("LANG", "ko_KR.UTF-8")], &["걷다", "--explain-query"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("쿼리: 걷다"));
    assert!(stdout.contains("요소[0]:"));
    assert!(!stdout.contains("query:"));
}

#[test]
fn closed_non_terminal_stdout_remains_a_normal_exit() {
    let readme = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kfind"))
        .args(["--literal", "kfind"])
        .arg(readme)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}
