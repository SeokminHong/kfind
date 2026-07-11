use std::sync::Arc;

use kfind_matcher::MorphMatcher;
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, QueryPlan, compile_query};

pub(crate) fn build_match_fixture() -> (Arc<QueryPlan>, MorphMatcher) {
    let lexicons = Arc::new(Lexicons::embedded().expect("embedded lexicons must be valid"));
    let analyzer = LexiconQueryAnalyzer::new(Arc::clone(&lexicons));
    let plan = Arc::new(
        compile_query("걷다", &CompileOptions::default(), &analyzer)
            .expect("fuzz query must compile"),
    );
    let matcher =
        MorphMatcher::new(Arc::clone(&plan)).expect("fuzz matcher must build from query plan");
    (plan, matcher)
}
