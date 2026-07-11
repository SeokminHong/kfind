#![no_main]

use std::path::PathBuf;
use std::sync::OnceLock;

use kfind_matcher::MorphMatcher;
use kfind_search::{InputEncoding, InputOptions, search_reader};
use libfuzzer_sys::fuzz_target;

mod support;

fuzz_target!(|data: &[u8]| {
    let selector = data.iter().fold(0_usize, |value, byte| {
        value.wrapping_mul(257).wrapping_add(usize::from(*byte))
    });
    let text = data
        .iter()
        .map(|byte| match byte % 17 {
            0 => b'\n',
            value => b'a' + value,
        })
        .collect::<Vec<_>>();
    let options = InputOptions {
        encoding: InputEncoding::Utf8,
        ..InputOptions::default()
    };

    let text_result = search_reader(matcher(), PathBuf::from("<text>"), text.as_slice(), options)
        .expect("in-memory text search must succeed");
    assert_eq!(text_result.binary_byte_offset, None);

    let nul_offset = selector % (text.len() + 1);
    let mut binary = text;
    binary.insert(nul_offset, b'\0');
    let binary_result = search_reader(
        matcher(),
        PathBuf::from("<binary>"),
        binary.as_slice(),
        options,
    )
    .expect("in-memory binary search must succeed");
    assert_eq!(binary_result.binary_byte_offset, Some(nul_offset as u64));
});

fn matcher() -> &'static MorphMatcher {
    static MATCHER: OnceLock<MorphMatcher> = OnceLock::new();
    MATCHER.get_or_init(|| support::build_match_fixture().1)
}
