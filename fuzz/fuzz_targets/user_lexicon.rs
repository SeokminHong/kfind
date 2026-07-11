#![no_main]

use std::sync::OnceLock;

use kfind_data::parse_user_lexicon_toml;
use kfind_query::Lexicons;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = parse_user_lexicon_toml("<fuzz>", &input, lexicons().rules());
});

fn lexicons() -> &'static Lexicons {
    static LEXICONS: OnceLock<Lexicons> = OnceLock::new();
    LEXICONS.get_or_init(|| Lexicons::embedded().expect("embedded lexicons must be valid"))
}
