use std::io::Cursor;

use kfind_data::{
    MecabSourceMorphologyEntry, decode_component_resource, decode_morphology_resource,
    encode_component_resource, encode_morphology_resource, parse_mecab_connection_matrix,
};

use super::*;

#[test]
fn node_limit_is_observable() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let error = evaluate_local_component_paths(
        &resource,
        "사용자권한",
        "사용자".len().."사용자권한".len(),
        DataFinePos::Nng,
        1,
    )
    .unwrap_err();

    assert!(matches!(error, LocalLatticeError::NodeLimit { .. }));
}

#[test]
fn exact_component_path_accepts_a_matching_node_span() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let report = evaluate_local_component_paths(
        &resource,
        "사용자권한",
        "사용자".len().."사용자권한".len(),
        DataFinePos::Nng,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert_eq!(report.decision, LocalLatticeDecision::Accept);
    assert!(report.paths.iter().any(|path| path.includes_query));
}

#[test]
fn compact_component_resource_supports_exact_path_evaluation() {
    let bytes = compact_fixture_resource();
    let resource = decode_component_resource("fixture", bytes, &[9; 32]).unwrap();
    let report = evaluate_local_component_paths(
        &resource,
        "사용자권한",
        "사용자".len().."사용자권한".len(),
        DataFinePos::Nng,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert_eq!(report.decision, LocalLatticeDecision::Accept);
    assert!(report.paths.iter().any(|path| path.includes_query));
}

#[test]
fn exact_component_path_rejects_a_crossing_substring() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let report = evaluate_local_component_paths(
        &resource,
        "대학교",
        "대".len().."대학교".len(),
        DataFinePos::Nng,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert_eq!(report.decision, LocalLatticeDecision::Reject);
    assert!(report.paths.iter().any(|path| path.includes_query));
    assert!(report.paths.iter().any(|path| !path.includes_query));
}

fn fixture_resource() -> Vec<u8> {
    let (entries, matrix, char_def, unk_def) = fixture_parts();
    encode_morphology_resource([9; 32], &entries, &matrix, char_def, unk_def).unwrap()
}

fn compact_fixture_resource() -> Vec<u8> {
    let (entries, matrix, char_def, unk_def) = fixture_parts();
    encode_component_resource([9; 32], &entries, &matrix, char_def, unk_def).unwrap()
}

fn fixture_parts() -> (
    [MecabSourceMorphologyEntry; 12],
    kfind_data::MecabConnectionMatrix,
    &'static [u8],
    &'static [u8],
) {
    let entries = [
        entry("매", DataFinePos::Nng, 1, 1, 30),
        entry("매일", DataFinePos::Mag, 1, 1, 1),
        entry("일", DataFinePos::Nng, 1, 1, 20),
        entry("일", DataFinePos::Vcp, 1, 1, 1),
        source_entry("인", "NNG", 1, 1, 20),
        source_entry("인", "VCP+ETM", 1, 1, 1),
        entry("대학교", DataFinePos::Nng, 1, 1, -5_000),
        source_entry("대", "XPN", 1, 1, 5_000),
        entry("학교", DataFinePos::Nng, 1, 1, 5_000),
        entry("사용자", DataFinePos::Nng, 1, 1, -5_000),
        entry("권한", DataFinePos::Nng, 1, 1, -5_000),
        entry("사용자권한", DataFinePos::Nng, 1, 1, 5_000),
    ];
    let matrix = parse_mecab_connection_matrix(
        "matrix.def",
        Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
    )
    .unwrap();
    (
        entries,
        matrix,
        b"DEFAULT 0 1 0\nNUMERIC 1 1 0\nHANGUL 0 1 2\n0x0030..0x0039 NUMERIC\n0xAC00..0xD7A3 HANGUL\n",
        b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nNUMERIC,1,1,100,SN,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
}

fn entry(
    surface: &str,
    pos: DataFinePos,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
) -> MecabSourceMorphologyEntry {
    source_entry(surface, pos.as_str(), left_id, right_id, word_cost)
}

fn source_entry(
    surface: &str,
    pos: &str,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id,
        right_id,
        word_cost,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: "*".to_owned(),
    }
}
