use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;

use super::*;

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            env::temp_dir().join(format!("kfind-cli-test-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: Args, stdin: &[u8], stdin_is_terminal: bool) -> (ExitStatus, Vec<u8>, Vec<u8>) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_with_io(
        &args,
        Language::English,
        stdin,
        &mut stdout,
        &mut stderr,
        stdin_is_terminal,
        false,
    )
    .unwrap();
    (status, stdout, stderr)
}

#[test]
fn piped_stdin_is_the_default_and_sets_match_exit_status() {
    let args = Args::try_parse_from(["kfind", "걷다"]).unwrap();

    let (status, stdout, stderr) = run(args, "길을 걸어 갔다.\n".as_bytes(), false);

    assert_eq!(status, ExitStatus::Match);
    assert_eq!(String::from_utf8(stdout).unwrap(), "길을 걸어 갔다.\n");
    assert!(stderr.is_empty());
}

#[test]
fn a_file_without_matches_returns_one() {
    let temp = TempDir::new();
    let path = temp.write("sample.txt", "멈췄다.\n");
    let args = Args::try_parse_from(["kfind", "걷다", path.to_str().unwrap()]).unwrap();

    let (status, stdout, stderr) = run(args, &[], true);

    assert_eq!(status, ExitStatus::NoMatch);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn count_reports_matching_lines() {
    let temp = TempDir::new();
    let path = temp.write("sample.txt", "걸어 갔다.\n또 걸었다.\n");
    let args = Args::try_parse_from(["kfind", "--count", "걷다", path.to_str().unwrap()]).unwrap();

    let (status, stdout, _) = run(args, &[], true);

    assert_eq!(status, ExitStatus::Match);
    assert_eq!(stdout, b"2\n");
}

#[test]
fn search_issues_are_reported_and_return_two() {
    let temp = TempDir::new();
    let valid = temp.write("valid.txt", "걸어 갔다.\n");
    let missing = temp.0.join("missing.txt");
    let args = Args::try_parse_from([
        "kfind",
        "걷다",
        missing.to_str().unwrap(),
        valid.to_str().unwrap(),
    ])
    .unwrap();

    let (status, stdout, stderr) = run(args, &[], true);

    assert_eq!(status, ExitStatus::Error);
    assert!(String::from_utf8(stdout).unwrap().contains("걸어"));
    assert!(String::from_utf8(stderr).unwrap().contains("missing.txt"));
}

#[test]
fn search_issue_paths_and_messages_escape_control_characters() {
    let issue = kfind_search::SearchIssue {
        kind: kfind_search::SearchIssueKind::Walk,
        path: Some(PathBuf::from("bad\t\u{1b}.txt")),
        message: "ignore\nfailed\u{1b}".to_owned(),
    };
    let mut stderr = Vec::new();

    write_issue(&mut stderr, &issue, Language::English).unwrap();

    assert_eq!(
        String::from_utf8(stderr).unwrap(),
        "kfind: bad\\t\\u{001B}.txt: file traversal failed: ignore\\nfailed\\u{001B}\n"
    );
}

#[test]
fn explicit_missing_data_directory_is_an_error() {
    let temp = TempDir::new();
    let args =
        Args::try_parse_from(["kfind", "--data-dir", temp.0.to_str().unwrap(), "걷다"]).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let error = run_with_io(
        &args,
        Language::English,
        &[][..],
        &mut stdout,
        &mut stderr,
        false,
        false,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::MissingData(_)));
}

#[test]
fn literal_query_does_not_decode_full_pos_lexicon() {
    let temp = TempDir::new();
    temp.write("lexicon.bin", "not a lexicon");
    let input = temp.write("input.txt", "no match\n");
    let args = Args::try_parse_from([
        "kfind",
        "--literal",
        "--explain-query",
        "--data-dir",
        temp.0.to_str().unwrap(),
        "missing",
        input.to_str().unwrap(),
    ])
    .unwrap();

    let (status, stdout, stderr) = run(args, &[], true);

    assert_eq!(status, ExitStatus::NoMatch);
    let stdout = String::from_utf8(stdout).unwrap();
    assert!(stdout.contains("status: not required (literal query)"));
    assert!(!stdout.contains("full POS lexicon unavailable"));
    assert!(stderr.is_empty());
}

#[test]
fn search_issue_context_is_localized() {
    let issue = kfind_search::SearchIssue {
        kind: kfind_search::SearchIssueKind::Input,
        path: Some(PathBuf::from("sample.txt")),
        message: "invalid input encoding: malformed UTF-16".to_owned(),
    };
    let mut stderr = Vec::new();

    write_issue(&mut stderr, &issue, Language::Korean).unwrap();

    assert_eq!(
        String::from_utf8(stderr).unwrap(),
        "kfind: sample.txt: 입력 검색 실패: 입력 인코딩이 올바르지 않습니다: malformed UTF-16\n"
    );
}

#[test]
fn product_owned_search_issue_details_are_localized() {
    assert_eq!(
        search_issue_detail(
            "file search stream closed before completion",
            Language::Korean
        ),
        "파일 검색 stream이 완료 전에 닫혔습니다"
    );
}

#[test]
fn missing_full_pos_candidates_are_preserved_for_preview_diagnostics() {
    let temp = TempDir::new();
    let candidates = vec![temp.0.join("first.bin"), temp.0.join("second.bin")];

    let status = select_full_pos(candidates.clone());

    assert_eq!(
        status,
        FullPosStatus::Preview {
            candidate_paths: candidates.into_boxed_slice(),
        }
    );
}

#[test]
fn full_pos_selection_uses_the_first_existing_candidate() {
    let temp = TempDir::new();
    let selected = temp.write("lexicon.bin", "data");
    let candidates = vec![temp.0.join("missing.bin"), selected.clone()];

    assert_eq!(
        select_full_pos(candidates),
        FullPosStatus::Loaded { path: selected }
    );
}
