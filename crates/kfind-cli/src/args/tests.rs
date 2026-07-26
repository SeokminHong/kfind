use super::*;

#[test]
fn defaults_match_the_spec() {
    let args = Args::try_parse_from(["kfind", "걷다"]).unwrap();

    let options = args.compile_options().unwrap();
    assert_eq!(options.global_pos, None);
    assert_eq!(options.expand, ExpandMode::Inflection);
    assert_eq!(options.boundary, BoundaryPolicy::Smart);
    assert_eq!(options.phrase.max_gap, 24);
    assert_eq!(args.query(), Some("걷다"));
    assert!(args.paths.is_empty());
    assert!(!args.no_pager);
}

#[test]
fn init_replaces_the_required_query() {
    let args = Args::try_parse_from([
        "kfind",
        "--init",
        "--agent",
        "codex",
        "--agent",
        "claude-code",
    ])
    .unwrap();

    assert!(args.init);
    assert_eq!(args.query(), None);
    assert_eq!(args.agent, [AgentArg::Codex, AgentArg::ClaudeCode]);
}

#[test]
fn uninstall_replaces_the_required_query() {
    let args = Args::try_parse_from([
        "kfind",
        "--uninstall",
        "--agent",
        "codex",
        "--agent",
        "gemini",
    ])
    .unwrap();

    assert!(args.uninstall);
    assert_eq!(args.query(), None);
    assert_eq!(args.agent, [AgentArg::Codex, AgentArg::Gemini]);
}

#[test]
fn integration_modes_are_mutually_exclusive() {
    let error = Args::try_parse_from(["kfind", "--init", "--uninstall"]).unwrap_err();
    assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
}

#[test]
fn agent_hook_is_a_hidden_exclusive_mode() {
    let args = Args::try_parse_from(["kfind", "--agent-hook"]).unwrap();

    assert!(args.agent_hook);
    assert_eq!(args.query(), None);
    let conflict = Args::try_parse_from(["kfind", "--agent-hook", "걷다"]).unwrap_err();
    assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);
}

#[test]
fn data_check_replaces_the_query_and_allows_json_and_data_dir() {
    let args =
        Args::try_parse_from(["kfind", "--check-data", "--json", "--data-dir", "data"]).unwrap();

    assert!(args.check_data);
    assert!(args.json);
    assert_eq!(args.data_dir, Some(PathBuf::from("data")));
    assert_eq!(args.query(), None);
}

#[test]
fn agent_requires_an_integration_mode() {
    let conflict = Args::try_parse_from(["kfind", "--agent", "codex", "걷다"]).unwrap_err();
    assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);

    let error = Args::try_parse_from(["kfind", "--agent", "codex"]).unwrap_err();
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn init_rejects_search_arguments() {
    for values in [
        ["kfind", "--init", "걷다"].as_slice(),
        ["kfind", "--init", "--json"].as_slice(),
        ["kfind", "--init", "--no-pager"].as_slice(),
        ["kfind", "--init", "--data-dir", "data"].as_slice(),
    ] {
        let error = Args::try_parse_from(values).unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}

#[test]
fn uninstall_rejects_search_arguments() {
    for values in [
        ["kfind", "--uninstall", "걷다"].as_slice(),
        ["kfind", "--uninstall", "--json"].as_slice(),
        ["kfind", "--uninstall", "--no-pager"].as_slice(),
        ["kfind", "--uninstall", "--data-dir", "data"].as_slice(),
    ] {
        let error = Args::try_parse_from(values).unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}

#[test]
fn short_h_controls_filenames() {
    let args = Args::try_parse_from(["kfind", "-h", "걷다", "."]).unwrap();

    assert!(args.no_filename);
    assert_eq!(args.paths, [PathBuf::from(".")]);
}

#[test]
fn options_may_follow_search_paths() {
    let args = Args::try_parse_from([
        "kfind",
        "n:권한 v:검증하다",
        "src",
        "docs",
        "--max-gap",
        "12",
    ])
    .unwrap();

    assert_eq!(args.paths, [PathBuf::from("src"), PathBuf::from("docs")]);
    assert_eq!(args.max_gap, Some(12));
}

#[test]
fn encoding_names_match_the_cli_contract() {
    for (value, expected) in [
        ("utf-8", EncodingArg::Utf8),
        ("utf-16le", EncodingArg::Utf16le),
        ("utf-16be", EncodingArg::Utf16be),
        ("euc-kr", EncodingArg::EucKr),
    ] {
        let args = Args::try_parse_from(["kfind", "--encoding", value, "걷다"]).unwrap();
        assert_eq!(args.encoding, expected);
    }
}

#[test]
fn literal_resolves_both_mode_axes() {
    let args = Args::try_parse_from(["kfind", "--literal", "걸어"]).unwrap();

    let options = args.compile_options().unwrap();
    assert_eq!(options.global_pos, Some(CoarsePos::Literal));
    assert_eq!(options.expand, ExpandMode::Literal);
    assert_eq!(options.boundary, BoundaryPolicy::Smart);
}

#[test]
fn literal_rejects_conflicting_pos() {
    let args = Args::try_parse_from(["kfind", "--literal", "--pos", "verb", "걸어"]).unwrap();

    assert!(matches!(
        args.compile_options(),
        Err(CompileOptionError::LiteralPosConflict {
            pos: CoarsePos::Verb
        })
    ));
}
