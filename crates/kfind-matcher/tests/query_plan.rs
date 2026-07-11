use std::sync::Arc;

use kfind_matcher::MorphMatcher;
use kfind_query::{
    BoundaryPolicy, CompileOptions, ExpandMode, LexiconQueryAnalyzer, Lexicons, NormalizationMode,
    compile_query,
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
fn compiled_predicate_plan_rejects_a_surface_attached_as_a_particle() {
    let matcher = compile("가다", CompileOptions::default());

    assert!(
        matcher
            .find_at_with_meta("친구가 먹었다.".as_bytes(), 0)
            .is_none()
    );
    assert!(
        matcher
            .find_at_with_meta("친구가 간다.".as_bytes(), 0)
            .is_some()
    );
}

#[test]
fn derivation_adverb_plan_matches_only_auxiliary_particles() {
    let matcher = compile(
        "빨리",
        CompileOptions {
            expand: ExpandMode::Derivation,
            ..CompileOptions::default()
        },
    );

    assert!(
        matcher
            .find_at_with_meta("일을 빨리도 끝냈다.".as_bytes(), 0)
            .is_some()
    );
    assert!(
        matcher
            .find_at_with_meta("빨리가 답이다.".as_bytes(), 0)
            .is_none()
    );
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
fn compiled_nominal_plan_enforces_particle_transitions() {
    let matcher = compile("사용자", CompileOptions::default());

    for text in ["사용자는은", "사용자도만"] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "accepted forbidden particle chain {text}"
        );
    }
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

#[test]
fn compiled_copula_plan_accepts_attached_adnominals_and_licensed_contraction() {
    let matcher = compile("이다", CompileOptions::default());

    for text in [
        "학생인 친구가 도착했다.",
        "학생일 때 많이 배웠다.",
        "학교여서 도보로 갔다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "compiled copula plan rejected {text}"
        );
    }
    assert!(
        matcher
            .find_at_with_meta("학생이여서 참석했다.".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn compiled_copula_environment_is_canonical_normalization_safe() {
    let options = CompileOptions {
        normalization: NormalizationMode::Canonical,
        ..CompileOptions::default()
    };
    let matcher = compile("이다", options);
    let accepted = "학교여서".nfd().collect::<String>();
    let rejected = "학생이여서".nfd().collect::<String>();

    assert!(matcher.find_at_with_meta(accepted.as_bytes(), 0).is_some());
    assert!(matcher.find_at_with_meta(rejected.as_bytes(), 0).is_none());
}

#[test]
fn nominal_overrides_replace_the_same_base_particle_path() {
    for (query, override_form, rejected) in [
        ("나", "내가", "나가"),
        ("너", "네가", "너가"),
        ("저", "제가", "저가"),
    ] {
        let matcher = compile(query, CompileOptions::default());
        assert!(
            matcher
                .find_at_with_meta(override_form.as_bytes(), 0)
                .is_some(),
            "{query} must accept {override_form}"
        );
        assert!(
            matcher.find_at_with_meta(rejected.as_bytes(), 0).is_none(),
            "{query} must reject {rejected}"
        );
        let topic = format!("{query}는");
        assert!(matcher.find_at_with_meta(topic.as_bytes(), 0).is_some());
    }
}

#[test]
fn direct_particle_plans_validate_the_attached_host_in_smart_mode() {
    for (query, accepted, rejected) in [
        ("는", ["사용자는", "권한은"], ["사용자은", "권한는"]),
        ("로", ["길로", "학교로"], ["길으로", "집로"]),
    ] {
        let matcher = compile(query, CompileOptions::default());
        for text in accepted {
            assert!(
                matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
                "direct particle query {query:?} rejected {text:?}"
            );
        }
        for text in rejected {
            assert!(
                matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
                "direct particle query {query:?} accepted {text:?}"
            );
        }
    }
}

#[test]
fn direct_particle_plans_preserve_token_and_any_boundary_modes() {
    let token = compile(
        "는",
        CompileOptions {
            boundary: BoundaryPolicy::Token,
            ..CompileOptions::default()
        },
    );
    assert!(token.find_at_with_meta("는".as_bytes(), 0).is_some());
    assert!(token.find_at_with_meta("사용자는".as_bytes(), 0).is_none());

    let any = compile(
        "는",
        CompileOptions {
            boundary: BoundaryPolicy::Any,
            ..CompileOptions::default()
        },
    );
    assert!(any.find_at_with_meta("권한는".as_bytes(), 0).is_some());
}

fn compile(query: &str, options: CompileOptions) -> MorphMatcher {
    let lexicons = Arc::new(Lexicons::embedded().expect("embedded lexicons must be valid"));
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan = compile_query(query, &options, &analyzer).expect("query must compile");
    MorphMatcher::new(Arc::new(plan)).expect("matcher must build")
}
