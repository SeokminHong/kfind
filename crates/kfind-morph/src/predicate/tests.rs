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
        ContinuationState::Eu => vec![
            format!("{}면", branch.anchor),
            format!("{}며", branch.anchor),
            format!("{}셨다", branch.anchor),
            format!("{}시다", branch.anchor),
            format!("{}십니다", branch.anchor),
        ],
    }
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
    assert_has_all(&copula, &["이고", "이어", "여서", "인", "일"]);
}

#[test]
fn action_present_declarative_and_copula_past_are_compiled() {
    let action = entry("검증하다", PredicatePos::Verb, LexicalAlternation::Ha);
    let action_surfaces = surfaces(&action);
    assert!(action_surfaces.contains("검증한다"));

    let copula = entry("이다", PredicatePos::Copula, LexicalAlternation::Copula);
    let copula_surfaces = surfaces(&copula);
    assert!(copula_surfaces.contains("이었"));
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
