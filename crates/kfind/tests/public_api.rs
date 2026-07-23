use kfind::{
    AnalyzeError, AnchorBuildError, CompileErrorKind, CompileOptions, DataErrorKind, Engine,
    GenerateError, LexicalAlternation, MorphMatcherBuildError, Origin, PhraseMatch, PhrasePolicy,
    PlanLimits, QueryError, QueryErrorKind, RuleId, SourceLocation, SourceSpan, VerifiedSpan,
};

#[test]
fn stable_facade_exposes_named_match_provenance() {
    let engine = Engine::new().unwrap();
    let matcher = engine.compile("걷다", &CompileOptions::default()).unwrap();
    let matches: Vec<PhraseMatch> = matcher.find_all("길을 걸었다".as_bytes());
    let atom: &VerifiedSpan = &matches[0].atoms[0];
    let origin: &Origin = &atom.origins[0];
    let rule: &RuleId = &origin.rule_path[0];

    assert!(!rule.as_str().is_empty());
}

#[test]
fn disjunction_finds_each_alternative_in_source_order() {
    let engine = Engine::new().unwrap();
    let matcher = engine
        .compile("lit:alpha|lit:beta", &CompileOptions::default())
        .unwrap();

    let matches = matcher.find_all(b"beta then alpha");

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].span, 0.."beta".len());
    assert_eq!(matches[1].span, "beta then ".len().."beta then alpha".len());
}

#[test]
fn disjunction_merges_provenance_for_the_same_span() {
    let engine = Engine::new().unwrap();
    let matcher = engine
        .compile("lit:alpha|lit:alpha", &CompileOptions::default())
        .unwrap();

    let matches = matcher.find_all(b"alpha");
    let analysis_indices = matches[0].atoms[0]
        .origins
        .iter()
        .map(|origin| origin.analysis_index)
        .collect::<Vec<_>>();

    assert_eq!(analysis_indices, vec![0, 1]);
}

#[test]
fn stable_facade_names_option_and_error_field_types() {
    let options = CompileOptions {
        phrase: PhrasePolicy::default(),
        limits: PlanLimits::default(),
        ..CompileOptions::default()
    };

    assert_eq!(options.phrase.max_gap, 24);
    assert_named::<AnalyzeError>();
    assert_named::<AnchorBuildError>();
    assert_named::<CompileErrorKind>();
    assert_named::<DataErrorKind>();
    assert_named::<GenerateError>();
    assert_named::<LexicalAlternation>();
    assert_named::<MorphMatcherBuildError>();
    assert_named::<QueryError>();
    assert_named::<QueryErrorKind>();
    assert_named::<SourceLocation>();
    assert_named::<SourceSpan>();
}

#[test]
fn expert_api_requires_an_explicit_module_import() {
    use kfind::expert::{EngineExt as _, Lexicons, MatcherExt as _, QueryPlan};

    let engine = Engine::from_lexicons(Lexicons::embedded().unwrap());
    let matcher = engine.compile("걷다", &CompileOptions::default()).unwrap();
    let plan: &QueryPlan = matcher.plan();

    assert_eq!(plan.atoms.len(), 1);
}

fn assert_named<T>() {}
