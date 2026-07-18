#![no_main]

use std::sync::OnceLock;

use kfind_matcher::MorphMatcher;
use libfuzzer_sys::fuzz_target;

mod seed;
mod support;

fuzz_target!(|data: &[u8]| {
    let data = seed::decode_hex(data);
    for matcher in matchers() {
        if let Some(matched) = matcher.find_at_with_meta(&data, 0) {
            assert_match_bounds(&data, &matched);
        }
        for matched in matcher.find_all_with_meta(&data) {
            assert_match_bounds(&data, &matched);
        }
    }
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

fn assert_match_bounds(data: &[u8], matched: &kfind_query::PhraseMatch) {
    assert!(matched.span.start <= matched.span.end);
    assert!(matched.span.end <= data.len());
    for atom in &matched.atoms {
        assert!(atom.core.start <= atom.core.end);
        assert!(atom.core.end <= data.len());
        assert!(atom.token.start <= atom.token.end);
        assert!(atom.token.end <= data.len());
        assert!(matched.span.start <= atom.token.start);
        assert!(atom.token.end <= matched.span.end);
        assert!(atom.token.start <= atom.core.start);
        assert!(atom.core.end <= atom.token.end);
    }
}
