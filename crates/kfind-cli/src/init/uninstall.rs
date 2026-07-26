use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::hook_config::{self, HookRemovalAction};
use super::{
    AgentArg, InitError, IntegrationOperation, MANAGED_MARKER, is_managed_homebrew_link,
    selected_agents, write_status,
};
use crate::{Args, Language};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SkillRemovalAction {
    Remove,
    Unchanged,
}

struct SkillRemoval {
    agent: AgentArg,
    path: PathBuf,
    action: SkillRemovalAction,
}

pub fn run_uninstall_with_io<R, W, E>(
    args: &Args,
    language: Language,
    mut stdin: R,
    _stdout: &mut W,
    stderr: &mut E,
    interactive_terminal: bool,
) -> Result<(), InitError>
where
    R: Read,
    W: Write,
    E: Write,
{
    let root = env::current_dir().map_err(InitError::CurrentDirectory)?;
    run_uninstall_in_directory(
        args,
        language,
        &mut stdin,
        stderr,
        interactive_terminal,
        &root,
    )
}

fn run_uninstall_in_directory<R, E>(
    args: &Args,
    language: Language,
    stdin: &mut R,
    stderr: &mut E,
    interactive_terminal: bool,
    root: &Path,
) -> Result<(), InitError>
where
    R: Read,
    E: Write,
{
    let agents = selected_agents(
        args,
        stdin,
        language,
        interactive_terminal,
        IntegrationOperation::Uninstall,
    )?;
    if agents.is_empty() {
        write_status(
            stderr,
            language.select("No agents selected.", "선택한 agent가 없습니다."),
        )?;
        return Ok(());
    }

    let mut skill_removals = Vec::new();
    let mut hook_removals = Vec::new();
    for agent in agents {
        let path = agent.skill_path(root);
        skill_removals.push(prepare_skill_removal(agent, path)?);
        if let Some(removal) = hook_config::prepare_hook_removal(root, agent)? {
            hook_removals.push(removal);
        }
    }

    for removal in &hook_removals {
        removal.apply()?;
        write_hook_removal_status(
            stderr,
            language,
            removal.agent,
            &removal.path,
            removal.action,
        )?;
    }
    for removal in &skill_removals {
        removal.apply()?;
        write_skill_removal_status(
            stderr,
            language,
            removal.agent,
            &removal.path,
            removal.action,
        )?;
    }

    Ok(())
}

impl SkillRemoval {
    fn apply(&self) -> Result<(), InitError> {
        if self.action == SkillRemovalAction::Remove {
            fs::remove_file(&self.path).map_err(|source| InitError::Remove {
                path: self.path.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

fn prepare_skill_removal(agent: AgentArg, path: PathBuf) -> Result<SkillRemoval, InitError> {
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(SkillRemoval {
                agent,
                path,
                action: SkillRemovalAction::Unchanged,
            });
        }
        Err(source) => {
            return Err(InitError::Inspect {
                path: path.clone(),
                source,
            });
        }
    };

    if metadata.file_type().is_symlink() {
        let target = fs::read_link(&path).map_err(|source| InitError::Inspect {
            path: path.clone(),
            source,
        })?;
        if !is_managed_homebrew_link(&target) {
            return Err(InitError::UnmanagedDestination(path));
        }
    } else if metadata.is_file() {
        let contents = fs::read_to_string(&path).map_err(|source| InitError::Inspect {
            path: path.clone(),
            source,
        })?;
        if !contents.contains(MANAGED_MARKER) {
            return Err(InitError::UnmanagedDestination(path));
        }
    } else {
        return Err(InitError::UnmanagedDestination(path));
    }

    Ok(SkillRemoval {
        agent,
        path,
        action: SkillRemovalAction::Remove,
    })
}

fn write_skill_removal_status(
    writer: &mut impl Write,
    language: Language,
    agent: AgentArg,
    path: &Path,
    action: SkillRemovalAction,
) -> Result<(), InitError> {
    let message = match (language, action) {
        (Language::English, SkillRemovalAction::Remove) => {
            format!("Removed {} skill: {}", agent.display_name(), path.display())
        }
        (Language::English, SkillRemovalAction::Unchanged) => {
            format!(
                "{} skill was not installed: {}",
                agent.display_name(),
                path.display()
            )
        }
        (Language::Korean, SkillRemovalAction::Remove) => {
            format!(
                "{} skill을 제거했습니다: {}",
                agent.display_name(),
                path.display()
            )
        }
        (Language::Korean, SkillRemovalAction::Unchanged) => {
            format!(
                "{} skill이 설치되어 있지 않습니다: {}",
                agent.display_name(),
                path.display()
            )
        }
    };
    write_status(writer, &message)
}

fn write_hook_removal_status(
    writer: &mut impl Write,
    language: Language,
    agent: AgentArg,
    path: &Path,
    action: HookRemovalAction,
) -> Result<(), InitError> {
    let message = match (language, action) {
        (Language::English, HookRemovalAction::Remove | HookRemovalAction::Update) => {
            format!("Removed {} hook: {}", agent.display_name(), path.display())
        }
        (Language::English, HookRemovalAction::Unchanged) => {
            format!(
                "{} hook was not installed: {}",
                agent.display_name(),
                path.display()
            )
        }
        (Language::Korean, HookRemovalAction::Remove | HookRemovalAction::Update) => {
            format!(
                "{} hook을 제거했습니다: {}",
                agent.display_name(),
                path.display()
            )
        }
        (Language::Korean, HookRemovalAction::Unchanged) => {
            format!(
                "{} hook이 설치되어 있지 않습니다: {}",
                agent.display_name(),
                path.display()
            )
        }
    };
    write_status(writer, &message)
}

#[cfg(test)]
mod tests;
