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

    write_issue(&mut stderr, &issue).unwrap();

    assert_eq!(
        String::from_utf8(stderr).unwrap(),
        "kfind: bad\\t\\u{001B}.txt: ignore\\nfailed\\u{001B}\n"
    );
}

#[test]
fn explicit_missing_data_directory_is_an_error() {
    let temp = TempDir::new();
    let args =
        Args::try_parse_from(["kfind", "--data-dir", temp.0.to_str().unwrap(), "걷다"]).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let error = run_with_io(&args, &[][..], &mut stdout, &mut stderr, false, false).unwrap_err();

    assert!(matches!(error, CliError::MissingData(_)));
}
