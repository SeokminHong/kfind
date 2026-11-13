use crate::{ContinuationState, PredicatePos, RuleId};

use super::{
    JIYO, JYO, MAE, PredicateContinuationMatch, STEM_FIXED_ENDINGS, Suffix, matched, suffix,
    verify_simple_continuation,
};

const RESULTATIVE_STEM_ENDINGS: &[Suffix] = &[
    JIYO,
    JYO,
    suffix("다", &["ending.final-da"]),
    suffix("고요", &["ending.connective-go", "ending.polite-yo"]),
    suffix("네", &["ending.final-ne"]),
    suffix("네요", &["ending.final-ne", "ending.polite-yo"]),
    suffix("더군", &["ending.final-deogun"]),
    suffix("더군요", &["ending.final-deogun", "ending.polite-yo"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("므로", &["ending.connective-meuro"]),
    MAE,
    suffix("기에", &["ending.connective-gie"]),
    suffix("는구나", &["ending.final-neun-exclamative"]),
    suffix("는군", &["ending.final-neun-exclamative"]),
];

pub(super) fn verify_resultative(
    anchor_len: usize,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    use ContinuationState::{
        AOrEo, BieupFinal, CompletedNieunEnding, Declarative, Future, Interrogative, NieunEnding,
        Past, RieulFinal, Terminal,
    };

    let (form, state, initial_rule) = match following.chars().next()? {
        '진' if following.starts_with("진다") => {
            ("진다", Declarative, Some("ending.declarative"))
        }
        '진' => ("진", CompletedNieunEnding, Some("ending.past-adnominal")),
        '질' => ("질", RieulFinal, Some("ending.future-adnominal")),
        '짐' => ("짐", Terminal, Some("ending.nominalizer")),
        '집' => ("집", BieupFinal, None),
        '져' => ("져", AOrEo, Some("contraction.i-eo")),
        '졌' => ("졌", Past, Some("ending.past")),
        '지' if following.starts_with("지겠") => ("지겠", Future, Some("ending.future")),
        '지' if following.starts_with("지느냐") => {
            ("지느냐", Interrogative, Some("ending.interrogative-neunya"))
        }
        '지' if following.starts_with("지어") => ("지어", AOrEo, Some("ending.aoeo")),
        '지' if following.starts_with("지기") => {
            ("지기", Terminal, Some("ending.nominalizer-gi"))
        }
        '지' => {
            let tail = &following['지'.len_utf8()..];
            if let Some(background) = tail.strip_prefix('는')
                && !background.starts_with('군')
                && !background.starts_with("구나")
            {
                ("지는", NieunEnding, Some("ending.present-adnominal"))
            } else {
                return verify_stem(
                    anchor_len,
                    "지",
                    tail,
                    "ending.auxiliary-jida",
                    RESULTATIVE_STEM_ENDINGS,
                );
            }
        }
        _ => return None,
    };
    verify_form(
        anchor_len,
        following,
        form,
        state,
        initial_rule,
        "ending.auxiliary-jida",
    )
}

pub(super) fn verify_form(
    anchor_len: usize,
    following: &str,
    form: &str,
    state: ContinuationState,
    initial_rule: Option<&str>,
    auxiliary_rule: &str,
) -> Option<PredicateContinuationMatch> {
    use ContinuationState::{CompletedNieunEnding, Future, NieunEnding, Past, RieulFinal};
    let tail = &following[form.len()..];
    let mut continuation =
        verify_simple_continuation(state, PredicatePos::AuxiliaryVerb, form, tail).or_else(
            || {
                matches!(state, CompletedNieunEnding | NieunEnding | RieulFinal)
                    .then(|| matched(form.len(), None))
            },
        )?;
    if matches!(state, Past | Future) && continuation.consumed_bytes == 0 {
        return None;
    }
    let initial_rule = match (state, continuation.consumed_bytes) {
        (RieulFinal | NieunEnding | CompletedNieunEnding, consumed) if consumed > 0 => None,
        _ => initial_rule,
    };
    if initial_rule.is_some_and(|id| id.starts_with("contraction."))
        && state == ContinuationState::AOrEo
    {
        continuation
            .rule_path
            .insert(0, RuleId::from("ending.aoeo"));
    }
    Some(with_auxiliary(
        anchor_len,
        form,
        initial_rule,
        continuation,
        auxiliary_rule,
    ))
}

pub(super) fn verify_stem(
    anchor_len: usize,
    stem: &str,
    tail: &str,
    auxiliary_rule: &str,
    stem_endings: &[Suffix],
) -> Option<PredicateContinuationMatch> {
    let fixed = STEM_FIXED_ENDINGS
        .iter()
        .chain(stem_endings)
        .filter(|suffix| tail.starts_with(suffix.surface))
        .max_by_key(|suffix| suffix.surface.len())
        .map(|suffix| matched(stem.len(), Some(suffix)));
    let continuation = [ContinuationState::StemEnding, ContinuationState::Eu]
        .into_iter()
        .filter_map(|state| {
            verify_simple_continuation(state, PredicatePos::AuxiliaryVerb, stem, tail)
        })
        .chain(fixed)
        .max_by_key(|matched| matched.consumed_bytes)?;
    Some(with_auxiliary(
        anchor_len,
        stem,
        None,
        continuation,
        auxiliary_rule,
    ))
}

fn with_auxiliary(
    anchor_len: usize,
    form: &str,
    initial_rule: Option<&str>,
    mut continuation: PredicateContinuationMatch,
    auxiliary_rule: &str,
) -> PredicateContinuationMatch {
    continuation.token_end += anchor_len;
    continuation.consumed_bytes += form.len();
    if let Some(rule) = initial_rule {
        continuation.rule_path.insert(0, RuleId::from(rule));
    }
    continuation
        .rule_path
        .insert(0, RuleId::from(auxiliary_rule));
    continuation
}
