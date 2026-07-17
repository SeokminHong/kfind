use crate::MecabSourceMorphologyEntry;

use super::*;

#[test]
fn resource_owns_bytes_and_preserves_only_aligned_structure() {
    let resource = decode_component_resource("fixture", fixture_resource(), &[7; 32]).unwrap();
    let mut prefixes = Vec::new();
    resource.common_prefixes("가나다".as_bytes(), |length, analyses| {
        prefixes.push((length, analyses.to_vec()));
    });

    assert_eq!(resource.stats().surface_count, 2);
    assert_eq!(resource.stats().analysis_count, 2);
    assert_eq!(resource.stats().component_count, 2);
    assert_eq!(prefixes.len(), 2);
    assert_eq!(prefixes[1].0, "가나".len());
    assert_eq!(prefixes[1].1[0].pos, "NNG+JX");
    assert_eq!(prefixes[1].1[0].components.len(), 2);
    assert_eq!(prefixes[1].1[0].components[0].span, 0.."가".len());
    assert_eq!(prefixes[1].1[0].components[0].pos, "NNG");
    assert_eq!(
        prefixes[1].1[0].components[1].span,
        "가".len().."가나".len()
    );
    assert_eq!(prefixes[1].1[0].components[1].pos, "JX");
}

#[test]
fn resource_rejects_schema_source_and_content_mismatches() {
    let bytes = fixture_resource();
    assert_eq!(
        decode_component_resource("fixture", bytes.clone(), &[8; 32])
            .unwrap_err()
            .kind
            .as_ref(),
        &DataErrorKind::ComponentResourceSourceMismatch
    );

    let mut schema = bytes.clone();
    schema[MAGIC.len()..MAGIC.len() + 4].copy_from_slice(&(SCHEMA_VERSION + 1).to_le_bytes());
    assert!(matches!(
        decode_component_resource("fixture", schema, &[7; 32])
            .unwrap_err()
            .kind
            .as_ref(),
        DataErrorKind::ComponentResourceSchema { .. }
    ));

    let mut cursor = 60;
    let mut lengths = [0_usize; SECTION_COUNT];
    for length in &mut lengths {
        *length = usize::try_from(read_u64(&bytes, &mut cursor).unwrap()).unwrap();
    }
    let ranges = section_ranges("fixture", bytes.len(), HEADER_LEN, lengths).unwrap();
    for range in ranges {
        let mut content = bytes.clone();
        content[range.start] ^= 1;
        assert_eq!(
            decode_component_resource("fixture", content, &[7; 32])
                .unwrap_err()
                .kind
                .as_ref(),
            &DataErrorKind::ComponentResourceCorrupt("section digest mismatch".to_owned())
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parallel_section_digests_match_sequential_validation() {
    let bytes = vec![7_u8; PARALLEL_DIGEST_MIN_SECTION_LEN * 2 + 8];
    let ranges = [
        0..PARALLEL_DIGEST_MIN_SECTION_LEN,
        PARALLEL_DIGEST_MIN_SECTION_LEN..PARALLEL_DIGEST_MIN_SECTION_LEN * 2,
        PARALLEL_DIGEST_MIN_SECTION_LEN * 2..bytes.len(),
    ];
    assert_eq!(
        section_digests(&bytes, &ranges),
        sequential_section_digests(&bytes, &ranges)
    );
}

fn fixture_resource() -> Vec<u8> {
    let entries = [
        entry("가", "NNG", 1, 1, 10),
        entry("가나", "NNG+JX", 1, 1, 20),
        entry("가나", "NNG+JX", 0, 0, 999),
    ];
    encode_component_resource([7; 32], &entries).unwrap()
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
        analysis_type: if surface == "가나" { "Inflect" } else { "*" }.to_owned(),
        start_pos: if surface == "가나" { "NNG" } else { "*" }.to_owned(),
        end_pos: if surface == "가나" { "JX" } else { "*" }.to_owned(),
        expression: if surface == "가나" {
            "가/NNG/*+나/JX/*"
        } else {
            "*"
        }
        .to_owned(),
    }
}
