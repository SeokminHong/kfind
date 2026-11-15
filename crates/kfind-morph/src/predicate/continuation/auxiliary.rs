use std::sync::OnceLock;

use crate::{
    ContinuationState, LexicalAlternation, PredicateEntry, PredicatePos, RuleId, SurfaceBranchSpec,
    generate_predicate_branches,
};

use super::{
    JIMAN, JIMANEUN, JIYO, JYO, MAE, PredicateContinuationMatch, STEM_FIXED_ENDINGS, Suffix,
    matched, suffix, verify_prefinal_chain, verify_simple_continuation,
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
    JIMAN,
    JIMANEUN,
    suffix("므로", &["ending.connective-meuro"]),
    MAE,
    suffix("기에", &["ending.connective-gie"]),
    suffix("는구나", &["ending.final-neun-exclamative"]),
    suffix("는군", &["ending.final-neun-exclamative"]),
];

pub(super) fn verify_progression_oda(
    anchor_len: usize,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    let (form, state, rule) = if following.starts_with('온') {
        (
            "온",
            ContinuationState::CompletedNieunEnding,
            "ending.past-adnominal",
        )
    } else {
        ("왔", ContinuationState::Past, "ending.past")
    };
    verify_form(
        anchor_len,
        following,
        form,
        state,
        Some(rule),
        "ending.auxiliary-oda",
    )
}

pub(super) fn verify_progression_gada(
    anchor_len: usize,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    let (form, state, rule) = if following.starts_with('갈') {
        (
            "갈",
            ContinuationState::RieulFinal,
            "ending.future-adnominal",
        )
    } else {
        ("갔", ContinuationState::Past, "ending.past")
    };
    verify_form(
        anchor_len,
        following,
        form,
        state,
        Some(rule),
        "ending.auxiliary-gada",
    )
}

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
                if let Some(rest) = tail.strip_prefix('자')
                    && !rest.starts_with(|ch: char| ch.is_alphanumeric() || ch == '_')
                {
                    return Some(with_auxiliary(
                        anchor_len,
                        "지",
                        None,
                        matched(
                            '지'.len_utf8(),
                            Some(&suffix("자", &["ending.connective-ja"])),
                        ),
                        "ending.auxiliary-jida",
                    ));
                }
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

pub(super) fn verify_complete_vowel_auxiliary(
    anchor_len: usize,
    host: PredicatePos,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    static COMPLETION: OnceLock<Vec<SurfaceBranchSpec>> = OnceLock::new();
    static STATIVE: OnceLock<Vec<SurfaceBranchSpec>> = OnceLock::new();

    // 한국어기초사전의 보조 동사 문형. 두 구성의 선행 용언 품사는 다르다.
    let (branches, lemma, alternation, rule) = match host.execution() {
        PredicatePos::Verb => (
            &COMPLETION,
            "내다",
            LexicalAlternation::Regular,
            "ending.auxiliary-naeda",
        ),
        PredicatePos::Adjective => (
            &STATIVE,
            "하다",
            LexicalAlternation::Ha,
            "ending.auxiliary-stative-hada",
        ),
        _ => return None,
    };
    let initial = following.chars().next()?;
    let branches = branches.get_or_init(|| {
        let entry = PredicateEntry::new(lemma, PredicatePos::AuxiliaryVerb, alternation);
        let mut branches = generate_predicate_branches(&entry).expect("valid auxiliary entry");
        if host.execution() == PredicatePos::Adjective {
            // Preserve the vowel and past paths while lexicalized compounds can
            // compete with a productive -어하다 analysis.
            branches.retain(|branch| {
                matches!(
                    branch.continuation,
                    ContinuationState::AOrEo | ContinuationState::Past
                )
            });
        }
        branches.sort_by(|left, right| left.anchor.cmp(&right.anchor));
        branches
    });
    let start = branches.partition_point(|branch| branch.anchor.chars().next() < Some(initial));
    if !branches.get(start)?.anchor.starts_with(initial) {
        return None;
    }
    let word = following
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .next()?;
    branches[start..]
        .iter()
        .take_while(|branch| branch.anchor.starts_with(initial))
        .filter_map(|branch| {
            let tail = word.strip_prefix(branch.anchor.as_ref())?;
            let continuation = if tail.is_empty() {
                if branch.rule_path.is_empty()
                    || matches!(
                        branch.continuation,
                        ContinuationState::Past | ContinuationState::Future
                    )
                {
                    return None;
                }
                matched(branch.anchor.len(), None)
            } else {
                verify_prefinal_chain(branch.continuation, branch.pos, branch.anchor.len(), tail)
                    .filter(|matched| matched.consumed_bytes == tail.len())
                    .or_else(|| {
                        verify_simple_continuation(
                            branch.continuation,
                            branch.pos,
                            &branch.anchor,
                            tail,
                        )
                    })?
            };
            (continuation.token_end == word.len()).then_some((branch, continuation))
        })
        .max_by_key(|(branch, _)| branch.anchor.len())
        .map(|(branch, continuation)| PredicateContinuationMatch {
            token_end: anchor_len + word.len(),
            consumed_bytes: word.len(),
            rule_path: std::iter::once(RuleId::from(rule))
                .chain(
                    branch
                        .rule_path
                        .iter()
                        .filter(|id| id.as_str().starts_with("ending."))
                        .cloned(),
                )
                .chain(continuation.rule_path)
                .collect(),
        })
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
    let ordinary = verify_simple_continuation(state, PredicatePos::AuxiliaryVerb, form, tail);
    let stacked = if state == Past && tail.starts_with(['었', '겠']) {
        verify_prefinal_chain(state, PredicatePos::AuxiliaryVerb, form.len(), tail)
    } else {
        None
    };
    let mut continuation = ordinary
        .into_iter()
        .chain(stacked)
        .max_by_key(|matched| matched.consumed_bytes)
        .or_else(|| {
            matches!(state, CompletedNieunEnding | NieunEnding | RieulFinal)
                .then(|| matched(form.len(), None))
        })?;
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
