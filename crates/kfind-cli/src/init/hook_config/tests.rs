use serde_json::json;

use super::*;

fn contract(agent: AgentArg) -> HookContract {
    HookContract::for_agent(agent).unwrap()
}

#[test]
fn adds_each_agent_hook_without_removing_existing_settings() {
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

        assert!(merge_hook(&mut document, contract(agent)).unwrap());
        assert_eq!(document["theme"], "dark");
        let groups = document["hooks"][event].as_array().unwrap();
        assert_eq!(groups[0]["hooks"][0]["command"], "existing-hook");
        assert_eq!(groups[1]["matcher"], matcher);
        assert_eq!(groups[1]["hooks"][0]["command"], HOOK_COMMAND);
    }
}

#[test]
fn repeated_merge_is_unchanged_and_deduplicates_managed_handlers() {
    let mut document = json!({});
    assert!(merge_hook(&mut document, contract(AgentArg::Codex)).unwrap());
    assert!(!merge_hook(&mut document, contract(AgentArg::Codex)).unwrap());

    let duplicate = document["hooks"]["PreToolUse"][0].clone();
    document["hooks"]["PreToolUse"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(merge_hook(&mut document, contract(AgentArg::Codex)).unwrap());
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
        assert!(merge_hook(&mut document, contract(AgentArg::Codex)).is_err());
    }
}
