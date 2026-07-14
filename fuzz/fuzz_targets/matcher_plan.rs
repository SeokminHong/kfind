#![no_main]

use std::sync::{Arc, OnceLock};

use kfind_matcher::MorphMatcher;
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, PhrasePolicy, compile_query};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&gap_seed, query)) = data.split_first() else {
        return;
    };
    let source = String::from_utf8_lossy(query);
    let options = CompileOptions {
        phrase: PhrasePolicy {
            max_gap: usize::from(gap_seed) * 32,
        },
        ..CompileOptions::default()
    };
    if let Ok(plan) = compile_query(&source, &options, analyzer()) {
        let _ = MorphMatcher::new(Arc::new(plan));
    }
});

fn analyzer() -> &'static LexiconQueryAnalyzer {
    static ANALYZER: OnceLock<LexiconQueryAnalyzer> = OnceLock::new();
    ANALYZER.get_or_init(|| {
        let lexicons = Lexicons::embedded().expect("embedded lexicons must be valid");
        LexiconQueryAnalyzer::new(Arc::new(lexicons))
    })
}
