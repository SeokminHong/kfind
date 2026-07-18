use std::sync::Arc;

use kfind_matcher::MorphMatcher;
use kfind_query::{
    BoundaryPolicy, CompileOptions, LexiconQueryAnalyzer, Lexicons, PhrasePolicy, QueryPlan,
    compile_query,
};

#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) fn build_phrase_fixture() -> MorphMatcher {
    let lexicons = Arc::new(Lexicons::embedded().expect("embedded lexicons must be valid"));
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let options = CompileOptions {
        boundary: BoundaryPolicy::Any,
        phrase: PhrasePolicy { max_gap: 4_096 },
        ..CompileOptions::default()
    };
    let plan = compile_query(
        "lit:가 lit:가 lit:가 lit:가 lit:가 lit:가 lit:가 lit:가",
        &options,
        &analyzer,
    )
    .expect("phrase fuzz query must compile");
    MorphMatcher::new(Arc::new(plan)).expect("phrase fuzz matcher must build")
}
