use std::sync::{Arc, OnceLock};

use kfind_matcher::MorphMatcher;
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};
use proptest::prelude::*;

const VERIFIED_TOKEN: &[u8] = "걸었습니다".as_bytes();

proptest! {
    #[test]
    fn verified_spans_stay_inside_malformed_input(
        prefix in prop::collection::vec(any::<u8>(), 0..64),
        suffix in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let token_start = prefix.len();
        let mut haystack = prefix;
        haystack.extend_from_slice(VERIFIED_TOKEN);
        haystack.extend_from_slice(&suffix);

        if let Some(matched) = matcher().find_at_with_meta(&haystack, token_start) {
            prop_assert!(matched.span.start <= matched.span.end);
            prop_assert!(matched.span.end <= haystack.len());
            for atom in matched.atoms {
                prop_assert!(atom.core.start <= atom.core.end);
                prop_assert!(atom.core.end <= haystack.len());
                prop_assert!(atom.token.start <= atom.token.end);
                prop_assert!(atom.token.end <= haystack.len());
                prop_assert!(matched.span.start <= atom.token.start);
                prop_assert!(atom.token.end <= matched.span.end);
                prop_assert!(atom.token.start <= atom.core.start);
                prop_assert!(atom.core.end <= atom.token.end);
            }
        }
    }
}

fn matcher() -> &'static MorphMatcher {
    static MATCHER: OnceLock<MorphMatcher> = OnceLock::new();
    MATCHER.get_or_init(|| {
        let lexicons = Lexicons::embedded().expect("embedded lexicons must be valid");
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
        let plan = compile_query("걷다", &CompileOptions::default(), &analyzer)
            .expect("property-test query must compile");
        MorphMatcher::new(Arc::new(plan)).expect("property-test matcher must build")
    })
}
