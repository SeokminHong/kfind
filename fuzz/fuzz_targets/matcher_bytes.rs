#![no_main]

use std::borrow::Cow;
use std::sync::OnceLock;

use kfind_matcher::MorphMatcher;
use libfuzzer_sys::fuzz_target;

mod support;

fuzz_target!(|data: &[u8]| {
    let data = decode_hex_seed(data);
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

fn decode_hex_seed(data: &[u8]) -> Cow<'_, [u8]> {
    let Some(hex) = data.strip_prefix(b"hex:") else {
        return Cow::Borrowed(data);
    };
    if hex.len() % 2 != 0 {
        return Cow::Borrowed(data);
    }
    let mut decoded = Vec::with_capacity(hex.len() / 2);
    for pair in hex.chunks_exact(2) {
        let Some(high) = hex_digit(pair[0]) else {
            return Cow::Borrowed(data);
        };
        let Some(low) = hex_digit(pair[1]) else {
            return Cow::Borrowed(data);
        };
        decoded.push((high << 4) | low);
    }
    Cow::Owned(decoded)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
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
