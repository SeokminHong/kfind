use crate::{ContinuationState, PredicatePos, RuleId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateContinuationMatch {
    /// Byte offset of the verified token end in `anchor + following`.
    pub token_end: usize,
    /// Number of bytes consumed from `following`.
    pub consumed_bytes: usize,
    pub rule_path: Vec<RuleId>,
}

#[derive(Debug, Clone, Copy)]
struct Suffix {
    surface: &'static str,
    rules: &'static [&'static str],
}

const A_OR_EO_SUFFIXES: &[Suffix] = &[
    suffix("가고", &["ending.auxiliary-gada", "ending.connective-go"]),
    suffix("가야", &["ending.auxiliary-gada", "ending.connective-ya"]),
    suffix(
        "졌습니다",
        &[
            "ending.auxiliary-jida",
            "ending.past",
            "ending.polite-declarative",
        ],
    ),
    suffix("서도", &["ending.aoeo-seo", "particle.additive"]),
    suffix(
        "졌다",
        &["ending.auxiliary-jida", "ending.past", "ending.final-da"],
    ),
    suffix("지면", &["ending.auxiliary-jida", "ending.conditional"]),
    suffix("지고", &["ending.auxiliary-jida", "ending.connective-go"]),
    suffix("진", &["ending.auxiliary-jida", "ending.past-adnominal"]),
    suffix("질", &["ending.auxiliary-jida", "ending.future-adnominal"]),
    suffix("지다", &["ending.auxiliary-jida", "ending.final-da"]),
    suffix("서", &["ending.aoeo-seo"]),
    suffix("도", &["ending.connective-do"]),
    suffix("야", &["ending.connective-ya"]),
    suffix("요", &["ending.polite-yo"]),
    suffix("라", &["ending.imperative-ra"]),
];

const PAST_SUFFIXES: &[Suffix] = &[
    suffix("습니다", &["ending.polite-declarative"]),
    suffix("으되", &["ending.connective-eudoe"]),
    suffix("느냐는", &["ending.interrogative-neunya", "particle.topic"]),
    suffix("으면", &["ending.conditional"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다고", &["ending.quotative-go"]),
    suffix("느냐", &["ending.interrogative-neunya"]),
    suffix("던", &["ending.retrospective-adnominal"]),
    suffix("을", &["ending.future-adnominal"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
    suffix("어요", &["ending.past-polite-yo"]),
];

const FUTURE_SUFFIXES: &[Suffix] = &[
    suffix("습니다", &["ending.polite-declarative"]),
    suffix("으되", &["ending.connective-eudoe"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
];

const DECLARATIVE_SUFFIXES: &[Suffix] = &[
    suffix("면서", &["ending.quotative-myeonseo"]),
    suffix("는데", &["ending.quotative-neunde"]),
    suffix("고", &["ending.quotative-go"]),
    suffix("는", &["ending.quotative-adnominal"]),
    suffix("던", &["ending.quotative-retrospective"]),
    suffix("면", &["ending.conditional"]),
    suffix("니", &["ending.quotative-ni"]),
    suffix("며", &["ending.quotative-myeo"]),
    suffix("지", &["ending.quotative-ji"]),
];

const EU_SUFFIXES: &[Suffix] = &[
    suffix("리라고", &["ending.prospective-quotative"]),
    suffix(
        "시겠습니다",
        &[
            "ending.honorific",
            "ending.future",
            "ending.polite-declarative",
        ],
    ),
    suffix(
        "셨습니다",
        &[
            "ending.honorific",
            "ending.past",
            "ending.polite-declarative",
        ],
    ),
    suffix(
        "셨다",
        &["ending.honorific", "ending.past", "ending.final-da"],
    ),
    suffix("십니다", &["ending.honorific", "ending.polite-declarative"]),
    suffix("시다", &["ending.honorific", "ending.final-da"]),
    suffix("시면", &["ending.honorific", "ending.conditional"]),
    suffix("신", &["ending.honorific", "ending.past-adnominal"]),
    suffix("실", &["ending.honorific", "ending.future-adnominal"]),
    suffix("면", &["ending.conditional"]),
    suffix("며", &["ending.coordinate-myeo"]),
    suffix("니", &["ending.connective-ni"]),
];

/// Consumes the longest suffix accepted by a predicate branch's verifier state.
#[must_use]
pub fn verify_predicate_continuation(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    let candidates = match state {
        ContinuationState::Terminal => {
            return Some(matched(anchor.len(), None));
        }
        ContinuationState::AOrEo => A_OR_EO_SUFFIXES,
        ContinuationState::Past => PAST_SUFFIXES,
        ContinuationState::Future => FUTURE_SUFFIXES,
        ContinuationState::Declarative => DECLARATIVE_SUFFIXES,
        ContinuationState::Eu => EU_SUFFIXES,
    };

    let suffix = candidates
        .iter()
        .filter(|suffix| {
            following.starts_with(suffix.surface)
                && (!suffix.rules.contains(&"ending.imperative-ra") || pos.is_action())
        })
        .max_by_key(|suffix| suffix.surface.len());
    if suffix.is_none() && state == ContinuationState::Eu {
        return None;
    }
    Some(matched(anchor.len(), suffix))
}

fn matched(anchor_len: usize, suffix: Option<&Suffix>) -> PredicateContinuationMatch {
    let consumed_bytes = suffix.map_or(0, |suffix| suffix.surface.len());
    let rule_path = suffix.map_or_else(Vec::new, |suffix| {
        suffix.rules.iter().copied().map(RuleId::from).collect()
    });
    PredicateContinuationMatch {
        token_end: anchor_len + consumed_bytes,
        consumed_bytes,
        rule_path,
    }
}

const fn suffix(surface: &'static str, rules: &'static [&'static str]) -> Suffix {
    Suffix { surface, rules }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn past_state_consumes_a_bounded_suffix() {
        let matched = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "걸었",
            "습니다. 다음",
        )
        .expect("valid continuation");
        assert_eq!(matched.consumed_bytes, "습니다".len());
        assert_eq!(matched.token_end, "걸었습니다".len());
        assert_eq!(matched.rule_path[0].as_str(), "ending.polite-declarative");

        let informal = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Adjective,
            "좋았",
            "어요",
        )
        .expect("past polite continuation");
        assert_eq!(informal.token_end, "좋았어요".len());
        assert_eq!(informal.rule_path[0].as_str(), "ending.past-polite-yo");
    }

    #[test]
    fn prefinal_states_consume_eudoe_connectives() {
        for (state, anchor) in [
            (ContinuationState::Past, "치렀"),
            (ContinuationState::Future, "하겠"),
        ] {
            let matched = verify_predicate_continuation(
                state,
                PredicatePos::Verb,
                anchor,
                "으되 조건은 남는다",
            )
            .expect("eudoe continuation");
            assert_eq!(matched.token_end, format!("{anchor}으되").len());
            assert_eq!(matched.rule_path, [RuleId::from("ending.connective-eudoe")]);
        }
    }

    #[test]
    fn declarative_state_consumes_bounded_continuations() {
        for (following, surface, rule_id) in [
            ("고 말했다", "쓴다고", "ending.quotative-go"),
            ("는 말", "쓴다는", "ending.quotative-adnominal"),
            ("던 말", "쓴다던", "ending.quotative-retrospective"),
            ("면 알겠다", "쓴다면", "ending.conditional"),
            ("니 놀랍다", "쓴다니", "ending.quotative-ni"),
            ("며 나섰다", "쓴다며", "ending.quotative-myeo"),
            ("면서 미뤘다", "쓴다면서", "ending.quotative-myeonseo"),
            ("는데 기다렸다", "쓴다는데", "ending.quotative-neunde"),
            ("지?", "쓴다지", "ending.quotative-ji"),
        ] {
            let matched = verify_predicate_continuation(
                ContinuationState::Declarative,
                PredicatePos::Verb,
                "쓴다",
                following,
            )
            .expect("declarative continuation");
            assert_eq!(matched.token_end, surface.len());
            assert_eq!(matched.rule_path, [RuleId::from(rule_id)]);
        }

        let bare = verify_predicate_continuation(
            ContinuationState::Declarative,
            PredicatePos::Verb,
            "쓴다",
            " 말했다",
        )
        .expect("bare declarative");
        assert_eq!(bare.consumed_bytes, 0);

        let unsupported = verify_predicate_continuation(
            ContinuationState::Declarative,
            PredicatePos::Verb,
            "쓴다",
            "도 말했다",
        )
        .expect("completed declarative remains valid");
        assert_eq!(unsupported.consumed_bytes, 0);
    }

    #[test]
    fn eu_state_requires_a_licensed_suffix() {
        let matched = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "걸으",
            "셨다.",
        )
        .expect("valid continuation");
        assert_eq!(matched.token_end, "걸으셨다".len());
        assert!(
            verify_predicate_continuation(ContinuationState::Eu, PredicatePos::Verb, "걸으", "xyz")
                .is_none()
        );

        let connective = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Adjective,
            "좋으",
            "니 계속한다",
        )
        .expect("reason connective");
        assert_eq!(connective.token_end, "좋으니".len());

        let prospective = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "얻으",
            "리라고 생각했다",
        )
        .expect("prospective quotative");
        assert_eq!(prospective.token_end, "얻으리라고".len());
    }

    #[test]
    fn continuation_respects_ending_paths_and_pos_requirements() {
        let future = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "걸으",
            "시겠습니다",
        )
        .expect("honorific future path");
        assert_eq!(future.token_end, "걸으시겠습니다".len());

        assert!(
            verify_predicate_continuation(ContinuationState::AOrEo, PredicatePos::Verb, "가", "라")
                .is_some()
        );
        let adjective = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Adjective,
            "예뻐",
            "라",
        )
        .expect("completed vowel anchor remains valid");
        assert_eq!(adjective.consumed_bytes, 0);
    }

    #[test]
    fn terminal_and_completed_vowel_states_accept_a_boundary() {
        for state in [ContinuationState::Terminal, ContinuationState::AOrEo] {
            let matched = verify_predicate_continuation(state, PredicatePos::Verb, "걸어", " 갔다")
                .expect("boundary is valid");
            assert_eq!(matched.consumed_bytes, 0);
        }
    }

    #[test]
    fn uses_the_longest_vowel_and_future_continuations() {
        let aeo = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "걸어",
            "서도 좋다",
        )
        .expect("valid continuation");
        assert_eq!(aeo.token_end, "걸어서도".len());
        assert_eq!(aeo.rule_path.len(), 2);

        let future = verify_predicate_continuation(
            ContinuationState::Future,
            PredicatePos::Verb,
            "걷겠",
            "습니다",
        )
        .expect("valid continuation");
        assert_eq!(future.token_end, "걷겠습니다".len());
    }

    #[test]
    fn accepts_gold_retrospective_quotative_and_change_suffixes() {
        let retrospective = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Adjective,
            "예뻤",
            "던",
        )
        .expect("retrospective adnominal");
        assert_eq!(retrospective.consumed_bytes, "던".len());

        let quotative = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "되었",
            "다고",
        )
        .expect("quotative connective");
        assert_eq!(quotative.consumed_bytes, "다고".len());

        let past_adnominal = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "불렀",
            "을 때",
        )
        .expect("past adnominal");
        assert_eq!(past_adnominal.token_end, "불렀을".len());
        assert_eq!(
            past_adnominal.rule_path[0].as_str(),
            "ending.future-adnominal"
        );

        let changed = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Adjective,
            "빨라",
            "졌다",
        )
        .expect("change auxiliary");
        assert_eq!(changed.consumed_bytes, "졌다".len());
    }

    #[test]
    fn accepts_bounded_progression_auxiliary_suffixes() {
        let coordinate = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "망해",
            "가고 있다",
        )
        .expect("progression coordinate");
        assert_eq!(coordinate.consumed_bytes, "가고".len());
        assert_eq!(
            coordinate.rule_path,
            ["ending.auxiliary-gada", "ending.connective-go"]
                .into_iter()
                .map(RuleId::from)
                .collect::<Vec<_>>()
        );

        let required = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "만들어",
            "가야 한다",
        )
        .expect("progression requirement");
        assert_eq!(required.consumed_bytes, "가야".len());

        let unsupported = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "해",
            "가며 배운다",
        )
        .expect("completed vowel anchor remains valid");
        assert_eq!(unsupported.consumed_bytes, 0);
    }

    #[test]
    fn accepts_bounded_past_interrogative_suffixes() {
        let topicalized = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "했",
            "느냐는 문제",
        )
        .expect("topicalized past interrogative");
        assert_eq!(topicalized.consumed_bytes, "느냐는".len());
        assert_eq!(
            topicalized.rule_path,
            ["ending.interrogative-neunya", "particle.topic"]
                .into_iter()
                .map(RuleId::from)
                .collect::<Vec<_>>()
        );

        let bare = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "먹었",
            "느냐?",
        )
        .expect("bare past interrogative");
        assert_eq!(bare.consumed_bytes, "느냐".len());

        let unsupported = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "했",
            "느냐도 논점이다",
        )
        .expect("past interrogative remains valid");
        assert_eq!(unsupported.consumed_bytes, "느냐".len());
    }
}
