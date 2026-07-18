use std::collections::BTreeSet;

use super::*;
use crate::{PredicateFlags, PredicatePos};

fn entry(lemma: &str, pos: PredicatePos, alternation: LexicalAlternation) -> PredicateEntry {
    PredicateEntry {
        lemma: lemma.into(),
        pos,
        alternation,
        flags: PredicateFlags::NONE,
        overrides: Box::new([]),
        derivations: Box::new([]),
    }
}

fn surfaces(entry: &PredicateEntry) -> BTreeSet<String> {
    generate_predicate_branches(entry)
        .expect("valid fixture")
        .iter()
        .flat_map(test_surfaces)
        .collect()
}

fn test_surfaces(branch: &SurfaceBranchSpec) -> Vec<String> {
    match branch.continuation {
        ContinuationState::Terminal | ContinuationState::AOrEo => {
            vec![branch.anchor.to_string()]
        }
        ContinuationState::Past => {
            vec![branch.anchor.to_string(), format!("{}다", branch.anchor)]
        }
        ContinuationState::Future => {
            vec![branch.anchor.to_string(), format!("{}다", branch.anchor)]
        }
        ContinuationState::Declarative => {
            ["", "고", "는", "던", "면", "니", "며", "면서", "는데", "지"]
                .into_iter()
                .map(|suffix| format!("{}{suffix}", branch.anchor))
                .collect()
        }
        ContinuationState::Eu => vec![
            format!("{}면", branch.anchor),
            format!("{}며", branch.anchor),
            format!("{}니", branch.anchor),
            format!("{}니까", branch.anchor),
            format!("{}니까는", branch.anchor),
            format!("{}니깐", branch.anchor),
            format!("{}리라", branch.anchor),
            format!("{}리라고", branch.anchor),
            format!("{}셨다", branch.anchor),
            format!("{}시다", branch.anchor),
            format!("{}십니다", branch.anchor),
        ],
    }
}

#[test]
fn dictionary_derivation_clones_share_generated_forms() {
    let derivation = PredicateDerivation::new(
        "밀리다",
        PredicatePos::Verb,
        RuleId::from("lexical.dictionary-voice"),
    );
    let cloned = derivation.clone();

    let branches = derivation.generated_branches().expect("valid derivation");
    let cloned_branches = cloned.generated_branches().expect("valid clone");
    assert!(std::ptr::eq(branches, cloned_branches));
    assert!(
        branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "밀려")
    );

    let fallback_stems = derivation
        .generated_fallback_stems()
        .expect("valid fallback stems");
    let cloned_fallback_stems = cloned
        .generated_fallback_stems()
        .expect("valid cloned fallback stems");
    assert!(std::ptr::eq(fallback_stems, cloned_fallback_stems));
}

fn assert_has_all(actual: &BTreeSet<String>, expected: &[&str]) {
    for expected in expected {
        assert!(
            actual.contains(*expected),
            "missing {expected} from {actual:?}"
        );
    }
}

#[test]
fn regular_stems_cover_consonant_and_vowel_endings() {
    let eat = surfaces(&entry(
        "먹다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&eat, &["먹어", "먹었다", "먹는", "먹은", "먹을"]);

    let go = surfaces(&entry(
        "가다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(
        &go,
        &[
            "가",
            "갔다",
            "가는",
            "간",
            "갈",
            "가시다",
            "가셨다",
            "가십니다",
        ],
    );

    let live = surfaces(&entry(
        "살다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&live, &["살며", "사시다", "사셨다", "사십니다", "사시면"]);

    let rare = surfaces(&entry(
        "드물다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&rare, &["드물며"]);
}

#[test]
fn mieum_nominalizer_uses_stem_environment_and_lexical_alternation() {
    for (predicate, expected) in [
        (
            entry("먹다", PredicatePos::Verb, LexicalAlternation::Regular),
            "먹음",
        ),
        (
            entry("가다", PredicatePos::Verb, LexicalAlternation::Regular),
            "감",
        ),
        (
            entry("알다", PredicatePos::Verb, LexicalAlternation::Regular),
            "앎",
        ),
        (
            entry("걷다", PredicatePos::Verb, LexicalAlternation::DToL),
            "걸음",
        ),
        (
            entry("짓다", PredicatePos::Verb, LexicalAlternation::DropS),
            "지음",
        ),
        (
            entry("돕다", PredicatePos::Verb, LexicalAlternation::BToWa),
            "도움",
        ),
        (
            entry(
                "아름답다",
                PredicatePos::Adjective,
                LexicalAlternation::BToWo,
            ),
            "아름다움",
        ),
        (
            entry("파랗다", PredicatePos::Adjective, LexicalAlternation::DropH),
            "파람",
        ),
        (
            entry(
                "빠르다",
                PredicatePos::Adjective,
                LexicalAlternation::ReuDoubleL,
            ),
            "빠름",
        ),
        (
            entry("푸르다", PredicatePos::Adjective, LexicalAlternation::Reo),
            "푸름",
        ),
        (
            entry("하다", PredicatePos::Verb, LexicalAlternation::Ha),
            "함",
        ),
        (
            entry("푸다", PredicatePos::Verb, LexicalAlternation::UToEo),
            "품",
        ),
        (
            entry("이다", PredicatePos::Copula, LexicalAlternation::Copula),
            "임",
        ),
    ] {
        assert!(
            surfaces(&predicate).contains(expected),
            "missing {expected}"
        );
    }
}

#[test]
fn action_predicates_cover_intentive_connectives() {
    for (predicate, expected) in [
        (
            entry("꾀하다", PredicatePos::Verb, LexicalAlternation::Ha),
            "꾀하려고",
        ),
        (
            entry("먹다", PredicatePos::Verb, LexicalAlternation::Regular),
            "먹으려고",
        ),
        (
            entry("듣다", PredicatePos::Verb, LexicalAlternation::DToL),
            "들으려고",
        ),
        (
            entry("돕다", PredicatePos::Verb, LexicalAlternation::BToWa),
            "도우려고",
        ),
    ] {
        assert!(surfaces(&predicate).contains(expected));
    }

    let adjective = entry("좋다", PredicatePos::Adjective, LexicalAlternation::Regular);
    assert!(!surfaces(&adjective).contains("좋으려고"));
}

#[test]
fn action_predicates_cover_geora_and_o_final_neora_imperatives() {
    for (predicate, expected) in [
        (
            entry("가다", PredicatePos::Verb, LexicalAlternation::Regular),
            "가거라",
        ),
        (
            entry("먹다", PredicatePos::Verb, LexicalAlternation::Regular),
            "먹거라",
        ),
        (
            entry("걷다", PredicatePos::Verb, LexicalAlternation::DToL),
            "걷거라",
        ),
        (
            entry("오다", PredicatePos::Verb, LexicalAlternation::Regular),
            "오너라",
        ),
        (
            entry("들어오다", PredicatePos::Verb, LexicalAlternation::Regular),
            "들어오너라",
        ),
    ] {
        assert!(surfaces(&predicate).contains(expected));
    }

    let go = surfaces(&entry(
        "가다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert!(!go.contains("가너라"));

    let adjective = surfaces(&entry(
        "좋다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    ));
    assert!(!adjective.contains("좋거라"));
}

#[test]
fn productive_predicates_cover_reason_connectives() {
    for (predicate, expected) in [
        (
            entry(
                "바쁘다",
                PredicatePos::Adjective,
                LexicalAlternation::Regular,
            ),
            ["바쁘니", "바쁘니까", "바쁘니까는", "바쁘니깐"],
        ),
        (
            entry("먹다", PredicatePos::Verb, LexicalAlternation::Regular),
            ["먹으니", "먹으니까", "먹으니까는", "먹으니깐"],
        ),
        (
            entry("살다", PredicatePos::Verb, LexicalAlternation::Regular),
            ["사니", "사니까", "사니까는", "사니깐"],
        ),
        (
            entry("듣다", PredicatePos::Verb, LexicalAlternation::DToL),
            ["들으니", "들으니까", "들으니까는", "들으니깐"],
        ),
        (
            entry("돕다", PredicatePos::Verb, LexicalAlternation::BToWa),
            ["도우니", "도우니까", "도우니까는", "도우니깐"],
        ),
    ] {
        assert_has_all(&surfaces(&predicate), &expected);
    }
}

#[test]
fn productive_predicates_cover_concessive_connectives() {
    for predicate in [
        entry("먹다", PredicatePos::Verb, LexicalAlternation::Regular),
        entry("듣다", PredicatePos::Verb, LexicalAlternation::DToL),
        entry("돕다", PredicatePos::Verb, LexicalAlternation::BToWa),
        entry("파랗다", PredicatePos::Adjective, LexicalAlternation::DropH),
    ] {
        let stem = predicate.lemma.strip_suffix('다').expect("predicate lemma");
        assert!(surfaces(&predicate).contains(&format!("{stem}더라도")));
    }
}

#[test]
fn productive_predicates_cover_prospective_endings() {
    for (predicate, expected) in [
        (
            entry("얻다", PredicatePos::Verb, LexicalAlternation::Regular),
            ["얻으리라", "얻으리라고"],
        ),
        (
            entry("가다", PredicatePos::Verb, LexicalAlternation::Regular),
            ["가리라", "가리라고"],
        ),
        (
            entry("듣다", PredicatePos::Verb, LexicalAlternation::DToL),
            ["들으리라", "들으리라고"],
        ),
        (
            entry("돕다", PredicatePos::Verb, LexicalAlternation::BToWa),
            ["도우리라", "도우리라고"],
        ),
        (
            entry("살다", PredicatePos::Verb, LexicalAlternation::Regular),
            ["살리라", "살리라고"],
        ),
    ] {
        assert_has_all(&surfaces(&predicate), &expected);
    }
}

#[test]
fn contractions_keep_the_required_uncontracted_forms() {
    let see = surfaces(&entry(
        "보다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&see, &["보아", "봐", "보았다", "봤다"]);

    let changed = surfaces(&entry(
        "되다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&changed, &["되어", "돼", "되었다", "됐다"]);

    let validate = surfaces(&entry(
        "검증하다",
        PredicatePos::Verb,
        LexicalAlternation::Ha,
    ));
    assert_has_all(&validate, &["검증하여", "검증해", "검증하였다", "검증했다"]);

    let turn_on = surfaces(&entry(
        "켜다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&turn_on, &["켜어", "켜", "켜었다", "켰다"]);
}

#[test]
fn d_irregular_is_computed_from_each_input_stem() {
    let walk = surfaces(&entry("걷다", PredicatePos::Verb, LexicalAlternation::DToL));
    assert_has_all(&walk, &["걸어", "걸었", "걸으면", "걸으셨다"]);

    let listen = surfaces(&entry("듣다", PredicatePos::Verb, LexicalAlternation::DToL));
    assert_has_all(&listen, &["들어", "들었", "들으면"]);
    assert!(!listen.contains("걸어"));

    let load = surfaces(&entry("싣다", PredicatePos::Verb, LexicalAlternation::DToL));
    assert!(load.contains("실어"));
}

#[test]
fn productive_endings_cover_retrospective_intentive_and_propositive_forms() {
    let walk = surfaces(&entry("걷다", PredicatePos::Verb, LexicalAlternation::DToL));
    assert_has_all(
        &walk,
        &[
            "걷던",
            "걷더니",
            "걷자",
            "걷자고",
            "걷곤",
            "걷느냐",
            "걷도록",
            "걸으려는",
            "걸읍시다",
        ],
    );

    for (lemma, alternation, expected) in [
        ("가다", LexicalAlternation::Regular, "갑시다"),
        ("먹다", LexicalAlternation::Regular, "먹읍시다"),
        ("살다", LexicalAlternation::Regular, "삽시다"),
    ] {
        assert!(surfaces(&entry(lemma, PredicatePos::Verb, alternation)).contains(expected));
    }
}

#[test]
fn multiple_lexicon_analyses_form_a_union() {
    let mut ask = surfaces(&entry("묻다", PredicatePos::Verb, LexicalAlternation::DToL));
    ask.extend(surfaces(&entry(
        "묻다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    )));
    assert_has_all(&ask, &["물어", "묻어", "물었다", "묻었다", "묻고"]);
}

#[test]
fn s_and_b_irregulars_are_lexical() {
    let build = surfaces(&entry(
        "짓다",
        PredicatePos::Verb,
        LexicalAlternation::DropS,
    ));
    assert!(build.contains("지어"));
    let improve = surfaces(&entry(
        "낫다",
        PredicatePos::Verb,
        LexicalAlternation::DropS,
    ));
    assert!(improve.contains("나아"));
    let connect = surfaces(&entry(
        "잇다",
        PredicatePos::Verb,
        LexicalAlternation::DropS,
    ));
    assert!(connect.contains("이어"));

    let help = surfaces(&entry(
        "돕다",
        PredicatePos::Verb,
        LexicalAlternation::BToWa,
    ));
    assert_has_all(&help, &["도와", "도우며"]);
    let lie = surfaces(&entry(
        "눕다",
        PredicatePos::Verb,
        LexicalAlternation::BToWo,
    ));
    assert!(lie.contains("누워"));
    let beautiful = surfaces(&entry(
        "아름답다",
        PredicatePos::Adjective,
        LexicalAlternation::BToWo,
    ));
    assert!(beautiful.contains("아름다워"));
}

#[test]
fn h_reu_reo_eu_and_u_rules_stay_distinct() {
    let blue = surfaces(&entry(
        "파랗다",
        PredicatePos::Adjective,
        LexicalAlternation::DropH,
    ));
    assert_has_all(&blue, &["파래", "파란"]);
    let so = surfaces(&entry(
        "그렇다",
        PredicatePos::Adjective,
        LexicalAlternation::DropH,
    ));
    assert_has_all(&so, &["그래", "그런"]);

    let fast = surfaces(&entry(
        "빠르다",
        PredicatePos::Adjective,
        LexicalAlternation::ReuDoubleL,
    ));
    assert!(fast.contains("빨라"));
    let call = surfaces(&entry(
        "부르다",
        PredicatePos::Verb,
        LexicalAlternation::ReuDoubleL,
    ));
    assert!(call.contains("불러"));
    let green = surfaces(&entry(
        "푸르다",
        PredicatePos::Adjective,
        LexicalAlternation::Reo,
    ));
    assert!(green.contains("푸르러"));

    let pretty = surfaces(&entry(
        "예쁘다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&pretty, &["예뻐", "예뻤다", "예쁜", "예쁠"]);
    assert!(!pretty.contains("예쁘어"));
    let scoop = surfaces(&entry(
        "푸다",
        PredicatePos::Verb,
        LexicalAlternation::UToEo,
    ));
    assert!(scoop.contains("퍼"));
}

#[test]
fn ha_rieul_and_copula_rules_cover_required_forms() {
    let validate = surfaces(&entry(
        "검증하다",
        PredicatePos::Verb,
        LexicalAlternation::Ha,
    ));
    assert_has_all(&validate, &["검증해", "검증했다"]);

    let live = surfaces(&entry(
        "살다",
        PredicatePos::Verb,
        LexicalAlternation::Regular,
    ));
    assert_has_all(&live, &["사는", "삽니다", "살고"]);

    let copula = surfaces(&entry(
        "이다",
        PredicatePos::Copula,
        LexicalAlternation::Copula,
    ));
    assert_has_all(
        &copula,
        &[
            "이고",
            "이어",
            "여서",
            "인",
            "일",
            "입니다",
            "이라고",
            "이라는",
            "이지",
            "이며",
            "이므로",
        ],
    );

    let negative = surfaces(&entry(
        "아니다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    ));
    assert!(negative.contains("아닐세"));
}

#[test]
fn gi_nominalization_uses_the_lexical_stem() {
    for (predicate, expected) in [
        (
            entry("걷다", PredicatePos::Verb, LexicalAlternation::DToL),
            "걷기",
        ),
        (
            entry("검증하다", PredicatePos::Verb, LexicalAlternation::Ha),
            "검증하기",
        ),
        (
            entry("이다", PredicatePos::Copula, LexicalAlternation::Copula),
            "이기",
        ),
    ] {
        assert!(surfaces(&predicate).contains(expected));
    }
}

#[test]
fn action_present_declarative_and_copula_past_are_compiled() {
    let action = entry("검증하다", PredicatePos::Verb, LexicalAlternation::Ha);
    let action_surfaces = surfaces(&action);
    assert!(action_surfaces.contains("검증한다"));
    assert!(action_surfaces.contains("검증한다고"));

    let declarative = generate_predicate_branches(&action)
        .expect("valid fixture")
        .into_iter()
        .find(|branch| branch.anchor.as_ref() == "검증한다")
        .expect("present declarative branch");
    assert_eq!(declarative.continuation, ContinuationState::Declarative);

    let copula = entry("이다", PredicatePos::Copula, LexicalAlternation::Copula);
    let copula_surfaces = surfaces(&copula);
    assert!(copula_surfaces.contains("이었"));
}

#[test]
fn complete_copula_surface_requires_an_exact_generated_inflection() {
    for surface in ["이다", "입니다", "이었다", "인", "다", "였다", "여서"] {
        assert!(verify_complete_copula_surface(surface), "{surface}");
    }
    for surface in ["이", "입", "이어", "이었", "였", "이기는", "이다른"] {
        assert!(!verify_complete_copula_surface(surface), "{surface}");
    }
}

#[test]
fn nominal_copula_contraction_depends_on_the_preceding_syllable() {
    for surface in ["다", "였다", "였고", "여서"] {
        assert!(
            verify_copula_surface_after_nominal('표', surface),
            "{surface}"
        );
        assert!(
            !verify_copula_surface_after_nominal('학', surface),
            "{surface}"
        );
    }
    for preceding in ['표', '학'] {
        for surface in ["이다", "이었다", "인", "일"] {
            assert!(
                verify_copula_surface_after_nominal(preceding, surface),
                "{preceding}{surface}"
            );
        }
    }
    assert!(!verify_copula_surface_after_nominal('A', "다"));
    assert!(!verify_copula_surface_after_nominal('표', "였"));
}

#[test]
fn descriptive_final_da_uses_declarative_continuations() {
    let descriptive = entry(
        "나쁘다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    );
    let final_da = generate_predicate_branches(&descriptive)
        .expect("valid fixture")
        .into_iter()
        .find(|branch| branch.anchor.as_ref() == "나쁘다")
        .expect("final declarative branch");
    assert_eq!(final_da.continuation, ContinuationState::Declarative);

    let action = entry("가다", PredicatePos::Verb, LexicalAlternation::Regular);
    let action_final_da = generate_predicate_branches(&action)
        .expect("valid fixture")
        .into_iter()
        .find(|branch| branch.anchor.as_ref() == "가다")
        .expect("dictionary form branch");
    assert_eq!(action_final_da.continuation, ContinuationState::Terminal);

    let mut negative_copula = entry(
        "아니다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    );
    negative_copula.flags = PredicateFlags::NO_DECLARATIVE_CONTINUATION;
    let negative_final_da = generate_predicate_branches(&negative_copula)
        .expect("valid fixture")
        .into_iter()
        .find(|branch| branch.anchor.as_ref() == "아니다")
        .expect("negative copula dictionary form branch");
    assert_eq!(negative_final_da.continuation, ContinuationState::Terminal);
}

#[test]
fn lexical_flag_can_forbid_i_eo_contraction() {
    let mut negative = entry(
        "아니다",
        PredicatePos::Adjective,
        LexicalAlternation::Regular,
    );
    negative.flags = PredicateFlags::NO_I_EO_CONTRACTION;
    let surfaces = surfaces(&negative);
    assert!(surfaces.contains("아니어"));
    assert!(!surfaces.contains("아녀"));
}

#[test]
fn branches_stop_before_productive_suffix_chains() {
    let branches =
        generate_predicate_branches(&entry("걷다", PredicatePos::Verb, LexicalAlternation::DToL))
            .expect("valid fixture");
    assert!(branches.iter().any(|branch| {
        branch.anchor.as_ref() == "걸었" && branch.continuation == ContinuationState::Past
    }));
    assert!(branches.iter().any(|branch| {
        branch.anchor.as_ref() == "걸으" && branch.continuation == ContinuationState::Eu
    }));
    assert!(
        !branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "걸으셨다")
    );
    assert!(branches.iter().any(|branch| {
        branch.anchor.as_ref() == "걷겠" && branch.continuation == ContinuationState::Future
    }));
    assert!(
        branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "걷지")
    );
    assert!(
        branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "걷게")
    );
}

#[test]
fn copula_rejects_noncanonical_stems() {
    let error = generate_predicate_branches(&entry(
        "보이다",
        PredicatePos::Copula,
        LexicalAlternation::Copula,
    ))
    .expect_err("copula morphology is defined only for 이다");
    assert!(matches!(error, GenerateError::AlternationMismatch { .. }));
}
