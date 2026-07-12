use std::io::Cursor;

use super::*;
use crate::parse_mecab_connection_matrix;

#[test]
fn resource_round_trips_prefixes_costs_and_source_definitions() {
    let entries = vec![
        entry("가", "NNG", 1, 1, 10),
        entry("가", "VV", 1, 0, 20),
        entry("가다", "VV+EF", 0, 1, 30),
    ];
    let matrix = parse_mecab_connection_matrix(
        "matrix.def",
        Cursor::new("2 2\n0 0 1\n0 1 2\n1 0 3\n1 1 4\n"),
    )
    .unwrap();
    let digest = [7; 32];
    let bytes = encode_morphology_resource(
        digest,
        &entries,
        &matrix,
        b"HANGUL 0 1 2\n",
        b"HANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    let decoded = decode_morphology_resource("fixture", &bytes, &digest).unwrap();

    let mut prefixes = Vec::new();
    decoded.common_prefixes("가다가".as_bytes(), |length, analyses| {
        prefixes.push((length, analyses.to_vec()));
    });
    assert_eq!(prefixes.len(), 2);
    assert_eq!(prefixes[0].0, "가".len());
    assert_eq!(prefixes[0].1.len(), 2);
    assert_eq!(prefixes[1].0, "가다".len());
    assert_eq!(prefixes[1].1[0].pos, "VV+EF");
    assert_eq!(decoded.stats().pos_counts["VV+EF"], 1);
    assert_eq!(decoded.connection_cost(1, 0), Some(3));
    assert_eq!(decoded.char_def(), b"HANGUL 0 1 2\n");
}

#[test]
fn resource_rejects_source_and_section_corruption() {
    let matrix = parse_mecab_connection_matrix("matrix.def", Cursor::new("1 1\n0 0 1\n")).unwrap();
    let mut bytes = encode_morphology_resource(
        [3; 32],
        &[entry("가", "NNG", 0, 0, 1)],
        &matrix,
        b"char",
        b"unknown",
    )
    .unwrap();
    assert!(decode_morphology_resource("fixture", &bytes, &[4; 32]).is_err());

    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert!(decode_morphology_resource("fixture", &bytes, &[3; 32]).is_err());
}

#[test]
fn sha256_parser_rejects_non_ascii_input_without_panicking() {
    let input = format!("{}가", "0".repeat(61));

    assert_eq!(input.len(), 64);
    assert!(parse_sha256(&input).is_err());
}

fn entry(
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
