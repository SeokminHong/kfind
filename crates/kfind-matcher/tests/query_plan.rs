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
fn compiled_predicate_plan_matches_a_prospective_final() {
    let matcher = compile(
        "않다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
    );

    assert!(
        matcher
            .find_at_with_meta("투명하지는 않으리라 생각된다.".as_bytes(), 0)
            .is_some(),
        "compiled 않다 plan rejected the prospective final"
    );
}

#[test]
fn nikl_attested_endings_require_complete_source_paths() {
    let resource = component_resource_from_entries([
        component_entry("섬나라", "NNG"),
        component_entry("이", "VCP"),
        component_entry("므로", "EC"),
        component_expression_entry("아닐세", "VCN+EF", "아니/VCN/*+ᆯ세/EF/*"),
    ]);
    let copula =
        compile_embedded_with_resource("이다", CompileOptions::default(), Arc::clone(&resource));
    let negative_copula =
        compile_embedded_with_resource("아니다", CompileOptions::default(), resource);

    assert!(
        copula
            .find_at_with_meta("섬나라이므로".as_bytes(), 0)
            .is_some()
    );
    assert!(
        copula
            .find_at_with_meta("섬나라므로".as_bytes(), 0)
            .is_none()
    );
    assert!(
        negative_copula
            .find_at_with_meta("아닐세".as_bytes(), 0)
            .is_some()
    );
    assert!(
        negative_copula
            .find_at_with_meta("아닐새".as_bytes(), 0)
            .is_none()
    );
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
fn predicate_endings_accept_only_source_backed_auxiliary_particle_chains() {
    let resource = component_resource_from_entries([
        component_entry("먹", "VV"),
        component_entry("고", "EC"),
        component_entry("는", "JX"),
        component_entry("도", "JX"),
        component_entry("만", "JX"),
        component_entry("조차", "JX"),
        component_entry("커녕", "JX"),
        component_entry("뿐", "JX"),
        component_entry("를", "JKO"),
    ]);
    let matcher = compile_with_full_pos_and_resource(
        "먹다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
        resource,
    );

    for text in ["먹고는", "먹고도", "먹고만", "먹고조차", "먹고는커녕"] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "source-backed auxiliary particle chain was rejected in {text}"
        );
    }
    for text in ["먹고를", "먹고뿐", "먹고도는"] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "unlicensed predicate-ending particle chain was accepted in {text}"
        );
    }

    let without_particle_source = compile_with_full_pos_and_resource(
        "먹다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
        component_resource_from_entries([component_entry("먹", "VV"), component_entry("고", "EC")]),
    );
    assert!(
        without_particle_source
            .find_at_with_meta("먹고도".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn adverbial_ge_uses_an_ending_then_auxiliary_particle_source_path() {
    let matcher = compile_with_full_pos(
        "이렇다",
        CompileOptions {
            global_pos: Some(CoarsePos::Adjective),
            ..CompileOptions::default()
        },
    );

    for text in ["또 이렇게도 비판하고 있다.", "또 이렇게는 비판하지 않는다."]
    {
        let matched = matcher
            .find_at_with_meta(text.as_bytes(), 0)
            .expect("adverbial -게 plus auxiliary particle source path was rejected");
        assert_eq!(&text[matched.atoms[0].core.clone()], "이렇");
        assert_eq!(&text[matched.atoms[0].token.clone()], "이렇게");
    }

    assert!(
        matcher
            .find_at_with_meta("문서에 이렇게를 적었다.".as_bytes(), 0)
            .is_none()
    );
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

    let exact_resource = component_resource_from_entries([
        component_expression_entry("올", "VV+ETM", "오/VV/*+ᆯ/ETM/*"),
        component_entry("지", "NNB"),
        component_entry("올지", "VV+EC"),
    ]);
    let exact_ending = compile_with_full_pos_and_resource(
        "오다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
        exact_resource,
    );
    assert!(
        exact_ending
            .find_at_with_meta("올지".as_bytes(), 0)
            .is_some(),
        "exact whole predicate ending path was rejected"
    );
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
fn smart_auxiliary_query_accepts_an_unaligned_whole_source_path() {
    let resource = component_resource_from_entries([
        component_expression_entry("빨라져", "VA+EC+VX+EC", "빠르/VA/*+아/EC/*+지/VX/*+어/EC/*"),
        component_expression_entry("알려진", "VV+EC+VX+ETM", "알리/VV/*+어/EC/*+지/VX/*+ᆫ/ETM/*"),
        component_expression_entry(
            "뚜렷해졌다",
            "XR+XSA+EC+VX+EP+EF",
            "뚜렷/XR/*+하/XSA/*+어/EC/*+지/VX/*+었/EP/*+다/EF/*",
        ),
        component_entry("사진", "NNG"),
    ]);
    let matcher = compile_with_full_pos_and_resource(
        "지다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
        resource,
    );

    for text in ["빨라져", "알려진", "뚜렷해졌다"] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "unaligned attached auxiliary source path was rejected for {text}"
        );
    }
    assert!(matcher.find_at_with_meta("사진".as_bytes(), 0).is_none());
}

#[test]
fn smart_auxiliary_query_accepts_a_split_derivational_source_path() {
    let resource = component_resource_from_entries([
        component_entry("뚜렷", "XR"),
        component_expression_entry("해졌", "XSA+EC+VX+EP", "하/XSA/*+어/EC/*+지/VX/*+었/EP/*"),
        component_entry("다", "EF"),
        component_entry("사진", "NNG"),
    ]);
    let matcher = compile_with_full_pos_and_resource(
        "지다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
        resource,
    );

    assert!(
        matcher
            .find_at_with_meta("뚜렷해졌다".as_bytes(), 0)
            .is_some()
    );
    assert!(matcher.find_at_with_meta("사진".as_bytes(), 0).is_none());
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
fn determiner_accepts_a_complete_derived_nominal_phrase_in_the_next_token() {
    let resource = component_resource_from_entries([
        component_entry("전", "MM"),
        component_entry("전", "NNG"),
        component_entry("전", "NNB"),
        component_entry("가구", "NNG"),
        component_entry("별", "XSN"),
        component_entry("로", "JKB"),
        component_entry("는", "ETM"),
        component_entry("한다고", "VV+EF+EC"),
    ]);
    let matcher = compile_with_full_pos_and_resource(
        "전",
        CompileOptions {
            global_pos: Some(CoarsePos::Determiner),
            ..CompileOptions::default()
        },
        resource,
    );

    assert!(
        matcher
            .find_at_with_meta("경우에는 전 가구별로".as_bytes(), 0)
            .is_some()
    );
    assert!(
        matcher
            .find_at_with_meta("경우에는 전 한다고".as_bytes(), 0)
            .is_none()
    );
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
fn standard_spacing_disambiguates_mot_homographs() {
    let resource = component_resource_from_entries([
        component_entry("못", "MAG"),
        component_entry("못", "NNG"),
        component_entry("못했", "VA"),
        component_entry("못하", "VA"),
        component_entry("하다", "NNG"),
        component_entry("다", "EF"),
        component_entry("하", "VV"),
        component_entry("했", "VV+EP"),
        component_entry("겠", "EP"),
        component_entry("어요", "EF"),
        component_entry("박", "VV"),
        component_entry("았", "EP"),
    ]);
    let matches = |query: &str, text: &str| {
        compile_embedded_with_resource(query, CompileOptions::default(), Arc::clone(&resource))
            .find_at_with_meta(text.as_bytes(), 0)
            .is_some()
    };

    assert!(!matches("adv:못", "일을 못했다"));
    assert!(!matches("n:못", "일을 못했다"));
    assert!(!matches("n:못", "형보다 못하다"));
    assert!(matches("adv:못", "못 하겠어요"));
    assert!(!matches("n:못", "못 하겠어요"));
    assert!(matches("adv:못", "일을 못 했다"));
    assert!(!matches("n:못", "일을 못 했다"));
    assert!(matches("n:못", "못 박았어요"));
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
        component_entry("어", "VV"),
        component_entry("느", "NNG"),
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
fn numeric_unit_path_keeps_only_a_dependent_noun_tail() {
    let resource = component_resource_from_entries([
        component_entry("년", "NNBC"),
        component_entry("간", "NNB"),
        component_entry("시", "NNBC"),
        component_entry("시간", "NNBC"),
        component_entry("명", "NNBC"),
        component_entry("사", "NNG"),
        component_entry("명사", "NNG"),
        component_entry("의", "JKG"),
    ]);
    let options = CompileOptions {
        global_pos: Some(CoarsePos::Noun),
        ..CompileOptions::default()
    };
    let year = compile_embedded_with_resource("년", options.clone(), Arc::clone(&resource));
    let period = compile_embedded_with_resource("간", options.clone(), Arc::clone(&resource));
    let time = compile_embedded_with_resource("시간", options.clone(), Arc::clone(&resource));
    let ordinary_tail = compile_embedded_with_resource("사", options, resource);

    assert!(year.find_at_with_meta("1년간".as_bytes(), 0).is_some());
    assert!(period.find_at_with_meta("1년간".as_bytes(), 0).is_some());
    assert!(period.find_at_with_meta("1년간의".as_bytes(), 0).is_some());
    assert!(time.find_at_with_meta("10시간".as_bytes(), 0).is_some());
    assert!(
        ordinary_tail
            .find_at_with_meta("197명사".as_bytes(), 0)
            .is_none()
    );
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
fn inflection_adverb_plan_matches_only_auxiliary_particles() {
    let matcher = compile("빨리", CompileOptions::default());

    for text in ["일을 빨리도 끝냈다.", "빨리는 끝냈다."] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "default inflection rejected the adverb-particle structure in {text}"
        );
    }
    assert!(
        matcher
            .find_at_with_meta("빨리가 답이다.".as_bytes(), 0)
            .is_none()
    );

    let literal = compile(
        "빨리",
        CompileOptions {
            expand: ExpandMode::Literal,
            ..CompileOptions::default()
        },
    );
    assert!(
        literal
            .find_at_with_meta("일을 빨리도 끝냈다.".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn adverb_particle_hosts_and_transitions_cover_complete_families() {
    let adverb_options = CompileOptions {
        global_pos: Some(CoarsePos::Adverb),
        ..CompileOptions::default()
    };
    let maybe = compile("혹시", adverb_options.clone());
    let far = compile("멀리", adverb_options.clone());
    let actually = compile("실제로", adverb_options);

    assert!(maybe.find_at_with_meta("혹시나".as_bytes(), 0).is_some());
    for text in ["멀리까지도", "멀리까지만", "멀리까지는"] {
        assert!(
            far.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "adverb particle graph rejected {text}"
        );
    }
    assert!(
        actually
            .find_at_with_meta("실제로는커녕".as_bytes(), 0)
            .is_some()
    );
    assert!(maybe.find_at_with_meta("혹시가".as_bytes(), 0).is_none());
    assert!(
        actually
            .find_at_with_meta("실제로커녕".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn adverb_particle_chain_survives_a_competing_nominal_particle_path() {
    let resource = component_resource_from_entries([
        component_entry("실제", "NNG"),
        component_entry("로", "JKB"),
        component_entry("는", "JX"),
        component_entry("가", "JKS"),
    ]);
    let matcher = compile_embedded_with_resource(
        "실제로",
        CompileOptions {
            global_pos: Some(CoarsePos::Adverb),
            ..CompileOptions::default()
        },
        resource,
    );

    assert!(
        matcher
            .find_at_with_meta("실제로는".as_bytes(), 0)
            .is_some()
    );
    assert!(
        matcher
            .find_at_with_meta("실제로가".as_bytes(), 0)
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

    let see = compile(
        "보다",
        CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            boundary: BoundaryPolicy::Any,
            ..CompileOptions::default()
        },
    );
    assert!(
        see.find_at_with_meta("방을 보로 가다".as_bytes(), 0)
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
fn compiled_nominal_plan_covers_particle_transition_families() {
    let matcher = compile("사용자", CompileOptions::default());

    for text in [
        "사용자까지도 왔다.",
        "사용자까지만 왔다.",
        "사용자까지는 왔다.",
        "사용자까지만은 허용한다.",
        "사용자로부터의 요청이다.",
        "사용자에게로 보냈다.",
        "사용자에서부터 시작했다.",
        "사용자에의 의존이다.",
        "사용자조차도 동의했다.",
        "사용자마저도 동의했다.",
        "사용자들로부터의 요청이다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "rejected particle transition family: {text}"
        );
    }
    for text in [
        "사용자는에게",
        "사용자도까지",
        "사용자까지도만",
        "사용자들로부터까지만",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "accepted forbidden or overlong particle chain: {text}"
        );
    }
}

#[test]
fn compiled_nominal_plan_composes_particle_chains_with_copula_grammar() {
    let matcher = compile("사용자", CompileOptions::default());

    for text in [
        "대상은 사용자뿐이다.",
        "대상은 사용자뿐만이다.",
        "범위는 사용자까지다.",
        "범위는 사용자로부터였다.",
        "대상은 사용자뿐이었다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "rejected nominal-particle-copula structure: {text}"
        );
    }
    for text in [
        "대상은 사용자뿐다.",
        "대상은 사용자뿐였다.",
        "대상은 사용자뿐이.",
        "대상은 사용자뿐도만이다.",
        "범위는 사용자까지였.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "accepted invalid nominal-particle-copula structure: {text}"
        );
    }
}

#[test]
fn compiled_nominal_plan_covers_dictionary_consensus_particle_families() {
    let matcher = compile("사용자", CompileOptions::default());

    for text in [
        "사용자께서 오셨다.",
        "사용자같이 처리한다.",
        "사용자대로 둔다.",
        "사용자더러 말했다.",
        "사용자마다 다르다.",
        "사용자만큼 빠르다.",
        "사용자밖에 없다.",
        "사용자보고 말했다.",
        "사용자보다 빠르다.",
        "사용자뿐 남았다.",
        "사용자처럼 행동한다.",
        "사용자커녕 아무도 없다.",
        "사용자께서는 오셨다.",
        "사용자뿐만 남았다.",
        "사용자는커녕 아무도 없다.",
        "사용자들마다 다르다.",
        "사용자보다도 빠르다.",
        "사용자나 관리자가 처리한다.",
        "사용자나마 남았다.",
        "사용자라도 처리한다.",
        "사용자랑 관리자가 처리한다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "rejected dictionary particle family: {text}"
        );
    }
    for text in [
        "사용자이나",
        "사용자이나마",
        "사용자이라도",
        "사용자이랑",
        "사용자은커녕",
        "사용자ㄴ커녕",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "accepted invalid dictionary particle allomorph: {text}"
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
fn compiled_vcp_plan_uses_complete_nominal_and_particle_hosts() {
    let resource = component_resource_from_entries([
        component_entry("상표", "NNG"),
        component_entry("구경거리", "NNG"),
        component_entry("학교", "NNG"),
        component_entry("대학", "NNG"),
        component_entry("매", "NNG"),
        component_entry("매일", "MAG"),
    ]);
    let matcher = compile_with_full_pos_and_resource("이다", CompileOptions::default(), resource);

    for text in [
        "버버리는 회사 상표다.",
        "끔찍한 구경거리였다.",
        "범위는 학교까지였다.",
        "대상은 대학뿐이다.",
        "대상은 대학뿐이었다.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "rejected complete copula frame: {text}"
        );
    }
    for text in [
        "대학다.",
        "대학였다.",
        "대학여서 갔다.",
        "매일 만났다.",
        "다.",
        "학교다른.",
    ] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "accepted invalid copula frame: {text}"
        );
    }
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
fn nominal_topic_contraction_covers_the_pronoun_family() {
    for (query, contracted, full) in [
        ("이거", "이건", "이거는"),
        ("그거", "그건", "그거는"),
        ("저거", "저건", "저거는"),
    ] {
        let matcher = compile(
            query,
            CompileOptions {
                global_pos: Some(CoarsePos::Pronoun),
                ..CompileOptions::default()
            },
        );
        assert!(
            matcher
                .find_at_with_meta(contracted.as_bytes(), 0)
                .is_some(),
            "{query} must accept contracted topic form {contracted}"
        );
        assert!(
            matcher.find_at_with_meta(full.as_bytes(), 0).is_some(),
            "{query} must preserve full topic form {full}"
        );
        let compound = format!("{contracted}물");
        assert!(
            matcher.find_at_with_meta(compound.as_bytes(), 0).is_none(),
            "{query} must not leak the contraction into {compound}"
        );
    }

    let other_pronoun = compile(
        "누구",
        CompileOptions {
            global_pos: Some(CoarsePos::Pronoun),
            ..CompileOptions::default()
        },
    );
    assert!(
        other_pronoun.plan().atoms[0]
            .programs
            .iter()
            .all(|branch| branch.anchor.as_ref() != "누건".as_bytes())
    );
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
        ("나", ["집이나", "바다나"], ["집나", "바다이나"]),
        ("나마", ["집이나마", "바다나마"], ["집나마", "바다이나마"]),
        ("라도", ["집이라도", "바다라도"], ["집라도", "바다이라도"]),
        ("랑", ["집이랑", "바다랑"], ["집랑", "바다이랑"]),
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
    compile_with_full_pos_and_resource(query, options, component_resource())
}

fn compile_with_full_pos_and_resource(
    query: &str,
    options: CompileOptions,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
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
    compile_with_lexicons_and_resource(query, options, Arc::new(lexicons), resource)
}

fn compile_with_lexicons(
    query: &str,
    options: CompileOptions,
    lexicons: Arc<Lexicons>,
) -> MorphMatcher {
    compile_with_lexicons_and_resource(query, options, lexicons, component_resource())
}

fn compile_with_lexicons_and_resource(
    query: &str,
    options: CompileOptions,
    lexicons: Arc<Lexicons>,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let plan = Arc::new(compile_query(query, &options, &analyzer).expect("query must compile"));
    if plan.requires_component_resource() {
        MorphMatcher::with_component_resource(plan, resource)
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
            component_entry("이렇", "VA"),
            component_entry("게", "EC"),
            component_entry("도", "JX"),
            component_expression_entry("이렇게", "MAG", "이렇게/MAG/*"),
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
