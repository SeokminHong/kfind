use std::io::Cursor;

use clap::Parser;
use tempfile::tempdir;

use super::*;

fn run_at(
    args: &Args,
    input: &[u8],
    root: &Path,
    source: SkillSource,
) -> Result<(String, String), InitError> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    run_init_in_directory(
        args,
        Language::English,
        &mut Cursor::new(input),
        &mut stdout,
        &mut stderr,
        false,
        root,
        source,
    )?;
    Ok((
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    ))
}

#[test]
fn explicit_agents_install_the_same_managed_skill() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from([
        "kfind",
        "--init",
        "--agent",
        "claude-code",
        "--agent",
        "codex",
        "--agent",
        "gemini",
    ])
    .unwrap();

    let (stdout, stderr) = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap();

    assert!(stdout.is_empty());
    for relative in [
        ".claude/skills/kfind/SKILL.md",
        ".agents/skills/kfind/SKILL.md",
        ".gemini/skills/kfind/SKILL.md",
    ] {
        assert_eq!(
            fs::read_to_string(root.path().join(relative)).unwrap(),
            SKILL_CONTENT
        );
    }
    assert!(stderr.contains("Installed Claude Code skill"));
    assert!(stderr.contains("Installed Codex skill"));
    assert!(stderr.contains("Installed Gemini CLI skill"));
}

#[test]
fn custom_writes_only_the_skill_to_stdout() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--init", "--agent", "custom"]).unwrap();

    let (stdout, stderr) = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap();

    assert_eq!(stdout, SKILL_CONTENT);
    assert!(stderr.is_empty());
}

#[test]
fn non_terminal_input_accepts_whitespace_and_deduplicates_agents() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--init"]).unwrap();

    let (_, stderr) = run_at(
        &args,
        b"codex\nclaude-code codex\n",
        root.path(),
        SkillSource::Embedded,
    )
    .unwrap();

    assert!(root.path().join(".agents/skills/kfind/SKILL.md").is_file());
    assert!(root.path().join(".claude/skills/kfind/SKILL.md").is_file());
    assert_eq!(stderr.matches("Codex skill").count(), 1);
}

#[test]
fn invalid_non_terminal_input_changes_nothing() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--init"]).unwrap();

    let error = run_at(
        &args,
        b"codex unknown\n",
        root.path(),
        SkillSource::Embedded,
    )
    .unwrap_err();

    assert!(matches!(error, InitError::UnknownAgent { value } if value == "unknown"));
    assert!(!root.path().join(".agents").exists());
}

#[test]
fn empty_non_terminal_input_is_an_error() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--init"]).unwrap();

    let error = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap_err();

    assert!(matches!(error, InitError::EmptyInput));
}

#[test]
fn rerun_updates_managed_skill_and_preserves_unmanaged_skill() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--init", "--agent", "codex"]).unwrap();
    let path = root.path().join(".agents/skills/kfind/SKILL.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, format!("{MANAGED_MARKER}\nold\n")).unwrap();

    let (_, stderr) = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), SKILL_CONTENT);
    assert!(stderr.contains("Updated Codex skill"));

    fs::write(&path, "user-authored\n").unwrap();
    let error = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap_err();
    assert!(matches!(error, InitError::UnmanagedDestination(conflict) if conflict == path));
    assert_eq!(fs::read_to_string(&path).unwrap(), "user-authored\n");
}

#[cfg(unix)]
#[test]
fn homebrew_source_link_observes_source_updates() {
    let root = tempdir().unwrap();
    let package = tempdir().unwrap();
    let source = package.path().join("SKILL.md");
    fs::write(&source, SKILL_CONTENT).unwrap();
    let args = Args::try_parse_from(["kfind", "--init", "--agent", "gemini"]).unwrap();

    run_at(
        &args,
        &[],
        root.path(),
        SkillSource::Homebrew(source.clone()),
    )
    .unwrap();

    let installed = root.path().join(".gemini/skills/kfind/SKILL.md");
    assert_eq!(fs::read_link(&installed).unwrap(), source);
    fs::write(&source, format!("{SKILL_CONTENT}\nupdated\n")).unwrap();
    assert!(
        fs::read_to_string(installed)
            .unwrap()
            .ends_with("updated\n")
    );
}
