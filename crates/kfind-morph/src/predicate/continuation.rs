use crate::{ContinuationState, RuleId};

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
    suffix("으면", &["ending.conditional"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다고", &["ending.quotative-go"]),
    suffix("던", &["ending.retrospective-adnominal"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
];

const FUTURE_SUFFIXES: &[Suffix] = &[
    suffix("습니다", &["ending.polite-declarative"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
];

const EU_SUFFIXES: &[Suffix] = &[
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
];

/// Consumes the longest suffix accepted by a predicate branch's verifier state.
#[must_use]
pub fn verify_predicate_continuation(
    state: ContinuationState,
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
        ContinuationState::Eu => EU_SUFFIXES,
    };

    let suffix = candidates
        .iter()
        .filter(|suffix| following.starts_with(suffix.surface))
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
        let matched =
            verify_predicate_continuation(ContinuationState::Past, "걸었", "습니다. 다음")
                .expect("valid continuation");
        assert_eq!(matched.consumed_bytes, "습니다".len());
        assert_eq!(matched.token_end, "걸었습니다".len());
        assert_eq!(matched.rule_path[0].as_str(), "ending.polite-declarative");
    }

    #[test]
    fn eu_state_requires_a_licensed_suffix() {
        let matched = verify_predicate_continuation(ContinuationState::Eu, "걸으", "셨다.")
            .expect("valid continuation");
        assert_eq!(matched.token_end, "걸으셨다".len());
        assert!(verify_predicate_continuation(ContinuationState::Eu, "걸으", "xyz").is_none());
    }

    #[test]
    fn terminal_and_completed_vowel_states_accept_a_boundary() {
        for state in [ContinuationState::Terminal, ContinuationState::AOrEo] {
            let matched =
                verify_predicate_continuation(state, "걸어", " 갔다").expect("boundary is valid");
            assert_eq!(matched.consumed_bytes, 0);
        }
    }

    #[test]
    fn uses_the_longest_vowel_and_future_continuations() {
        let aeo = verify_predicate_continuation(ContinuationState::AOrEo, "걸어", "서도 좋다")
            .expect("valid continuation");
        assert_eq!(aeo.token_end, "걸어서도".len());
        assert_eq!(aeo.rule_path.len(), 2);

        let future = verify_predicate_continuation(ContinuationState::Future, "걷겠", "습니다")
            .expect("valid continuation");
        assert_eq!(future.token_end, "걷겠습니다".len());
    }

    #[test]
    fn accepts_gold_retrospective_quotative_and_change_suffixes() {
        let retrospective = verify_predicate_continuation(ContinuationState::Past, "예뻤", "던")
            .expect("retrospective adnominal");
        assert_eq!(retrospective.consumed_bytes, "던".len());

        let quotative = verify_predicate_continuation(ContinuationState::Past, "되었", "다고")
            .expect("quotative connective");
        assert_eq!(quotative.consumed_bytes, "다고".len());

        let changed = verify_predicate_continuation(ContinuationState::AOrEo, "빨라", "졌다")
            .expect("change auxiliary");
        assert_eq!(changed.consumed_bytes, "졌다".len());
    }
}
