use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{AgentArg, InitError, InstallAction, NEXT_TEMP_FILE, Ordering};

const HOOK_COMMAND: &str = "kfind --agent-hook";

pub(super) struct HookInstallation {
    pub(super) agent: AgentArg,
    pub(super) path: PathBuf,
    pub(super) action: InstallAction,
    contents: Option<String>,
    permissions: Option<fs::Permissions>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HookRemovalAction {
    Remove,
    Update,
    Unchanged,
}

pub(super) struct HookRemoval {
    pub(super) agent: AgentArg,
    pub(super) path: PathBuf,
    pub(super) action: HookRemovalAction,
    contents: Option<String>,
    permissions: Option<fs::Permissions>,
}

impl HookInstallation {
    pub(super) fn write(&self) -> Result<(), InitError> {
        let Some(contents) = &self.contents else {
            return Ok(());
        };
        let parent = self
            .path
            .parent()
            .ok_or_else(|| InitError::InvalidAgentConfig {
                path: self.path.clone(),
                reason: "configuration path has no parent directory",
            })?;
        fs::create_dir_all(parent).map_err(|source| InitError::CreateAgentConfigDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
        let temporary = temporary_path(parent, &self.path);
        let result =
            write_temporary_config(&temporary, contents.as_bytes(), self.permissions.as_ref());
        if let Err(source) = result {
            let _ = fs::remove_file(&temporary);
            return Err(InitError::WriteAgentConfig {
                path: temporary,
                source,
            });
        }
        if let Err(source) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(InitError::WriteAgentConfig {
                path: self.path.clone(),
                source,
            });
        }
        Ok(())
    }
}

impl HookRemoval {
    pub(super) fn apply(&self) -> Result<(), InitError> {
        match self.action {
            HookRemovalAction::Remove => {
                fs::remove_file(&self.path).map_err(|source| InitError::RemoveAgentConfig {
                    path: self.path.clone(),
                    source,
                })
            }
            HookRemovalAction::Update => {
                let contents = self
                    .contents
                    .as_ref()
                    .expect("updated hook configuration must have contents");
                let parent = self
                    .path
                    .parent()
                    .ok_or_else(|| InitError::InvalidAgentConfig {
                        path: self.path.clone(),
                        reason: "configuration path has no parent directory",
                    })?;
                let temporary = temporary_path(parent, &self.path);
                let result = write_temporary_config(
                    &temporary,
                    contents.as_bytes(),
                    self.permissions.as_ref(),
                );
                if let Err(source) = result {
                    let _ = fs::remove_file(&temporary);
                    return Err(InitError::RemoveAgentConfig {
                        path: temporary,
                        source,
                    });
                }
                if let Err(source) = fs::rename(&temporary, &self.path) {
                    let _ = fs::remove_file(&temporary);
                    return Err(InitError::RemoveAgentConfig {
                        path: self.path.clone(),
                        source,
                    });
                }
                Ok(())
            }
            HookRemovalAction::Unchanged => Ok(()),
        }
    }
}

pub(super) fn prepare_hook_installation(
    root: &Path,
    agent: AgentArg,
) -> Result<Option<HookInstallation>, InitError> {
    let Some(contracts) = AgentHookContracts::for_agent(agent) else {
        return Ok(None);
    };
    let path = root.join(contracts.path);
    let (existing, permissions, initial_action) = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            let contents =
                fs::read_to_string(&path).map_err(|source| InitError::InspectAgentConfig {
                    path: path.clone(),
                    source,
                })?;
            (
                Some(contents),
                Some(metadata.permissions()),
                InstallAction::Update,
            )
        }
        Ok(_) => return Err(InitError::UnmanagedAgentConfig(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            (None, None, InstallAction::Install)
        }
        Err(source) => {
            return Err(InitError::InspectAgentConfig { path, source });
        }
    };

    let mut document = match existing.as_deref() {
        Some(contents) => {
            serde_json::from_str(contents).map_err(|source| InitError::ParseAgentConfig {
                path: path.clone(),
                source,
            })?
        }
        None => json!({}),
    };
    let mut changed = false;
    for contract in contracts.hooks {
        changed |= merge_hook(&mut document, *contract).map_err(|reason| {
            InitError::InvalidAgentConfig {
                path: path.clone(),
                reason,
            }
        })?;
    }
    let (action, contents) = if changed {
        let mut contents = serde_json::to_string_pretty(&document).map_err(|source| {
            InitError::EncodeAgentConfig {
                path: path.clone(),
                source,
            }
        })?;
        contents.push('\n');
        (initial_action, Some(contents))
    } else {
        (InstallAction::Unchanged, None)
    };

    Ok(Some(HookInstallation {
        agent,
        path,
        action,
        contents,
        permissions,
    }))
}

pub(super) fn prepare_hook_removal(
    root: &Path,
    agent: AgentArg,
) -> Result<Option<HookRemoval>, InitError> {
    let Some(contracts) = AgentHookContracts::for_agent(agent) else {
        return Ok(None);
    };
    let path = root.join(contracts.path);
    let (contents, permissions) = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            let contents =
                fs::read_to_string(&path).map_err(|source| InitError::InspectAgentConfig {
                    path: path.clone(),
                    source,
                })?;
            (contents, metadata.permissions())
        }
        Ok(_) => return Err(InitError::UnmanagedAgentConfig(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Some(HookRemoval {
                agent,
                path,
                action: HookRemovalAction::Unchanged,
                contents: None,
                permissions: None,
            }));
        }
        Err(source) => {
            return Err(InitError::InspectAgentConfig { path, source });
        }
    };

    let mut document =
        serde_json::from_str(&contents).map_err(|source| InitError::ParseAgentConfig {
            path: path.clone(),
            source,
        })?;
    let mut changed = false;
    for contract in contracts.hooks {
        changed |= remove_hook(&mut document, *contract).map_err(|reason| {
            InitError::InvalidAgentConfig {
                path: path.clone(),
                reason,
            }
        })?;
    }
    let remove_file = document.as_object().is_some_and(serde_json::Map::is_empty);
    let (action, contents) = if !changed {
        (HookRemovalAction::Unchanged, None)
    } else if remove_file {
        (HookRemovalAction::Remove, None)
    } else {
        let mut contents = serde_json::to_string_pretty(&document).map_err(|source| {
            InitError::EncodeAgentConfig {
                path: path.clone(),
                source,
            }
        })?;
        contents.push('\n');
        (HookRemovalAction::Update, Some(contents))
    };

    Ok(Some(HookRemoval {
        agent,
        path,
        action,
        contents,
        permissions: Some(permissions),
    }))
}

#[derive(Clone, Copy)]
struct HookContract {
    event: &'static str,
    matcher: Option<&'static str>,
    handler_name: Option<&'static str>,
}

#[derive(Clone, Copy)]
struct AgentHookContracts {
    path: &'static str,
    hooks: &'static [HookContract],
}

const CLAUDE_HOOKS: [HookContract; 2] = [
    HookContract {
        event: "SessionStart",
        matcher: None,
        handler_name: None,
    },
    HookContract {
        event: "PreToolUse",
        matcher: Some("Bash"),
        handler_name: None,
    },
];
const CODEX_HOOKS: [HookContract; 2] = CLAUDE_HOOKS;
const GEMINI_HOOKS: [HookContract; 2] = [
    HookContract {
        event: "SessionStart",
        matcher: None,
        handler_name: Some("kfind-agent-instructions"),
    },
    HookContract {
        event: "BeforeTool",
        matcher: Some("run_shell_command"),
        handler_name: Some("kfind-korean-search"),
    },
];

impl AgentHookContracts {
    const fn for_agent(agent: AgentArg) -> Option<Self> {
        match agent {
            AgentArg::ClaudeCode => Some(Self {
                path: ".claude/settings.json",
                hooks: &CLAUDE_HOOKS,
            }),
            AgentArg::Codex => Some(Self {
                path: ".codex/hooks.json",
                hooks: &CODEX_HOOKS,
            }),
            AgentArg::Gemini => Some(Self {
                path: ".gemini/settings.json",
                hooks: &GEMINI_HOOKS,
            }),
            AgentArg::Custom => None,
        }
    }
}

impl HookContract {
    fn handler(&self) -> Value {
        match self.handler_name {
            Some(name) => json!({
                "name": name,
                "type": "command",
                "command": HOOK_COMMAND,
            }),
            None => json!({
                "type": "command",
                "command": HOOK_COMMAND,
            }),
        }
    }

    fn group(&self) -> Value {
        let mut group = serde_json::Map::new();
        if let Some(matcher) = self.matcher {
            group.insert("matcher".to_owned(), json!(matcher));
        }
        group.insert("hooks".to_owned(), json!([self.handler()]));
        Value::Object(group)
    }
}

fn merge_hook(document: &mut Value, contract: HookContract) -> Result<bool, &'static str> {
    let root = document
        .as_object_mut()
        .ok_or("configuration root must be a JSON object")?;
    if !root.contains_key("hooks") {
        root.insert("hooks".to_owned(), json!({}));
    }
    let hooks = root
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .ok_or("`hooks` must be a JSON object")?;
    if !hooks.contains_key(contract.event) {
        hooks.insert(contract.event.to_owned(), json!([]));
    }
    let groups = hooks
        .get_mut(contract.event)
        .and_then(Value::as_array_mut)
        .ok_or("hook event must be a JSON array")?;
    let canonical = contract.handler();

    let mut managed_count = 0;
    let mut canonical_count = 0;
    for group in groups.iter() {
        let group = group
            .as_object()
            .ok_or("hook event entries must be JSON objects")?;
        let handlers = group
            .get("hooks")
            .and_then(Value::as_array)
            .ok_or("hook group `hooks` must be a JSON array")?;
        for handler in handlers {
            let handler = handler
                .as_object()
                .ok_or("hook handlers must be JSON objects")?;
            if handler
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| command == HOOK_COMMAND)
            {
                managed_count += 1;
                if group_matches_contract(group, contract)
                    && Value::Object(handler.clone()) == canonical
                {
                    canonical_count += 1;
                }
            }
        }
    }
    if managed_count == 1 && canonical_count == 1 {
        return Ok(false);
    }

    for group in groups.iter_mut() {
        let group = group
            .as_object_mut()
            .ok_or("hook event entries must be JSON objects")?;
        let handlers = group
            .get_mut("hooks")
            .and_then(Value::as_array_mut)
            .ok_or("hook group `hooks` must be a JSON array")?;
        handlers.retain(|handler| {
            handler
                .get("command")
                .and_then(Value::as_str)
                .is_none_or(|command| command != HOOK_COMMAND)
        });
    }
    groups.retain(|group| {
        group
            .get("hooks")
            .and_then(Value::as_array)
            .is_none_or(|handlers| !handlers.is_empty())
    });
    groups.push(contract.group());
    Ok(true)
}

fn remove_hook(document: &mut Value, contract: HookContract) -> Result<bool, &'static str> {
    let root = document
        .as_object_mut()
        .ok_or("configuration root must be a JSON object")?;
    let Some(hooks) = root.get_mut("hooks") else {
        return Ok(false);
    };
    let hooks = hooks
        .as_object_mut()
        .ok_or("`hooks` must be a JSON object")?;
    let Some(groups) = hooks.get_mut(contract.event) else {
        return Ok(false);
    };
    let groups = groups
        .as_array_mut()
        .ok_or("hook event must be a JSON array")?;
    let mut changed = false;

    for group in groups.iter_mut() {
        let group = group
            .as_object_mut()
            .ok_or("hook event entries must be JSON objects")?;
        let handlers = group
            .get_mut("hooks")
            .and_then(Value::as_array_mut)
            .ok_or("hook group `hooks` must be a JSON array")?;
        let previous_len = handlers.len();
        for handler in handlers.iter() {
            if !handler.is_object() {
                return Err("hook handlers must be JSON objects");
            }
        }
        handlers.retain(|handler| {
            handler
                .get("command")
                .and_then(Value::as_str)
                .is_none_or(|command| command != HOOK_COMMAND)
        });
        changed |= handlers.len() != previous_len;
    }

    if changed {
        groups.retain(|group| !is_empty_managed_group(group, contract));
    }
    if changed && groups.is_empty() {
        hooks.remove(contract.event);
    }
    if changed && hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(changed)
}

fn is_empty_managed_group(group: &Value, contract: HookContract) -> bool {
    let Some(group) = group.as_object() else {
        return false;
    };
    let expected_len = if contract.matcher.is_some() { 2 } else { 1 };
    group.len() == expected_len
        && group_matches_contract(group, contract)
        && group
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
}

fn group_matches_contract(group: &serde_json::Map<String, Value>, contract: HookContract) -> bool {
    match contract.matcher {
        Some(matcher) => group
            .get("matcher")
            .and_then(Value::as_str)
            .is_some_and(|value| value == matcher),
        None => !group.contains_key("matcher"),
    }
}

fn write_temporary_config(
    path: &Path,
    contents: &[u8],
    permissions: Option<&fs::Permissions>,
) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions.clone())?;
    }
    file.write_all(contents)?;
    file.sync_all()
}

fn temporary_path(parent: &Path, destination: &Path) -> PathBuf {
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    parent.join(format!(
        ".{filename}.kfind-{}-{sequence}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests;
