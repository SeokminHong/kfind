#![no_main]

use kfind_data::{
    MecabSourceMorphologyEntry, decode_component_resource, encode_component_resource,
};
use libfuzzer_sys::fuzz_target;

mod seed;

const SOURCE_DIGEST: [u8; 32] = [0xa5; 32];
const STRUCTURED_PREFIX: &[u8] = b"valid:";
const SURFACE_SCALARS: [char; 8] = ['가', '나', '다', '라', '마', '바', '사', '아'];
const POS_TAGS: [&str; 4] = ["NNG", "NNP", "MAG", "VV"];

fuzz_target!(|data: &[u8]| {
    let data = seed::decode_hex(data);
    if let Some(structured) = data.strip_prefix(STRUCTURED_PREFIX) {
        exercise_valid_resource(structured);
    }
    let _ = decode_component_resource("<fuzz>", data.into_owned(), &SOURCE_DIGEST);
});

fn exercise_valid_resource(data: &[u8]) {
    let mut entries = vec![
        atomic_entry("가", "NNG", 0),
        MecabSourceMorphologyEntry {
            surface: "가나".to_owned(),
            pos: "NNG+JX".to_owned(),
            left_id: 0,
            right_id: 0,
            word_cost: 0,
            analysis_type: "Inflect".to_owned(),
            start_pos: "NNG".to_owned(),
            end_pos: "JX".to_owned(),
            expression: "가/NNG/*+나/JX/*".to_owned(),
        },
    ];
    for chunk in data.chunks(4).take(16) {
        let surface = chunk
            .iter()
            .map(|byte| SURFACE_SCALARS[usize::from(*byte) % SURFACE_SCALARS.len()])
            .collect::<String>();
        if surface.is_empty() {
            continue;
        }
        let selector = usize::from(chunk[0]);
        entries.push(atomic_entry(
            &surface,
            POS_TAGS[selector % POS_TAGS.len()],
            i32::from(chunk[0]),
        ));
    }

    let bytes = encode_component_resource(SOURCE_DIGEST, &entries)
        .expect("constructed component entries must encode");
    let resource = decode_component_resource("<fuzz-valid>", bytes, &SOURCE_DIGEST)
        .expect("encoded component resource must decode");
    assert!(resource.stats().surface_count > 0);
    for entry in entries.iter().take(16) {
        let mut referenced = 0_usize;
        resource.common_prefix_analysis_refs(entry.surface.as_bytes(), |length, analysis| {
            assert!(length <= entry.surface.len());
            assert!(!analysis.pos().is_empty());
            assert!(!analysis.positions().is_empty());
            let components = analysis.components();
            assert_eq!(components.clone().count(), components.len());
            for component in components {
                assert!(component.span.start < component.span.end);
                assert!(component.span.end <= length);
                assert!(!component.pos.is_empty());
            }
            referenced += 1;
        });
        assert!(referenced > 0);
        resource.common_prefixes(entry.surface.as_bytes(), |length, analyses| {
            assert!(length <= entry.surface.len());
            assert!(!analyses.is_empty());
        });
    }
}

fn atomic_entry(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 0,
        right_id: 0,
        word_cost,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: "*".to_owned(),
    }
}
