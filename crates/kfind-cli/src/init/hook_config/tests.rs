use serde_json::json;

use super::*;

fn contracts(agent: AgentArg) -> AgentHookContracts {
    AgentHookContracts::for_agent(agent).unwrap()
}

fn contract(agent: AgentArg, event: &str) -> HookContract {
    *contracts(agent)
        .hooks
        .iter()
        .find(|contract| contract.event == event)
        .unwrap()
}

#[test]
fn adds_each_agent_hooks_without_removing_existing_settings() {
    for (agent, event, matcher) in [
        (AgentArg::ClaudeCode, "PreToolUse", "Bash"),
        (AgentArg::Codex, "PreToolUse", "Bash"),
        (AgentArg::Gemini, "BeforeTool", "run_shell_command"),
    ] {
        let mut document = json!({
            "theme": "dark",
            "hooks": {
                event: [{
                    "matcher": "Read",
                    "hooks": [{
                        "type": "command",
                        "command": "existing-hook"
                    }]
                }]
            }
        });

        for contract in contracts(agent).hooks {
            assert!(merge_hook(&mut document, *contract).unwrap());
        }
        assert_eq!(document["theme"], "dark");
        let groups = document["hooks"][event].as_array().unwrap();
        assert_eq!(groups[0]["hooks"][0]["command"], "existing-hook");
        assert_eq!(groups[1]["matcher"], matcher);
        assert_eq!(groups[1]["hooks"][0]["command"], HOOK_COMMAND);
        let session_group = &document["hooks"]["SessionStart"][0];
        assert!(session_group.get("matcher").is_none());
        assert_eq!(session_group["hooks"][0]["command"], HOOK_COMMAND);
    }
}

#[test]
fn repeated_merge_is_unchanged_and_deduplicates_managed_handlers() {
    let mut document = json!({});
    for contract in contracts(AgentArg::Codex).hooks {
        assert!(merge_hook(&mut document, *contract).unwrap());
        assert!(!merge_hook(&mut document, *contract).unwrap());
    }

    let duplicate = document["hooks"]["PreToolUse"][0].clone();
    document["hooks"]["PreToolUse"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(merge_hook(&mut document, contract(AgentArg::Codex, "PreToolUse")).unwrap());
    let handlers = document["hooks"]["PreToolUse"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["hooks"].as_array().unwrap())
        .filter(|handler| handler["command"] == HOOK_COMMAND)
        .count();
    assert_eq!(handlers, 1);
}

#[test]
fn rejects_invalid_shapes_in_the_modified_path() {
    for mut document in [
        json!([]),
        json!({ "hooks": [] }),
        json!({ "hooks": { "PreToolUse": {} } }),
        json!({ "hooks": { "PreToolUse": [[]] } }),
        json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": {}
                }]
            }
        }),
    ] {
        assert!(merge_hook(&mut document, contract(AgentArg::Codex, "PreToolUse")).is_err());
    }
}

#[test]
fn removes_managed_handlers_and_preserves_other_hooks() {
    let mut document = json!({
        "theme": "dark",
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Read",
                    "hooks": [{
                        "type": "command",
                        "command": "existing-hook"
                    }]
                },
                contract(AgentArg::Codex, "PreToolUse").group()
            ],
            "SessionStart": [contract(AgentArg::Codex, "SessionStart").group()]
        }
    });

    for contract in contracts(AgentArg::Codex).hooks {
        assert!(remove_hook(&mut document, *contract).unwrap());
    }
    assert_eq!(document["theme"], "dark");
    let groups = document["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["hooks"][0]["command"], "existing-hook");
    assert!(document["hooks"].get("SessionStart").is_none());
}

#[test]
fn removal_is_unchanged_when_the_managed_handler_is_absent() {
    let mut document = json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": "existing-hook"
                }]
            }]
        }
    });

    assert!(!remove_hook(&mut document, contract(AgentArg::Codex, "PreToolUse")).unwrap());
}

#[test]
fn removal_rejects_invalid_shapes_in_the_modified_path() {
    for mut document in [
        json!([]),
        json!({ "hooks": [] }),
        json!({ "hooks": { "PreToolUse": {} } }),
        json!({ "hooks": { "PreToolUse": [[]] } }),
        json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": {}
                }]
            }
        }),
    ] {
        assert!(remove_hook(&mut document, contract(AgentArg::Codex, "PreToolUse")).is_err());
    }
}
