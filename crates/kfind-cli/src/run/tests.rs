use std::fs;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use kfind_data::{
    MecabSourceMorphologyEntry, encode_component_resource, encode_pos_lexicon, extract_mecab_ko_dic,
};

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

    fn write_bytes(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
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
    let args = Args::try_parse_from(["kfind", "--embedded", "걷다"]).unwrap();

    let (status, stdout, stderr) = run(args, "길을 걸어 갔다.\n".as_bytes(), false);

    assert_eq!(status, ExitStatus::Match);
    assert_eq!(String::from_utf8(stdout).unwrap(), "길을 걸어 갔다.\n");
    assert!(stderr.is_empty());
}

#[test]
fn a_file_without_matches_returns_one() {
    let temp = TempDir::new();
    let path = temp.write("sample.txt", "멈췄다.\n");
    let args =
        Args::try_parse_from(["kfind", "--embedded", "걷다", path.to_str().unwrap()]).unwrap();

    let (status, stdout, stderr) = run(args, &[], true);

    assert_eq!(status, ExitStatus::NoMatch);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn count_reports_matching_lines() {
    let temp = TempDir::new();
    let path = temp.write("sample.txt", "걸어 갔다.\n또 걸었다.\n");
    let args = Args::try_parse_from([
        "kfind",
        "--embedded",
        "--count",
        "걷다",
        path.to_str().unwrap(),
    ])
    .unwrap();

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
        "--embedded",
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
fn literal_query_does_not_resolve_the_component_resource() {
    let temp = TempDir::new();
    let input = temp.write("input.txt", "missing\n");
    let args = Args::try_parse_from([
        "kfind",
        "--literal",
        "--data-dir",
        temp.0.to_str().unwrap(),
        "missing",
        input.to_str().unwrap(),
    ])
    .unwrap();

    let (status, stdout, stderr) = run(args, &[], true);

    assert_eq!(status, ExitStatus::Match);
    assert_eq!(stdout, b"missing\n");
    assert!(stderr.is_empty());
}

#[test]
fn embedded_mode_does_not_decode_full_pos_lexicon() {
    let temp = TempDir::new();
    temp.write(FULL_POS_FILE, "not a lexicon");
    let input = temp.write("input.txt", "길을 걸었다.\n");
    let args = Args::try_parse_from([
        "kfind",
        "--embedded",
        "--boundary",
        "any",
        "--pos",
        "verb",
        "--explain-query",
        "--data-dir",
        temp.0.to_str().unwrap(),
        "걷다",
        input.to_str().unwrap(),
    ])
    .unwrap();

    let (status, stdout, stderr) = run(args, &[], true);

    assert_eq!(status, ExitStatus::Match);
    let stdout = String::from_utf8(stdout).unwrap();
    assert!(stdout.contains("status: not required (embedded mode)"));
    assert!(stdout.contains("길을 걸었다."));
    assert!(stderr.is_empty());
}

#[test]
fn embedded_mode_still_requires_component_evidence_for_smart_queries() {
    let temp = TempDir::new();
    let mut args = component_args(&temp, "권한");
    args.embedded = true;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let error = run_with_io(
        &args,
        Language::English,
        "사용자권한\n".as_bytes(),
        &mut stdout,
        &mut stderr,
        false,
        false,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::MissingComponent(_)));
    assert!(stdout.is_empty());
}

#[test]
fn nominal_component_query_requires_the_explicit_resource() {
    let temp = TempDir::new();
    let args = component_args(&temp, "권한");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let error = run_with_io(
        &args,
        Language::English,
        "사용자권한\n".as_bytes(),
        &mut stdout,
        &mut stderr,
        false,
        false,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::MissingComponent(_)));
    assert!(stdout.is_empty());
}

#[test]
fn corrupt_component_resource_fails_before_output() {
    let temp = TempDir::new();
    temp.write(COMPONENT_RESOURCE_FILE, "not a component resource");
    let args = component_args(&temp, "권한");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let error = run_with_io(
        &args,
        Language::English,
        "사용자권한\n".as_bytes(),
        &mut stdout,
        &mut stderr,
        false,
        false,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CliError::Data(DataError {
            kind,
            ..
        }) if matches!(kind.as_ref(), kfind_data::DataErrorKind::ComponentResourceCorrupt(_))
    ));
    assert!(stdout.is_empty());
}

#[test]
fn default_smart_loads_component_resource_and_keeps_supported_components() {
    let temp = TempDir::new();
    temp.write_bytes(COMPONENT_RESOURCE_FILE, &component_resource());
    let accepted_args = component_args(&temp, "권한");
    assert_eq!(
        accepted_args.compile_options().unwrap().boundary,
        kfind_query::BoundaryPolicy::Smart
    );

    let accepted = run(accepted_args, "사용자권한\n".as_bytes(), false);
    assert_eq!(accepted.0, ExitStatus::Match);
    assert_eq!(accepted.1, "사용자권한\n".as_bytes());

    let component = run(component_args(&temp, "학교"), "대학교\n".as_bytes(), false);
    assert_eq!(component.0, ExitStatus::Match);
    assert_eq!(component.1, "대학교\n".as_bytes());
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

#[test]
fn enriched_predicates_load_from_an_explicit_data_directory() {
    let temp = TempDir::new();
    temp.write(
        ENRICHED_PREDICATES_FILE,
        "lemma\tpos\talternation\tflags\toverrides\n가르다\tVV\tReuDoubleL\t\t\n",
    );
    let args = Args::try_parse_from([
        "kfind",
        "--pos",
        "verb",
        "--data-dir",
        temp.0.to_str().unwrap(),
        "가르다",
    ])
    .unwrap();

    let loaded = load_lexicons(
        &args,
        FullPosMode::Disabled(FullPosNotRequiredReason::LiteralQuery),
    )
    .unwrap();

    assert!(
        loaded
            .lexicons
            .lookup("가르다")
            .iter()
            .any(|analysis| { analysis.source == kfind_query::AnalysisSource::EnrichedLexicon })
    );
}

#[test]
fn enriched_predicates_recover_irregular_surfaces_and_mixed_regular_analysis() {
    let temp = TempDir::new();
    temp.write_bytes(FULL_POS_FILE, &full_pos_resource());
    temp.write_bytes(COMPONENT_RESOURCE_FILE, &component_resource());
    temp.write(
        ENRICHED_PREDICATES_FILE,
        concat!(
            "lemma\tpos\talternation\tflags\toverrides\n",
            "가깝다\tVA\tBToWo\t\t\n",
            "결정짓다\tVV\tDropS\t\t\n",
            "곱다\tVA\tBToWa\t\t\n",
            "곱다\tVA\tRegular\t\t\n",
            "깨닫다\tVV\tDToL\t\t\n",
            "노랗다\tVA\tDropH\t\t\n",
        ),
    );

    for (pos, query, text) in [
        ("verb", "깨닫다", "그제야 진실을 깨달았다.\n"),
        ("verb", "결정짓다", "한 표가 승부를 결정지었다.\n"),
        ("adjective", "가깝다", "이전보다 훨씬 가까워졌다.\n"),
        ("adjective", "곱다", "한복의 자태가 고와 보였다.\n"),
        ("adjective", "곱다", "추위에 손이 곱아 버렸다.\n"),
        ("adjective", "노랗다", "얼굴빛이 노래졌다.\n"),
    ] {
        let args = Args::try_parse_from([
            "kfind",
            "--pos",
            pos,
            "--data-dir",
            temp.0.to_str().unwrap(),
            query,
        ])
        .unwrap();

        let (status, stdout, stderr) = run(args, text.as_bytes(), false);

        assert_eq!(status, ExitStatus::Match, "{query} should match {text}");
        assert_eq!(stdout, text.as_bytes());
        assert!(stderr.is_empty());
    }
}

#[test]
fn embedded_mode_skips_enriched_predicates() {
    let temp = TempDir::new();
    temp.write(
        ENRICHED_PREDICATES_FILE,
        "lemma\tpos\talternation\tflags\toverrides\n가르다\tVV\tReuDoubleL\t\t\n",
    );
    let args = Args::try_parse_from([
        "kfind",
        "--embedded",
        "--pos",
        "verb",
        "--data-dir",
        temp.0.to_str().unwrap(),
        "가르다",
    ])
    .unwrap();

    let loaded = load_lexicons(
        &args,
        FullPosMode::Disabled(FullPosNotRequiredReason::EmbeddedMode),
    )
    .unwrap();

    assert!(loaded.lexicons.lookup("가르다").is_empty());
}

fn component_args(temp: &TempDir, query: &str) -> Args {
    temp.write_bytes(FULL_POS_FILE, &full_pos_resource());
    Args::try_parse_from([
        "kfind",
        "--pos",
        "noun",
        "--data-dir",
        temp.0.to_str().unwrap(),
        query,
    ])
    .unwrap()
}

fn full_pos_resource() -> Vec<u8> {
    let extraction = extract_mecab_ko_dic(
        "fixture.csv",
        Cursor::new("권한,1,1,0,NNG,*,T,권한,*,*,*,*\n학교,1,1,0,NNG,*,T,학교,*,*,*,*\n"),
    )
    .unwrap();
    encode_pos_lexicon(&extraction.into_pos_lexicon()).unwrap()
}

fn component_resource() -> Vec<u8> {
    let entries = [
        component_entry("사용자", "NNG", -5_000),
        component_entry("권한", "NNG", -5_000),
        component_entry("사용자권한", "NNG", 5_000),
        component_entry("대", "XPN", 5_000),
        component_entry("학교", "NNG", 5_000),
        component_entry("대학교", "NNG", -5_000),
        component_entry("깨달", "VV", -5_000),
        component_entry("결정지", "VV", -5_000),
        component_entry("가까워", "VA", -5_000),
        component_entry("고와", "VA", -5_000),
        component_entry("곱아", "VA", -5_000),
        component_entry("노래", "VA", -5_000),
    ];
    encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries).unwrap()
}

fn component_entry(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: "*".to_owned(),
    }
}
