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
    for relative in [
        ".claude/settings.json",
        ".codex/hooks.json",
        ".gemini/settings.json",
    ] {
        let contents = fs::read_to_string(root.path().join(relative)).unwrap();
        assert!(contents.contains("kfind --agent-hook"));
    }
    assert!(stderr.contains("Installed Claude Code hook"));
    assert!(stderr.contains("Installed Codex hook"));
    assert!(stderr.contains("Installed Gemini CLI hook"));
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

#[test]
fn rerun_preserves_existing_agent_settings_and_does_not_duplicate_the_hook() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--init", "--agent", "codex"]).unwrap();
    let config = root.path().join(".codex/hooks.json");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(
        &config,
        r#"{
  "theme": "dark",
  "hooks": {
    "PreToolUse": [{
      "matcher": "Read",
      "hooks": [{"type": "command", "command": "existing-hook"}]
    }]
  }
}
"#,
    )
    .unwrap();

    run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap();
    let (_, stderr) = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap();

    let contents = fs::read_to_string(config).unwrap();
    let document: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(document["theme"], "dark");
    assert_eq!(contents.matches("existing-hook").count(), 1);
    assert_eq!(contents.matches("kfind --agent-hook").count(), 1);
    assert!(stderr.contains("Unchanged Codex hook"));
}

#[test]
fn invalid_agent_settings_prevent_all_selected_installations() {
    let root = tempdir().unwrap();
    let args =
        Args::try_parse_from(["kfind", "--init", "--agent", "codex", "--agent", "gemini"]).unwrap();
    let invalid = root.path().join(".gemini/settings.json");
    fs::create_dir_all(invalid.parent().unwrap()).unwrap();
    fs::write(&invalid, "{").unwrap();

    let error = run_at(&args, &[], root.path(), SkillSource::Embedded).unwrap_err();

    assert!(matches!(
        error,
        InitError::ParseAgentConfig { path, .. } if path == invalid
    ));
    assert!(!root.path().join(".agents").exists());
    assert!(!root.path().join(".codex").exists());
    assert!(!root.path().join(".gemini/skills").exists());
    assert_eq!(fs::read_to_string(invalid).unwrap(), "{");
}

#[cfg(unix)]
#[test]
fn homebrew_opt_link_tracks_keg_switches() {
    let root = tempdir().unwrap();
    let prefix = tempdir().unwrap();
    let first_keg = prefix.path().join("Cellar/kfind/0.2.0");
    let second_keg = prefix.path().join("Cellar/kfind/0.2.1");
    let first_source = first_keg.join("share/kfind/skills/kfind/SKILL.md");
    let second_source = second_keg.join("share/kfind/skills/kfind/SKILL.md");
    fs::create_dir_all(first_source.parent().unwrap()).unwrap();
    fs::create_dir_all(second_source.parent().unwrap()).unwrap();
    fs::write(&first_source, SKILL_CONTENT).unwrap();
    let next_release = format!("{SKILL_CONTENT}\n<!-- next release -->\n");
    fs::write(&second_source, &next_release).unwrap();

    let opt = prefix.path().join("opt/kfind");
    fs::create_dir_all(opt.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&first_keg, &opt).unwrap();
    let stable_source = opt.join("share/kfind/skills/kfind/SKILL.md");
    let args = Args::try_parse_from(["kfind", "--init", "--agent", "gemini"]).unwrap();

    run_at(
        &args,
        &[],
        root.path(),
        SkillSource::Homebrew(stable_source.clone()),
    )
    .unwrap();

    let installed = root.path().join(".gemini/skills/kfind/SKILL.md");
    assert_eq!(fs::read_link(&installed).unwrap(), stable_source);
    assert_eq!(fs::read_to_string(&installed).unwrap(), SKILL_CONTENT);

    fs::remove_file(&opt).unwrap();
    std::os::unix::fs::symlink(&second_keg, &opt).unwrap();

    assert_eq!(fs::read_to_string(installed).unwrap(), next_release);
}
