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
fn dependent_noun_query_accepts_the_nnbc_source_tag() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();

    let dependent = evaluate_local_component_decision(
        &resource,
        "명",
        0.."명".len(),
        DataFinePos::Nnb,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();
    let common = evaluate_local_component_decision(
        &resource,
        "명",
        0.."명".len(),
        DataFinePos::Nng,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
    .unwrap();

    assert_eq!(dependent, LocalLatticeDecision::Accept);
    assert_eq!(common, LocalLatticeDecision::Reject);
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

#[test]
fn decision_only_evaluation_matches_diagnostic_costs() {
    let bytes = fixture_resource();
    let resource = decode_morphology_resource("fixture", &bytes, &[9; 32]).unwrap();
    let cases = [
        (
            "사용자권한",
            "사용자".len().."사용자권한".len(),
            LocalLatticeDecision::Accept,
        ),
        (
            "대학교",
            "대".len().."대학교".len(),
            LocalLatticeDecision::Reject,
        ),
        ("공공", 0.."공".len(), LocalLatticeDecision::Ambiguous),
    ];

    for (text, query_span, expected) in cases {
        let decision = evaluate_local_component_decision(
            &resource,
            text,
            query_span.clone(),
            DataFinePos::Nng,
            DEFAULT_LATTICE_NODE_LIMIT,
        )
        .unwrap();
        let costs = evaluate_local_costs(
            &resource,
            text,
            query_span.clone(),
            DataFinePos::Nng,
            DEFAULT_LATTICE_NODE_LIMIT,
        )
        .unwrap();
        let report = evaluate_local_component_paths(
            &resource,
            text,
            query_span,
            DataFinePos::Nng,
            DEFAULT_LATTICE_NODE_LIMIT,
        )
        .unwrap();

        assert_eq!(decision, expected);
        assert_eq!(decision, report.decision);
        assert_eq!(costs.include, report.include_cost);
        assert_eq!(costs.exclude, report.exclude_cost);
    }
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
    [MecabSourceMorphologyEntry; 15],
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
        entry("공", DataFinePos::Nng, 1, 1, 0),
        entry("공공", DataFinePos::Nng, 1, 1, 0),
        source_entry("명", "NNBC", 1, 1, -5_000),
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
