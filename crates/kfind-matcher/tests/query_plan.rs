use std::sync::Arc;

use kfind_matcher::MorphMatcher;
use kfind_morph::CoarsePos;
use kfind_query::{
    BoundaryPolicy, CompileOptions, ContextRequirement, ExpandMode, LexiconQueryAnalyzer, Lexicons,
    NormalizationMode, compile_query,
};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Copy)]
struct VcpBoundaryFixture {
    case_name: &'static str,
    text: &'static str,
    gold_vcp: bool,
}

const VCP_BOUNDARY_FIXTURES: [VcpBoundaryFixture; 7] = [
    VcpBoundaryFixture {
        case_name: "ud-korean-ksl/dev/KH-B100001-32-1-8",
        text: "매일 양복이 입고 너무 비싼 장소를 돌어간다.",
        gold_vcp: false,
    },
    VcpBoundaryFixture {
        case_name: "constructed/student+VCP-ETM",
        text: "학생일 가능성이 있다.",
        gold_vcp: true,
    },
    VcpBoundaryFixture {
        case_name: "constructed/book+VCP-ETM",
        text: "책일 가능성이 있다.",
        gold_vcp: true,
    },
    VcpBoundaryFixture {
        case_name: "ud-korean-ksl/dev/KH-B200000-2-3224",
        text: "고전소설인 책에서 생각보다 심한 사회 문제가 반영했다.",
        gold_vcp: true,
    },
    VcpBoundaryFixture {
        case_name: "ud-korean-ksl/dev/KH-C100007-2-5589",
        text: "근데, 가장 의미가 있은 날은 고등학교 출업한 날이다.",
        gold_vcp: true,
    },
    VcpBoundaryFixture {
        case_name: "ud-korean-ksl/dev/KH-B200085-42-2-12",
        text: "하지만, 보고 나니까 우정에 대한 영화인 것을 알게 됐다.",
        gold_vcp: true,
    },
    VcpBoundaryFixture {
        case_name: "ud-korean-ksl/dev/KH-B100002-32-1-3",
        text: "고향은 12월부터 3월까지 겨울입니다.",
        gold_vcp: true,
    },
];

#[test]
fn compiled_predicate_plan_matches_irregular_and_homonymous_surfaces() {
    let matcher = compile("걷다", CompileOptions::default());

    for text in [
        "길을 걸어 갔다.",
        "손님이 오래 걸었습니다.",
        "천천히 걸으셨다.",
        "천천히 걸으시겠습니다.",
        "전화를 걸어 봤다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "compiled 걷다 plan rejected {text}"
        );
    }
}

#[test]
fn compiled_predicate_plan_applies_ending_pos_requirements() {
    let verb = compile("가다", CompileOptions::default());
    let adjective = compile("예쁘다", CompileOptions::default());

    assert!(verb.find_at_with_meta("어서 가라".as_bytes(), 0).is_some());
    assert!(
        adjective
            .find_at_with_meta("꽃이 예뻐라".as_bytes(), 0)
            .is_none()
    );
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
    assert!(
        matcher
            .find_at_with_meta("어르신이 가셨다.".as_bytes(), 0)
            .is_some()
    );
    assert!(
        matcher
            .find_at_with_meta("내일 가십니다.".as_bytes(), 0)
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
fn compiled_gi_nominalizer_consumes_only_valid_particle_chains() {
    for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Token] {
        let matcher = compile(
            "걷다",
            CompileOptions {
                boundary,
                ..CompileOptions::default()
            },
        );
        for (text, token) in [
            ("매일 걷기가 즐겁다.", "걷기가"),
            ("오래 걷기를 권했다.", "걷기를"),
            ("걷기에서도 배운다.", "걷기에서도"),
        ] {
            let matched = matcher
                .find_at_with_meta(text.as_bytes(), 0)
                .unwrap_or_else(|| panic!("rejected nominalized particle chain {text}"));
            let atom = &matched.atoms[0];
            assert_eq!(&text[atom.token.clone()], token);
            assert!(
                atom.origins[0]
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str() == "ending.nominalizer-gi")
            );
            assert!(
                atom.origins[0]
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str().starts_with("particle."))
            );
        }

        for text in [
            "걷기이 어렵다.",
            "걷기을 권했다.",
            "걷기으로 충분하다.",
            "걷기가를 권했다.",
        ] {
            assert!(
                matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
                "accepted invalid nominalized particle chain {text}"
            );
        }
    }
}

#[test]
fn any_boundary_keeps_invalid_suffix_candidates_and_extends_valid_tokens() {
    let matcher = compile(
        "걷다",
        CompileOptions {
            boundary: BoundaryPolicy::Any,
            ..CompileOptions::default()
        },
    );
    let valid = "걷기가";
    let matched = matcher
        .find_at_with_meta(valid.as_bytes(), 0)
        .expect("any boundary should retain a valid nominalizer candidate");
    assert_eq!(&valid[matched.atoms[0].token.clone()], valid);
    assert!(matcher.find_at_with_meta("걷기을".as_bytes(), 0).is_some());
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
fn compiled_vcp_plan_accepts_corpus_attestations_and_licensed_contraction() {
    let matcher = compile("이다", CompileOptions::default());

    for fixture in VCP_BOUNDARY_FIXTURES
        .iter()
        .filter(|fixture| fixture.gold_vcp)
    {
        assert!(
            matcher
                .find_at_with_meta(fixture.text.as_bytes(), 0)
                .is_some(),
            "compiled VCP plan rejected {} ({})",
            fixture.case_name,
            fixture.text
        );
    }
    assert!(
        matcher
            .find_at_with_meta("학생이여서 참석했다.".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn smart_vcp_corpus_fixtures_preserve_union_results() {
    let matcher = compile("이다", CompileOptions::default());
    assert!(
        matcher.plan().atoms[0]
            .branches
            .iter()
            .all(|branch| branch.context_requirement == ContextRequirement::PredicateLexical)
    );

    for fixture in VCP_BOUNDARY_FIXTURES {
        assert!(
            matcher
                .find_at_with_meta(fixture.text.as_bytes(), 0)
                .is_some(),
            "union result differed for {} ({})",
            fixture.case_name,
            fixture.text
        );
    }
}

#[test]
fn local_analysis_candidates_preserve_window_limit_errors() {
    let matcher = compile("권한", CompileOptions::default());
    let text = format!("{}권한", "가".repeat(90));
    let candidates = matcher.local_analysis_candidates(text.as_bytes());

    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| matches!(
        candidate.window,
        Err(kfind_matcher::AnalysisWindowError::RawBytes { .. })
    )));
}

#[test]
fn canonical_vcp_corpus_fixtures_preserve_union_results() {
    let options = CompileOptions {
        normalization: NormalizationMode::Canonical,
        ..CompileOptions::default()
    };
    let matcher = compile("이다", options);

    for fixture in VCP_BOUNDARY_FIXTURES {
        let decomposed = fixture.text.nfd().collect::<String>();
        assert!(
            matcher
                .find_at_with_meta(decomposed.as_bytes(), 0)
                .is_some(),
            "canonical union result differed for {} ({})",
            fixture.case_name,
            fixture.text
        );
    }
    assert!(
        matcher.plan().atoms[0]
            .branches
            .iter()
            .all(|branch| branch.context_requirement == ContextRequirement::PredicateLexical)
    );
}

#[test]
fn compiled_vcp_environment_is_canonical_normalization_safe() {
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
    let options = CompileOptions {
        global_pos: Some(CoarsePos::Particle),
        ..CompileOptions::default()
    };
    for (query, accepted, rejected) in [
        ("는", ["사용자는", "권한은"], ["사용자은", "권한는"]),
        ("로", ["길로", "학교로"], ["길으로", "집로"]),
    ] {
        let matcher = compile(query, options.clone());
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
fn untagged_smart_direct_particle_keeps_the_typed_surface() {
    let smart = compile("이", CompileOptions::default());
    assert!(smart.find_at_with_meta("집이".as_bytes(), 0).is_some());
    assert!(smart.find_at_with_meta("날씨가".as_bytes(), 0).is_none());

    let any = compile(
        "이",
        CompileOptions {
            boundary: BoundaryPolicy::Any,
            ..CompileOptions::default()
        },
    );
    assert!(any.find_at_with_meta("날씨가".as_bytes(), 0).is_some());
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
