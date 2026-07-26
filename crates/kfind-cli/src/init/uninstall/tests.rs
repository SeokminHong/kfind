use std::io::Cursor;

use clap::Parser;
use tempfile::tempdir;

use super::super::{SKILL_CONTENT, SkillSource, run_init_in_directory};
use super::*;

fn run_init_at(args: &Args, root: &Path) {
    run_init_in_directory(
        args,
        Language::English,
        &mut Cursor::new([]),
        &mut Vec::new(),
        &mut Vec::new(),
        false,
        root,
        SkillSource::Embedded,
    )
    .unwrap();
}

fn run_uninstall_at(args: &Args, root: &Path) -> Result<(String, String), InitError> {
    let mut stderr = Vec::new();
    run_uninstall_in_directory(
        args,
        Language::English,
        &mut Cursor::new([]),
        &mut stderr,
        false,
        root,
    )?;
    Ok((String::new(), String::from_utf8(stderr).unwrap()))
}

#[test]
fn removes_managed_skills_and_hook_only_configuration_files() {
    let root = tempdir().unwrap();
    let agents = ["claude-code", "codex", "gemini"];
    let mut init_values = vec!["kfind", "--init"];
    let mut uninstall_values = vec!["kfind", "--uninstall"];
    for agent in agents {
        init_values.extend(["--agent", agent]);
        uninstall_values.extend(["--agent", agent]);
    }
    run_init_at(&Args::try_parse_from(init_values).unwrap(), root.path());

    let (stdout, stderr) = run_uninstall_at(
        &Args::try_parse_from(uninstall_values).unwrap(),
        root.path(),
    )
    .unwrap();

    assert!(stdout.is_empty());
    for relative in [
        ".claude/skills/kfind/SKILL.md",
        ".agents/skills/kfind/SKILL.md",
        ".gemini/skills/kfind/SKILL.md",
        ".claude/settings.json",
        ".codex/hooks.json",
        ".gemini/settings.json",
    ] {
        assert!(!root.path().join(relative).exists(), "{relative} remains");
    }
    for agent in ["Claude Code", "Codex", "Gemini CLI"] {
        assert!(stderr.contains(&format!("Removed {agent} hook")));
        assert!(stderr.contains(&format!("Removed {agent} skill")));
    }
}

#[test]
fn removes_only_the_managed_hook_from_existing_settings() {
    let root = tempdir().unwrap();
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
    let init_args = Args::try_parse_from(["kfind", "--init", "--agent", "codex"]).unwrap();
    run_init_at(&init_args, root.path());

    let uninstall_args =
        Args::try_parse_from(["kfind", "--uninstall", "--agent", "codex"]).unwrap();
    run_uninstall_at(&uninstall_args, root.path()).unwrap();

    let contents = fs::read_to_string(&config).unwrap();
    let document: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(document["theme"], "dark");
    assert_eq!(contents.matches("existing-hook").count(), 1);
    assert!(!contents.contains("kfind --agent-hook"));
    assert!(!root.path().join(".agents/skills/kfind/SKILL.md").exists());
}

#[test]
fn unmanaged_skill_prevents_all_selected_removals() {
    let root = tempdir().unwrap();
    let init_args =
        Args::try_parse_from(["kfind", "--init", "--agent", "codex", "--agent", "gemini"]).unwrap();
    run_init_at(&init_args, root.path());
    let codex_skill = root.path().join(".agents/skills/kfind/SKILL.md");
    fs::write(&codex_skill, "user-authored\n").unwrap();

    let uninstall_args = Args::try_parse_from([
        "kfind",
        "--uninstall",
        "--agent",
        "codex",
        "--agent",
        "gemini",
    ])
    .unwrap();
    let error = run_uninstall_at(&uninstall_args, root.path()).unwrap_err();

    assert!(matches!(
        error,
        InitError::UnmanagedDestination(path) if path == codex_skill
    ));
    assert_eq!(fs::read_to_string(codex_skill).unwrap(), "user-authored\n");
    assert_eq!(
        fs::read_to_string(root.path().join(".gemini/skills/kfind/SKILL.md")).unwrap(),
        SKILL_CONTENT
    );
    assert!(
        fs::read_to_string(root.path().join(".codex/hooks.json"))
            .unwrap()
            .contains("kfind --agent-hook")
    );
}

#[test]
fn invalid_hook_configuration_prevents_all_selected_removals() {
    let root = tempdir().unwrap();
    let init_args =
        Args::try_parse_from(["kfind", "--init", "--agent", "codex", "--agent", "gemini"]).unwrap();
    run_init_at(&init_args, root.path());
    let invalid = root.path().join(".gemini/settings.json");
    fs::write(&invalid, "{").unwrap();

    let uninstall_args = Args::try_parse_from([
        "kfind",
        "--uninstall",
        "--agent",
        "codex",
        "--agent",
        "gemini",
    ])
    .unwrap();
    let error = run_uninstall_at(&uninstall_args, root.path()).unwrap_err();

    assert!(matches!(
        error,
        InitError::ParseAgentConfig { path, .. } if path == invalid
    ));
    assert!(root.path().join(".agents/skills/kfind/SKILL.md").exists());
    assert!(root.path().join(".gemini/skills/kfind/SKILL.md").exists());
    assert!(
        fs::read_to_string(root.path().join(".codex/hooks.json"))
            .unwrap()
            .contains("kfind --agent-hook")
    );
}

#[test]
fn missing_integration_is_an_idempotent_success() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--uninstall", "--agent", "codex"]).unwrap();

    let (_, stderr) = run_uninstall_at(&args, root.path()).unwrap();

    assert!(stderr.contains("Codex hook was not installed"));
    assert!(stderr.contains("Codex skill was not installed"));
}

#[test]
fn custom_output_is_not_an_uninstall_target() {
    let root = tempdir().unwrap();
    let args = Args::try_parse_from(["kfind", "--uninstall", "--agent", "custom"]).unwrap();

    let error = run_uninstall_at(&args, root.path()).unwrap_err();

    assert!(matches!(error, InitError::CustomCannotBeUninstalled));
    assert!(root.path().read_dir().unwrap().next().is_none());
}

#[cfg(unix)]
#[test]
fn removes_a_broken_managed_homebrew_link() {
    let root = tempdir().unwrap();
    let skill = root.path().join(".agents/skills/kfind/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(
        "/opt/homebrew/opt/kfind/share/kfind/skills/kfind/SKILL.md",
        &skill,
    )
    .unwrap();
    let args = Args::try_parse_from(["kfind", "--uninstall", "--agent", "codex"]).unwrap();

    run_uninstall_at(&args, root.path()).unwrap();

    assert!(fs::symlink_metadata(skill).is_err());
}
