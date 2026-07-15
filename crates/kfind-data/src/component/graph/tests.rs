use std::io::Cursor;

use crate::{
    MorphologyExpressionAlignmentKind, align_morphology_expression, decode_component_resource,
    decode_morphology_resource, encode_morphology_resource, parse_mecab_connection_matrix,
};

use super::*;

#[test]
fn graph_resource_round_trips_structural_relations_without_scoring_data() {
    let entries = fixture_entries();
    let matrix = fixture_matrix();
    let bytes = encode_morphology_graph_resource(
        [7; 32],
        &entries,
        &matrix,
        b"HANGUL 0 1 2\n",
        b"HANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    let resource = decode_morphology_graph_resource("fixture", bytes, &[7; 32]).unwrap();

    assert_eq!(resource.stats().schema_version, 4);
    assert_eq!(resource.stats().surface_count, 5);
    assert_eq!(resource.stats().analysis_count, 5);
    assert_eq!(resource.stats().component_count, 6);
    assert_eq!(resource.stats().transition_count, 3);
    assert_eq!(
        resource.stats().expression_counts[&MorphologyGraphExpressionKind::Absent],
        1
    );
    assert!(resource.allows_transition("NNG", "NNG"));
    assert!(resource.allows_transition("VV", "EC"));
    assert!(!resource.allows_transition("NNG", "EC"));
    assert_eq!(resource.char_def(), b"HANGUL 0 1 2\n");

    let mut prefixes = Vec::new();
    resource.common_prefixes("산속에서".as_bytes(), |length, surface, analyses| {
        prefixes.push((length, surface.to_owned(), analyses.to_vec()));
    });
    assert_eq!(prefixes.len(), 1);
    assert_eq!(prefixes[0].0, "산속".len());
    assert_eq!(prefixes[0].1, "산속");
    assert_eq!(prefixes[0].2[0].analysis_type, "Compound");
    assert_eq!(
        prefixes[0].2[0].expression_kind,
        MorphologyGraphExpressionKind::SpanAligned
    );
    assert_eq!(
        prefixes[0].2[0].components,
        vec![
            MorphologyGraphComponent {
                surface: "산",
                pos: "NNG",
                span: Some(0..3),
            },
            MorphologyGraphComponent {
                surface: "속",
                pos: "NNG",
                span: Some(3..6),
            },
        ]
    );
}

#[test]
fn graph_projection_matches_full_morphology_source_rows() {
    let entries = fixture_entries();
    let matrix = fixture_matrix();
    let full_bytes =
        encode_morphology_resource([7; 32], &entries, &matrix, b"char", b"unknown").unwrap();
    let graph_bytes =
        encode_morphology_graph_resource([7; 32], &entries, &matrix, b"char", b"unknown").unwrap();
    let full = decode_morphology_resource("full", &full_bytes, &[7; 32]).unwrap();
    let graph = decode_morphology_graph_resource("graph", graph_bytes, &[7; 32]).unwrap();
    assert_eq!(
        validate_morphology_graph_projection("projection", &full, &graph).unwrap(),
        MorphologyGraphProjectionStats {
            surface_count: 5,
            analysis_count: 5,
            component_count: 6,
            transition_count: 3,
            source_matrix_cost_count: 4,
        }
    );

    for surface in entries.iter().map(|entry| &entry.surface) {
        let input = format!("{surface}밖");
        let mut full_projection = Vec::new();
        full.common_prefixes(input.as_bytes(), |length, analyses| {
            if length == surface.len() {
                full_projection.extend(analyses.iter().map(|analysis| {
                    let alignment = (!matches!(analysis.expression, "" | "*"))
                        .then(|| align_morphology_expression(surface, analysis.expression));
                    let kind = alignment.as_ref().map_or(
                        MorphologyGraphExpressionKind::Absent,
                        |alignment| match alignment.kind {
                            MorphologyExpressionAlignmentKind::SpanAligned => {
                                MorphologyGraphExpressionKind::SpanAligned
                            }
                            MorphologyExpressionAlignmentKind::Fused => {
                                MorphologyGraphExpressionKind::Fused
                            }
                            MorphologyExpressionAlignmentKind::Unaligned => {
                                MorphologyGraphExpressionKind::Unaligned
                            }
                            MorphologyExpressionAlignmentKind::Invalid => {
                                MorphologyGraphExpressionKind::Invalid
                            }
                        },
                    );
                    (
                        analysis.pos.to_owned(),
                        analysis.analysis_type.to_owned(),
                        analysis.start_pos.to_owned(),
                        analysis.end_pos.to_owned(),
                        kind,
                        alignment.map_or_else(Vec::new, |alignment| {
                            alignment
                                .components
                                .into_iter()
                                .map(|component| {
                                    (
                                        component.surface.to_owned(),
                                        component.pos.to_owned(),
                                        component.span,
                                    )
                                })
                                .collect()
                        }),
                    )
                }));
            }
        });
        let mut graph_projection = Vec::new();
        graph.common_prefixes(input.as_bytes(), |length, graph_surface, analyses| {
            if length == surface.len() {
                assert_eq!(graph_surface, surface);
                graph_projection.extend(analyses.iter().map(|analysis| {
                    (
                        analysis.pos.to_owned(),
                        analysis.analysis_type.to_owned(),
                        analysis.start_pos.to_owned(),
                        analysis.end_pos.to_owned(),
                        analysis.expression_kind,
                        analysis
                            .components
                            .iter()
                            .map(|component| {
                                (
                                    component.surface.to_owned(),
                                    component.pos.to_owned(),
                                    component.span.clone(),
                                )
                            })
                            .collect(),
                    )
                }));
            }
        });

        assert_eq!(graph_projection, full_projection, "surface={surface}");
    }
}

#[test]
fn schema_four_isolated_from_the_product_schema_one_loader() {
    let bytes = encode_morphology_graph_resource(
        [7; 32],
        &[entry("가", "NNG", "*", "*", "*", "*", 0, 0, 1)],
        &parse_mecab_connection_matrix("matrix.def", Cursor::new("1 1\n0 0 1\n")).unwrap(),
        b"char",
        b"unknown",
    )
    .unwrap();

    assert!(matches!(
        decode_component_resource("schema-one", bytes.clone(), &[7; 32])
            .unwrap_err()
            .kind
            .as_ref(),
        DataErrorKind::ComponentResourceSchema {
            expected: 1,
            actual: 4
        }
    ));
    decode_morphology_graph_resource("schema-four", bytes, &[7; 32]).unwrap();
}

#[test]
fn graph_resource_rejects_source_section_and_relation_corruption() {
    let entries = [entry(
        "산속",
        "NNG",
        "Compound",
        "NNG",
        "NNG",
        "산/NNG/*+속/NNG/*",
        0,
        0,
        1,
    )];
    let matrix = parse_mecab_connection_matrix("matrix.def", Cursor::new("1 1\n0 0 1\n")).unwrap();
    let bytes =
        encode_morphology_graph_resource([7; 32], &entries, &matrix, b"char", b"unknown").unwrap();

    assert!(matches!(
        decode_morphology_graph_resource("fixture", bytes.clone(), &[8; 32])
            .unwrap_err()
            .kind
            .as_ref(),
        DataErrorKind::ComponentResourceSourceMismatch
    ));

    let mut content = bytes.clone();
    *content.last_mut().unwrap() ^= 1;
    assert!(matches!(
        decode_morphology_graph_resource("fixture", content, &[7; 32])
            .unwrap_err()
            .kind
            .as_ref(),
        DataErrorKind::ComponentResourceCorrupt(_)
    ));

    let mut relation = bytes;
    let index_len = usize::try_from(read_u64_at(&relation, 64).unwrap()).unwrap();
    let payload_start = HEADER_LEN + index_len;
    let expression_kind_offset = payload_start + 56;
    relation[expression_kind_offset] = MorphologyGraphExpressionKind::Fused.encode();
    refresh_payload_digest(&mut relation, payload_start);
    let error = decode_morphology_graph_resource("fixture", relation, &[7; 32]).unwrap_err();
    assert!(matches!(
        error.kind.as_ref(),
        DataErrorKind::ComponentResourceCorrupt(message)
            if message.contains("opaque graph components")
    ));
}

fn fixture_entries() -> Vec<MecabSourceMorphologyEntry> {
    vec![
        entry("가", "NNG", "*", "*", "*", "*", 1, 1, 10),
        entry(
            "산속",
            "NNG",
            "Compound",
            "NNG",
            "NNG",
            "산/NNG/*+속/NNG/*",
            1,
            0,
            20,
        ),
        entry(
            "한",
            "XSA+ETM",
            "Inflect",
            "XSA",
            "ETM",
            "하/XSA/*+ᆫ/ETM/*",
            0,
            1,
            30,
        ),
        entry(
            "비춰",
            "VV+EC",
            "Inflect",
            "VV",
            "EC",
            "비추/VV/*+어/EC/*",
            0,
            1,
            40,
        ),
        entry("잘못", "NNG", "Broken", "NNG", "NNG", "broken", 1, 1, 50),
    ]
}

fn fixture_matrix() -> MecabConnectionMatrix {
    parse_mecab_connection_matrix(
        "matrix.def",
        Cursor::new("2 2\n0 0 1\n0 1 2\n1 0 3\n1 1 4\n"),
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn entry(
    surface: &str,
    pos: &str,
    analysis_type: &str,
    start_pos: &str,
    end_pos: &str,
    expression: &str,
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
        analysis_type: analysis_type.to_owned(),
        start_pos: start_pos.to_owned(),
        end_pos: end_pos.to_owned(),
        expression: expression.to_owned(),
    }
}

fn refresh_payload_digest(bytes: &mut [u8], payload_start: usize) {
    let payload_len = usize::try_from(read_u64_at(bytes, 72).unwrap()).unwrap();
    let digest = sha256(&bytes[payload_start..payload_start + payload_len]);
    bytes[144..176].copy_from_slice(&digest);
}

fn read_u64_at(input: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        input.get(offset..offset + 8)?.try_into().ok()?,
    ))
}
