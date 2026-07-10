use std::sync::Arc;

use kfind_matcher::MorphMatcher;
use kfind_query::{
    CompileOptions, LexiconQueryAnalyzer, Lexicons, NormalizationMode, compile_query,
};
use unicode_normalization::UnicodeNormalization;

#[test]
fn compiled_predicate_plan_matches_irregular_and_homonymous_surfaces() {
    let matcher = compile("걷다", CompileOptions::default());

    for text in [
        "길을 걸어 갔다.",
        "손님이 오래 걸었습니다.",
        "천천히 걸으셨다.",
        "전화를 걸어 봤다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "compiled 걷다 plan rejected {text}"
        );
    }
}

#[test]
fn compiled_plans_reject_unlicensed_predicate_and_particle_surfaces() {
    let pretty = compile("예쁘다", CompileOptions::default());
    assert!(
        pretty
            .find_at_with_meta("꽃이 예쁜 모습이다".as_bytes(), 0)
            .is_some()
    );
    assert!(
        pretty
            .find_at_with_meta("꽃이 예쁘어 보인다".as_bytes(), 0)
            .is_none()
    );

    let road = compile("길", CompileOptions::default());
    assert!(
        road.find_at_with_meta("길로 들어섰다".as_bytes(), 0)
            .is_some()
    );
    assert!(
        road.find_at_with_meta("길으로 들어섰다".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn compiled_nominal_plan_keeps_core_and_consumed_particle_span() {
    let matcher = compile("사용자", CompileOptions::default());
    let text = "사용자들에게 알렸다.";
    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("compiled nominal plan should consume a particle chain");

    assert_eq!(&text[matched.atoms[0].core.clone()], "사용자");
    assert_eq!(&text[matched.atoms[0].token.clone()], "사용자들에게");
    assert!(
        matched.atoms[0].origins[0]
            .rule_path
            .iter()
            .any(|rule| rule.as_str() == "particle.plural")
    );
}

#[test]
fn compiled_phrase_plan_joins_verified_atoms_without_a_surface_product() {
    let mut options = CompileOptions::default();
    options.phrase.max_gap = 4;
    let matcher = compile("권한 검증하다", options);
    let text = "권한을 먼저 검증했다";
    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("compiled phrase should join atom spans");

    assert_eq!(matched.atoms.len(), 2);
    assert_eq!(matched.span, 0..text.len());
}

#[test]
fn compiled_canonical_plan_matches_nfd_inflection() {
    let options = CompileOptions {
        normalization: NormalizationMode::Canonical,
        ..CompileOptions::default()
    };
    let matcher = compile("걷다", options);
    let text = "천천히 걸었습니다".nfd().collect::<String>();

    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("canonical plan should verify an NFD continuation");
    assert_eq!(matched.span.end, text.len());
}

fn compile(query: &str, options: CompileOptions) -> MorphMatcher {
    let lexicons = Arc::new(Lexicons::embedded().expect("embedded lexicons must be valid"));
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan = compile_query(query, &options, &analyzer).expect("query must compile");
    MorphMatcher::new(Arc::new(plan)).expect("matcher must build")
}
