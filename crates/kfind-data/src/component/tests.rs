use crate::MecabSourceMorphologyEntry;

use super::*;

#[test]
fn resource_size_limit_is_checked_without_allocating_the_input() {
    assert!(validate_encoded_size(MAX_COMPONENT_RESOURCE_BYTES, "fixture").is_ok());
    assert!(validate_encoded_size(MAX_COMPONENT_RESOURCE_BYTES + 1, "fixture").is_err());
}

#[test]
fn component_positions_keep_prefix_categories_in_one_byte() {
    assert_eq!(std::mem::size_of::<ComponentPos>(), 1);
    assert!(ComponentPos::parse("NNBC").is_nominal_tag());
    assert!(ComponentPos::parse("NNQ").is_nominal_tag());
    assert!(ComponentPos::parse("VQ").is_predicate_tag());
    assert!(ComponentPos::parse("EQ").is_ending());
    assert!(ComponentPos::parse("JQ").is_particle());
}

#[test]
fn resource_owns_bytes_and_preserves_only_aligned_structure() {
    let resource = decode_component_resource("fixture", fixture_resource(), &[7; 32]).unwrap();
    let mut prefixes = Vec::new();
    resource.common_prefixes("가나다".as_bytes(), |length, analyses| {
        prefixes.push((length, analyses.to_vec()));
    });

    assert_eq!(resource.stats().surface_count, 2);
    assert_eq!(
        resource.stats().resource_version,
        COMPONENT_RESOURCE_VERSION
    );
    assert_eq!(resource.stats().analysis_count, 2);
    assert_eq!(resource.stats().component_count, 2);
    assert_eq!(prefixes.len(), 2);
    assert_eq!(prefixes[1].0, "가나".len());
    assert_eq!(prefixes[1].1[0].pos, "NNG+JX");
    assert_eq!(
        prefixes[1].1[0].positions,
        [ComponentPos::NNG, ComponentPos::JX]
    );
    assert_eq!(prefixes[1].1[0].components.len(), 2);
    assert_eq!(prefixes[1].1[0].components[0].span, 0.."가".len());
    assert_eq!(prefixes[1].1[0].components[0].pos, "NNG");
    assert_eq!(
        prefixes[1].1[0].components[1].span,
        "가".len().."가나".len()
    );
    assert_eq!(prefixes[1].1[0].components[1].pos, "JX");

    let mut analysis_refs = Vec::new();
    resource.common_prefix_analysis_refs("가나다".as_bytes(), |length, analysis| {
        analysis_refs.push((length, analysis));
    });
    assert_eq!(analysis_refs.len(), 2);
    assert_eq!(analysis_refs[1].0, "가나".len());
    assert_eq!(analysis_refs[1].1.pos(), "NNG+JX");
    assert_eq!(
        analysis_refs[1].1.positions(),
        [ComponentPos::NNG, ComponentPos::JX]
    );
    assert_eq!(
        analysis_refs[1].1.components().collect::<Vec<_>>(),
        prefixes[1].1[0].components
    );
    assert!(std::mem::size_of::<ComponentAnalysisRef<'_>>() <= std::mem::size_of::<usize>() * 2);

    let mut positions = Vec::new();
    resource.common_prefix_positions("가나다".as_bytes(), |length, sequence| {
        positions.push((length, sequence.to_vec()));
    });
    assert_eq!(positions.len(), 2);
    assert_eq!(
        positions[1],
        ("가나".len(), vec![ComponentPos::NNG, ComponentPos::JX])
    );
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

    let mut version = bytes.clone();
    let version_start = MAGIC.len() + 4 + 4 + 32;
    version[version_start..version_start + RESOURCE_VERSION_LEN].fill(0);
    version[version_start..version_start + 5].copy_from_slice(b"0.0.0");
    assert!(matches!(
        decode_component_resource("fixture", version, &[7; 32])
            .unwrap_err()
            .kind
            .as_ref(),
        DataErrorKind::ComponentResourceVersionMismatch { .. }
    ));

    let mut cursor = SECTION_LENGTHS_OFFSET;
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
