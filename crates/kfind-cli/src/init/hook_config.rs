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

pub(super) fn prepare_hook_installation(
    root: &Path,
    agent: AgentArg,
) -> Result<Option<HookInstallation>, InitError> {
    let Some(contract) = HookContract::for_agent(agent) else {
        return Ok(None);
    };
    let path = root.join(contract.path);
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
    let changed =
        merge_hook(&mut document, contract).map_err(|reason| InitError::InvalidAgentConfig {
            path: path.clone(),
            reason,
        })?;
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

struct HookContract {
    path: &'static str,
    event: &'static str,
    matcher: &'static str,
    handler_name: Option<&'static str>,
}

impl HookContract {
    const fn for_agent(agent: AgentArg) -> Option<Self> {
        match agent {
            AgentArg::ClaudeCode => Some(Self {
                path: ".claude/settings.json",
                event: "PreToolUse",
                matcher: "Bash",
                handler_name: None,
            }),
            AgentArg::Codex => Some(Self {
                path: ".codex/hooks.json",
                event: "PreToolUse",
                matcher: "Bash",
                handler_name: None,
            }),
            AgentArg::Gemini => Some(Self {
                path: ".gemini/settings.json",
                event: "BeforeTool",
                matcher: "run_shell_command",
                handler_name: Some("kfind-korean-search"),
            }),
            AgentArg::Custom => None,
        }
    }

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
        json!({
            "matcher": self.matcher,
            "hooks": [self.handler()],
        })
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
                if group
                    .get("matcher")
                    .and_then(Value::as_str)
                    .is_some_and(|matcher| matcher == contract.matcher)
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
