use std::sync::{Arc, OnceLock};

use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, ComponentResource, DataAlternation, DataFinePos, LexiconData,
    MecabSourceMorphologyEntry, NominalRecord, PredicateRecord, collect_pos_entries,
    decode_component_resource, encode_component_resource, encode_pos_lexicon,
};
use kfind_matcher::MorphMatcher;
use kfind_morph::CoarsePos;
use kfind_query::{
    BoundaryPolicy, CompileOptions, ExpandMode, LexiconQueryAnalyzer, Lexicons, NormalizationMode,
    compile_query,
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
fn full_pos_smart_predicate_plan_preserves_a_same_pos_homograph_union() {
    for query in ["걷다", "걸다"] {
        let matcher = compile_with_full_pos(
            query,
            CompileOptions {
                global_pos: Some(CoarsePos::Verb),
                ..CompileOptions::default()
            },
        );

        assert!(
            matcher
                .find_at_with_meta("전화를 걸었어.".as_bytes(), 0)
                .is_some(),
            "compiled {query} plan rejected the shared homographic form"
        );
    }
}

#[test]
fn contracted_aoeo_program_consumes_a_proven_auxiliary_sequence() {
    let matcher = compile_with_full_pos(
        "빼다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );

    assert!(
        matcher
            .find_at_with_meta("빼놓을 수 없다.".as_bytes(), 0)
            .is_some()
    );
    assert!(
        matcher
            .find_at_with_meta("빼문서는 없다.".as_bytes(), 0)
            .is_none()
    );

    let contracted = compile_with_full_pos(
        "비추다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );
    assert!(
        contracted
            .find_at_with_meta("매출액에 비춰볼 때.".as_bytes(), 0)
            .is_some()
    );
}

#[test]
fn generated_predicate_branch_consumes_a_complete_source_ending_path() {
    let matcher = compile_with_full_pos(
        "오다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );

    for text in [
        "눈이 왔으니까.",
        "오래전부터 왔었다.",
        "그가 왔다는 말이다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "complete source ending path was rejected for {text}"
        );
    }
    assert!(
        matcher
            .find_at_with_meta("문서에는 왔다를 적었다.".as_bytes(), 0)
            .is_none()
    );
    assert!(
        matcher
            .find_at_with_meta("친구가 먼저 들어왔었다.".as_bytes(), 0)
            .is_none()
    );

    let prefix = compile_with_full_pos(
        "말다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );
    assert!(
        prefix
            .find_at_with_meta("만들려 한다.".as_bytes(), 0)
            .is_none()
    );

    let other_pos = compile_with_full_pos(
        "하다",
        CompileOptions {
            global_pos: Some(CoarsePos::Adjective),
            ..CompileOptions::default()
        },
    );
    assert!(
        other_pos
            .find_at_with_meta("겨울이 없을 거라고 한다.".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn declarative_adnominal_uses_a_complete_source_ending_path() {
    for (query, text) in [
        ("오다", "그가 왔다는 말이다."),
        ("있다", "문제가 있다는 뜻이다."),
        ("않다", "쉽지 않다는 결론이다."),
    ] {
        let matcher = compile_with_full_pos(
            query,
            CompileOptions {
                global_pos: Some(CoarsePos::Verb),
                ..CompileOptions::default()
            },
        );

        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "declarative adnominal source path was rejected for {query} in {text}"
        );
    }
}

#[test]
fn connective_topic_uses_an_ending_then_particle_source_path() {
    for (query, pos, text) in [
        (
            "위하다",
            CoarsePos::Verb,
            "취업하기 위해서는 준비가 필요하다.",
        ),
        (
            "대하다",
            CoarsePos::Verb,
            "그 문제에 대해서는 의견이 다르다.",
        ),
        ("없다", CoarsePos::Adjective, "문제가 없지는 않다."),
    ] {
        let matcher = compile_with_full_pos(
            query,
            CompileOptions {
                global_pos: Some(pos),
                ..CompileOptions::default()
            },
        );

        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "connective topic source path was rejected for {query} in {text}"
        );
    }

    for (query, pos, text) in [
        ("위하다", CoarsePos::Verb, "문서에 위해서를 적었다."),
        ("없다", CoarsePos::Adjective, "문서에 없지를 적었다."),
    ] {
        let matcher = compile_with_full_pos(
            query,
            CompileOptions {
                global_pos: Some(pos),
                ..CompileOptions::default()
            },
        );

        assert!(matcher.find_at_with_meta(text.as_bytes(), 0).is_none());
    }
}

#[test]
fn adnominal_dependent_noun_particle_uses_a_complete_source_path() {
    let matcher = compile_with_full_pos(
        "오다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );
    let text = "지금의 세계를 만들어 온지를 배운다.";
    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("adnominal dependent-noun source path was rejected");

    assert_eq!(&text[matched.atoms[0].core.clone()], "온");
    for rejected in ["온지", "온를"] {
        assert!(
            matcher.find_at_with_meta(rejected.as_bytes(), 0).is_none(),
            "accepted incomplete adnominal dependent-noun path {rejected}"
        );
    }
}

#[test]
fn adnominal_interrogative_uses_a_complete_source_predicate_path() {
    let options = CompileOptions {
        global_pos: Some(CoarsePos::Adjective),
        ..CompileOptions::default()
    };
    for matcher in [
        compile_embedded_with_component("어떻다", options.clone()),
        compile_with_full_pos("어떻다", options),
    ] {
        let text = "반면 미국은 어떤가.";
        let matched = matcher
            .find_at_with_meta(text.as_bytes(), 0)
            .expect("adnominal interrogative source path was rejected");

        assert_eq!(&text[matched.atoms[0].core.clone()], "어떤");
        assert!(
            matcher
                .find_at_with_meta("어떤가를".as_bytes(), 0)
                .is_none()
        );
    }
    assert!(
        compile("어떻다", CompileOptions::default())
            .find_at_with_meta("어떤가".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn smart_auxiliary_query_accepts_a_complete_attached_source_path() {
    let matcher = compile_with_full_pos(
        "지다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );

    for text in [
        "수입으로 메꾸어졌다.",
        "축구장에서 떨어진 공이다.",
        "낮시간이 길어진 기분이다.",
        "사정이 달라졌다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "attached auxiliary source path was rejected for {text}"
        );
    }
    assert!(
        matcher
            .find_at_with_meta("사진을 걸었다.".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn adjacent_layout_limits_disambiguation_to_supported_pos_competitions() {
    let noun = compile_with_full_pos(
        "새",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
    );
    let determiner = compile_with_full_pos(
        "새",
        CompileOptions {
            global_pos: Some(CoarsePos::Determiner),
            ..CompileOptions::default()
        },
    );
    let connective = compile_with_full_pos(
        "주다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );
    let adnominal = compile_with_full_pos(
        "걸다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );
    let particle_host = compile_with_full_pos(
        "학교",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
    );
    let adverb = compile_with_full_pos(
        "너무",
        CompileOptions {
            global_pos: Some(CoarsePos::Adverb),
            ..CompileOptions::default()
        },
    );
    let pronoun = compile_with_full_pos(
        "제",
        CompileOptions {
            global_pos: Some(CoarsePos::Pronoun),
            ..CompileOptions::default()
        },
    );
    let numeral = compile_with_full_pos(
        "한",
        CompileOptions {
            global_pos: Some(CoarsePos::Numeral),
            ..CompileOptions::default()
        },
    );

    assert!(noun.find_at_with_meta("새 기능".as_bytes(), 0).is_none());
    assert!(
        determiner
            .find_at_with_meta("새 기능".as_bytes(), 0)
            .is_some()
    );
    assert!(
        connective
            .find_at_with_meta("주지 스님".as_bytes(), 0)
            .is_some()
    );
    assert!(
        adnominal
            .find_at_with_meta("건 사람".as_bytes(), 0)
            .is_some()
    );
    assert!(
        particle_host
            .find_at_with_meta("학교에서 새 문서".as_bytes(), 0)
            .is_some()
    );
    assert!(
        adverb
            .find_at_with_meta("너무 빨라도".as_bytes(), 0)
            .is_some()
    );
    assert!(pronoun.find_at_with_meta("제 나라".as_bytes(), 0).is_some());
    assert!(numeral.find_at_with_meta("한 사람".as_bytes(), 0).is_some());
}

#[test]
fn runtime_nominal_component_remains_available_without_a_source_decomposition() {
    let matcher = compile_with_full_pos(
        "명사",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
    );

    assert!(
        matcher
            .find_at_with_meta("복합명사를".as_bytes(), 0)
            .is_some()
    );
}

#[test]
fn whole_nominal_source_component_survives_a_shorter_particle_split() {
    let matcher = compile_embedded_with_component(
        "주의",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
    );
    let crossing = compile_embedded_with_component(
        "본주",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
    );

    assert!(
        matcher
            .find_at_with_meta("자본주의".as_bytes(), 0)
            .is_some()
    );
    assert!(
        crossing
            .find_at_with_meta("자본주의".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn modifier_led_nominal_path_keeps_exact_tail_but_not_a_whole_adverb_component() {
    let resource = component_resource_from_entries([
        component_entry("어느", "MM"),
        component_entry("어느", "NP"),
        component_entry("날", "NNG"),
        component_entry("날", "JKO"),
        component_entry("매", "MM"),
        component_entry("매", "NNG"),
        component_entry("일", "NNG"),
        component_entry("일", "JKO"),
        component_entry("매일", "MAG"),
        component_entry("아무", "MM"),
        component_entry("아무", "NP"),
        component_entry("나", "NP"),
        component_entry("나", "JKO"),
        component_entry("칠", "MM"),
        component_entry("칠", "NR"),
        component_entry("월", "NNG"),
        component_entry("월", "NNBC"),
        component_entry("소", "MM"),
        component_entry("소", "NNG"),
        component_entry("년", "NNG"),
        component_entry("년", "NNB"),
        component_entry("은", "JX"),
    ]);
    let day = compile_embedded_with_resource(
        "날",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
        Arc::clone(&resource),
    );
    let every_day = compile_embedded_with_resource(
        "일",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
        Arc::clone(&resource),
    );
    let anyone = compile_embedded_with_resource(
        "나",
        CompileOptions {
            global_pos: Some(CoarsePos::Pronoun),
            ..CompileOptions::default()
        },
        Arc::clone(&resource),
    );
    let month = compile_embedded_with_resource(
        "월",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
        Arc::clone(&resource),
    );
    let year = compile_embedded_with_resource(
        "년",
        CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
        resource,
    );

    assert!(day.find_at_with_meta("어느날".as_bytes(), 0).is_some());
    assert!(every_day.find_at_with_meta("매일".as_bytes(), 0).is_none());
    assert!(anyone.find_at_with_meta("아무나".as_bytes(), 0).is_none());
    assert!(month.find_at_with_meta("칠월".as_bytes(), 0).is_some());
    assert!(year.find_at_with_meta("소년은".as_bytes(), 0).is_none());
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
fn explicit_pos_smart_connective_ji_recovers_only_a_right_edge_suffix() {
    let explicit = compile(
        "주다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );
    assert!(
        explicit
            .find_at_with_meta("나무를 심어주지".as_bytes(), 0)
            .is_some()
    );
    assert!(
        explicit
            .find_at_with_meta("나무를 심어주지는".as_bytes(), 0)
            .is_none()
    );

    let untagged = compile("주다", CompileOptions::default());
    assert!(
        untagged
            .find_at_with_meta("나무를 심어주지".as_bytes(), 0)
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
fn compiled_mieum_nominalizer_consumes_only_valid_particle_chains() {
    for boundary in [BoundaryPolicy::Smart, BoundaryPolicy::Token] {
        let matcher = compile(
            "걷다",
            CompileOptions {
                boundary,
                ..CompileOptions::default()
            },
        );
        for (text, token) in [
            ("매일 걸음이 이어진다.", "걸음이"),
            ("오랜 걸음을 기록했다.", "걸음을"),
            ("걸음에서도 특징이 드러난다.", "걸음에서도"),
            ("걸음으로 건강을 지킨다.", "걸음으로"),
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
                    .any(|rule| rule.as_str() == "ending.nominalizer")
            );
            assert!(
                atom.origins[0]
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str().starts_with("particle."))
            );
        }

        for text in [
            "걸음가 이어진다.",
            "걸음를 기록했다.",
            "걸음로 건강을 지킨다.",
            "걸음이를 기록했다.",
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
fn smart_vcp_corpus_fixtures_apply_component_evidence() {
    let matcher = compile("이다", CompileOptions::default());
    assert!(
        matcher.plan().atoms[0]
            .programs
            .iter()
            .all(|branch| branch.decision.is_structural())
    );

    for fixture in VCP_BOUNDARY_FIXTURES {
        assert_eq!(
            matcher
                .find_at_with_meta(fixture.text.as_bytes(), 0)
                .is_some(),
            fixture.gold_vcp,
            "component-aware result differed for {} ({})",
            fixture.case_name,
            fixture.text
        );
    }
}

#[test]
fn smart_predicate_component_preserves_nominalizer_particle_validation() {
    let matcher = compile("걷다", CompileOptions::default());

    for text in ["걷기이 어렵다.", "걷기을 권했다.", "걷기가를 권했다."] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "component evidence bypassed nominalizer particle validation for {text}"
        );
    }
    assert!(
        matcher
            .find_at_with_meta("걷기가 어렵다.".as_bytes(), 0)
            .is_some()
    );
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
fn canonical_vcp_corpus_fixtures_preserve_union_without_an_exact_resource_surface() {
    let options = CompileOptions {
        normalization: NormalizationMode::Canonical,
        ..CompileOptions::default()
    };
    let matcher = compile("이다", options);

    for fixture in VCP_BOUNDARY_FIXTURES {
        let decomposed = fixture.text.nfd().collect::<String>();
        assert_eq!(
            matcher
                .find_at_with_meta(decomposed.as_bytes(), 0)
                .is_some(),
            fixture.gold_vcp,
            "canonical union result differed for {} ({})",
            fixture.case_name,
            fixture.text
        );
    }
    assert!(
        matcher.plan().atoms[0]
            .programs
            .iter()
            .all(|branch| branch.decision.is_structural())
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
fn nominal_overrides_preserve_replacement_and_alias_contracts() {
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

    for (query, override_form, base_form) in [
        ("저", "제 생각", "저의 생각"),
        ("누구", "누가 왔다", "누구를 기다렸다"),
    ] {
        let matcher = compile(query, CompileOptions::default());
        assert!(
            matcher
                .find_at_with_meta(override_form.as_bytes(), 0)
                .is_some()
        );
        assert!(matcher.find_at_with_meta(base_form.as_bytes(), 0).is_some());
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
    compile_with_lexicons(query, options, lexicons)
}

fn compile_embedded_with_component(query: &str, options: CompileOptions) -> MorphMatcher {
    compile_embedded_with_resource(query, options, component_resource())
}

fn compile_embedded_with_resource(
    query: &str,
    options: CompileOptions,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
    let lexicons = Arc::new(Lexicons::embedded().expect("embedded lexicons must be valid"));
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan = Arc::new(compile_query(query, &options, &analyzer).expect("query must compile"));
    MorphMatcher::with_component_resource(plan, resource)
        .expect("component-aware matcher must build")
}

fn component_resource_from_entries(
    entries: impl IntoIterator<Item = MecabSourceMorphologyEntry>,
) -> Arc<ComponentResource> {
    let entries = entries.into_iter().collect::<Vec<_>>();
    let bytes = encode_component_resource([8; 32], &entries).expect("valid component entries");
    Arc::new(
        decode_component_resource("test", bytes, &[8; 32]).expect("component entries must decode"),
    )
}

fn compile_with_full_pos(query: &str, options: CompileOptions) -> MorphMatcher {
    let mut lexicons = Lexicons::embedded().expect("embedded lexicons must be valid");
    let full_data = LexiconData {
        predicates: vec![PredicateRecord {
            lemma: "지다".to_owned(),
            pos: DataFinePos::Vx,
            alternation: DataAlternation::Regular,
            flags: Default::default(),
            overrides: Vec::new(),
        }],
        nominals: vec![NominalRecord {
            lemma: "전체사전표식".to_owned(),
            pos: DataFinePos::Nng,
            flags: Default::default(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    lexicons
        .load_full_pos(&encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap())
        .expect("test full-POS lexicon must load");
    compile_with_lexicons(query, options, Arc::new(lexicons))
}

fn compile_with_lexicons(
    query: &str,
    options: CompileOptions,
    lexicons: Arc<Lexicons>,
) -> MorphMatcher {
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan = Arc::new(compile_query(query, &options, &analyzer).expect("query must compile"));
    if plan.requires_component_resource() {
        MorphMatcher::with_component_resource(plan, component_resource())
            .expect("component-aware matcher must build")
    } else {
        MorphMatcher::new(plan).expect("matcher must build")
    }
}

fn component_resource() -> Arc<ComponentResource> {
    static RESOURCE: OnceLock<Arc<ComponentResource>> = OnceLock::new();
    Arc::clone(RESOURCE.get_or_init(|| {
        let entries = [
            component_entry("매일", "MAG"),
            component_entry("매", "NNG"),
            component_entry("일", "VCP"),
            component_entry("걷", "VV"),
            component_entry("걸", "VV"),
            component_entry("었", "EP"),
            component_entry("어", "EF"),
            component_entry("어", "EC"),
            component_expression_entry("걸었어", "VV+EP+EF", "걸/VV/*+었/EP/*+어/EF/*"),
            component_expression_entry("왔", "VV+EP", "오/VV/*+았/EP/*"),
            component_entry("있", "VV"),
            component_entry("않", "VV"),
            component_entry("으니까", "EC"),
            component_entry("었다", "EP+EF"),
            component_entry("다는", "EF+ETM"),
            component_entry("다", "EF"),
            component_entry("는", "ETM"),
            component_entry("는", "JX"),
            component_expression_entry("위해", "VV+EC", "위하/VV/*+어/EC/*"),
            component_expression_entry("대해", "VV+EC", "대하/VV/*+어/EC/*"),
            component_entry("없", "VA"),
            component_entry("서는", "EC+JX"),
            component_entry("지는", "EC+JX"),
            component_entry("서를", "EC+JKO"),
            component_entry("지를", "EC+JKO"),
            component_entry("메꾸", "VV"),
            component_entry("졌", "VX+EP"),
            component_entry("떨", "VV"),
            component_entry("진", "VX+ETM"),
            component_entry("떨어진", "VV+ETM"),
            component_entry("길", "VA"),
            component_entry("달라", "VA+EC"),
            component_entry("사", "NNG"),
            component_entry("사진", "NNG"),
            component_entry("만", "VV"),
            component_entry("들려", "EC"),
            component_expression_entry("만들려", "VV+EC", "만들/VV/*+려고/EC/*"),
            component_entry("한", "VA+ETM"),
            component_expression_entry("한다", "VV+EF", "하/VV/*+ㄴ다/EF/*"),
            component_entry("새", "MM"),
            component_entry("새", "NNG"),
            component_entry("기능", "NNG"),
            component_entry("문서", "NNG"),
            component_entry("학교", "NNG"),
            component_entry("학교에서", "NNG"),
            component_entry("자본주", "NNG"),
            component_expression_entry("자본주의", "NNG", "자본/NNG/*+주의/NNG/*"),
            component_entry("에", "NNG"),
            component_entry("에서", "JKB"),
            component_entry("서", "JKB"),
            component_entry("서", "EC"),
            component_entry("복합", "NNG"),
            component_entry("명사", "NNG"),
            component_entry("복합명사", "NNG"),
            component_entry("를", "JKO"),
            component_entry("너무", "MAG"),
            component_entry("너무", "NNG"),
            component_entry("빨", "NNG"),
            component_entry("빨라도", "VA+EC"),
            component_entry("제", "MM"),
            component_entry("제", "NP"),
            component_entry("나라", "NNG"),
            component_entry("한", "MM"),
            component_entry("한", "NR"),
            component_entry("주", "VV"),
            component_entry("지", "EC"),
            component_entry("주지", "NNG"),
            component_expression_entry("주지", "VV+EC", "주/VV/*+지/EC/*"),
            component_entry("온", "MM"),
            component_expression_entry("온", "VV+ETM", "오/VV/*+ᆫ/ETM/*"),
            component_entry("어떤", "VA"),
            component_entry("어떤가", "MM+EC"),
            component_entry("가", "EC"),
            component_entry("지", "NNB"),
            component_entry("빼", "VV"),
            component_entry("놓", "VX"),
            component_entry("을", "ETM"),
            component_entry("볼", "VX+ETM"),
            component_entry("비춰볼", "VV+EC+VX+ETM"),
            component_entry("건", "NNB"),
            component_entry("건", "VV+ETM"),
            component_entry("스님", "NNG"),
            component_entry("사람", "NNG"),
            component_entry("기", "ETN"),
            component_entry("이", "JKS"),
            component_entry("을", "JKO"),
            component_entry("가", "JKS"),
            component_entry("를", "JKO"),
        ];
        let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
            .expect("test component resource must encode");
        Arc::new(
            decode_component_resource("test", bytes, &COMPONENT_RESOURCE_SOURCE_DIGEST)
                .expect("test component resource must decode"),
        )
    }))
}

fn component_entry(surface: &str, pos: &str) -> MecabSourceMorphologyEntry {
    component_expression_entry(surface, pos, "*")
}

fn component_expression_entry(
    surface: &str,
    pos: &str,
    expression: &str,
) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost: -5_000,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: expression.to_owned(),
    }
}
