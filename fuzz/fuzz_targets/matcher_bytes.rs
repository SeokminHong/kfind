#![no_main]

use std::sync::{Arc, OnceLock};

use kfind_matcher::MorphMatcher;
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(matched) = matcher().find_at_with_meta(data, 0) {
        assert_match_bounds(data, &matched);
    }
    for matched in matcher().find_all_with_meta(data) {
        assert_match_bounds(data, &matched);
    }
});

fn matcher() -> &'static MorphMatcher {
    static MATCHER: OnceLock<MorphMatcher> = OnceLock::new();
    MATCHER.get_or_init(|| {
        let lexicons = Lexicons::embedded().expect("embedded lexicons must be valid");
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
        let plan = compile_query("걷다", &CompileOptions::default(), &analyzer)
            .expect("fuzz query must compile");
        MorphMatcher::new(Arc::new(plan)).expect("fuzz matcher must build")
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
