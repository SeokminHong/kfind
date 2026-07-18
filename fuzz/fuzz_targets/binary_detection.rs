#![no_main]

use std::path::PathBuf;
use std::sync::OnceLock;

use kfind_matcher::MorphMatcher;
use kfind_search::{InputEncoding, InputOptions, SearchRecord, search_reader};
use libfuzzer_sys::fuzz_target;

mod support;

fuzz_target!(|data: &[u8]| {
    let selector = data.iter().fold(0_usize, |value, byte| {
        value.wrapping_mul(257).wrapping_add(usize::from(*byte))
    });
    let mut text = Vec::with_capacity(data.len().saturating_mul(3));
    for byte in data {
        match byte % 12 {
            0 => text.push(b'\n'),
            1 => text.push(b'\r'),
            2 => text.push(b' '),
            3 | 4 => text.extend_from_slice("가".as_bytes()),
            5 | 6 => text.extend_from_slice("나".as_bytes()),
            7 => text.push(b'h'),
            value => text.push(b'a' + value),
        }
    }
    let detailed_options = InputOptions {
        encoding: InputEncoding::Utf8,
        ..InputOptions::default()
    };

    for matcher in matchers() {
        let detailed = search_reader(
            matcher,
            PathBuf::from("<text>"),
            text.as_slice(),
            detailed_options,
        )
        .expect("in-memory text search must succeed");
        assert_eq!(detailed.binary_byte_offset, None);
        assert_record_bounds(&text, &detailed.records);

        let summary = search_reader(
            matcher,
            PathBuf::from("<text>"),
            text.as_slice(),
            InputOptions {
                capture_records: false,
                ..detailed_options
            },
        )
        .expect("in-memory summary search must succeed");
        assert_eq!(summary.binary_byte_offset, None);
        assert_eq!(summary.matching_lines, detailed.matching_lines);
        assert_eq!(summary.has_match(), detailed.has_match());
    }

    let text_str = std::str::from_utf8(&text).expect("generated text must be valid UTF-8");
    let boundary_count = text_str.char_indices().count() + 1;
    let nul_offset = text_str
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(text.len()))
        .nth(selector % boundary_count)
        .expect("text has at least the final UTF-8 boundary");
    let mut binary = text;
    binary.insert(nul_offset, b'\0');
    let binary_result = search_reader(
        &matchers()[0],
        PathBuf::from("<binary>"),
        binary.as_slice(),
        detailed_options,
    )
    .expect("in-memory binary search must succeed");
    assert_eq!(binary_result.binary_byte_offset, Some(nul_offset as u64));
});

fn matchers() -> &'static [MorphMatcher; 2] {
    static MATCHERS: OnceLock<[MorphMatcher; 2]> = OnceLock::new();
    MATCHERS.get_or_init(|| {
        [
            support::build_match_fixture().1,
            support::build_phrase_fixture(),
        ]
    })
}

fn assert_record_bounds(input: &[u8], records: &[SearchRecord]) {
    for record in records {
        let SearchRecord::Line(line) = record else {
            continue;
        };
        for matched in &line.matches {
            assert!(matched.span.start <= matched.span.end);
            assert!(matched.span.end <= line.bytes.len());
            let absolute_end = line
                .absolute_byte_offset
                .saturating_add(matched.span.end as u64);
            assert!(absolute_end <= input.len() as u64);
        }
    }
}
