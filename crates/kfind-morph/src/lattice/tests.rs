use std::io::Cursor;

use kfind_data::{
    MecabMorphologyEntry, decode_morphology_resource, encode_morphology_resource,
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

fn fixture_resource() -> Vec<u8> {
    let entries = [
        entry("매", DataFinePos::Nng, 1, 1, 30),
        entry("매일", DataFinePos::Mag, 1, 1, 1),
        entry("일", DataFinePos::Nng, 1, 1, 20),
        entry("일", DataFinePos::Vcp, 1, 1, 1),
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
        b"HANGUL 0 1 2\n",
        b"HANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
    .unwrap()
}

fn entry(
    surface: &str,
    pos: DataFinePos,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
) -> MecabMorphologyEntry {
    MecabMorphologyEntry {
        surface: surface.to_owned(),
        pos,
        left_id,
        right_id,
        word_cost,
    }
}
