use std::io::Cursor;

use kfind_data::{
    MecabSourceMorphologyEntry, decode_morphology_resource, encode_morphology_resource,
    parse_mecab_connection_matrix,
};

use super::*;

#[test]
fn vcp_constraint_rejects_lexical_maeil_and_preserves_both_best_paths() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();

    let maeil = evaluate_local_lattice(
        &resource,
        "매일",
        "매".len().."매일".len(),
        DataFinePos::Vcp,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();
    assert_eq!(maeil.decision, LocalLatticeDecision::Reject);
    assert!(maeil.exclude_cost < maeil.include_cost);
    assert!(maeil.paths.iter().any(|path| path.includes_query));
    assert!(maeil.paths.iter().any(|path| !path.includes_query));
    assert!(maeil.paths.len() <= N_BEST);
}

#[test]
fn unknown_hangul_keeps_a_complete_non_query_path() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let report = evaluate_local_lattice(
        &resource,
        "미등록",
        "미등".len().."미등록".len(),
        DataFinePos::Vcp,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert_eq!(report.decision, LocalLatticeDecision::Reject);
    assert_eq!(report.include_cost, None);
    assert!(report.paths[0].nodes.iter().all(|node| node.unknown));
}

#[test]
fn node_limit_is_observable() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let error = evaluate_local_lattice(
        &resource,
        "매일",
        "매".len().."매일".len(),
        DataFinePos::Vcp,
        1,
    )
    .unwrap_err();

    assert!(matches!(error, LocalLatticeError::NodeLimit { .. }));
}

#[test]
fn compound_pos_component_can_satisfy_query() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let report = evaluate_local_lattice(
        &resource,
        "인",
        0.."인".len(),
        DataFinePos::Vcp,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert_eq!(report.decision, LocalLatticeDecision::Accept);
    assert_eq!(report.paths[0].nodes[0].pos.as_deref(), Some("VCP+ETM"));
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

#[test]
fn numeric_unknown_class_keeps_a_complete_path() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let report = evaluate_local_lattice(
        &resource,
        "4일",
        "4".len().."4일".len(),
        DataFinePos::Vcp,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert!(report.paths.iter().all(|path| path.nodes[0].unknown));
}

fn fixture_resource() -> Vec<u8> {
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
    encode_morphology_resource(
        [9; 32],
        &entries,
        &matrix,
        b"DEFAULT 0 1 0\nNUMERIC 1 1 0\nHANGUL 0 1 2\n0x0030..0x0039 NUMERIC\n0xAC00..0xD7A3 HANGUL\n",
        b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nNUMERIC,1,1,100,SN,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
    .unwrap()
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
