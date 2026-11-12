use super::{Suffix, suffix};
use crate::{ContinuationState, PredicatePos};

macro_rules! hao_endings {
    ($prefix:literal, [$($prior:literal),*] $(, $plain:expr)*) => {
        &[
            $($plain,)*
            suffix(concat!($prefix, "오니까"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오리다"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오리까"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오리이까"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오리이다"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오이다"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오이까"), &[$($prior,)* "ending.hao-respectful-final"]),
            suffix(concat!($prefix, "오니"), &[$($prior,)* "ending.hao-respectful-connective"]),
            suffix(concat!($prefix, "오면"), &[$($prior,)* "ending.hao-respectful-connective"]),
        ]
    };
}

pub(in crate::predicate) const HAO_ENDINGS: &[Suffix] =
    hao_endings!("", [], suffix("오", &["ending.hao-final"]), HAO_U);
pub(in crate::predicate) const HONORIFIC_HAO_ENDINGS: &[Suffix] = hao_endings!(
    "시",
    ["ending.honorific"],
    suffix("시오", &["ending.honorific", "ending.hao-final"]),
    HONORIFIC_HAO_U
);
pub(in crate::predicate) const HONORIFIC_HAO_AFTER_ACTION: &[Suffix] =
    hao_endings!("시", ["ending.honorific"], HONORIFIC_HAO_U);
const HONORIFIC_HAO_U: Suffix = suffix("시우", &["ending.honorific", "ending.hao-final-u"]);
const PREFINAL_HAO_ENDINGS: &[Suffix] = hao_endings!("으", []);
const HONORIFIC_PAST_HAO_ENDINGS: &[Suffix] =
    hao_endings!("셨으", ["ending.honorific", "ending.past"]);
const HONORIFIC_FUTURE_HAO_ENDINGS: &[Suffix] =
    hao_endings!("시겠으", ["ending.honorific", "ending.future"]);
const HAO_SO: Suffix = suffix("소", &["ending.hao-final-so"]);
pub(super) const HAO_U: Suffix = suffix("우", &["ending.hao-final-u"]);
pub(in crate::predicate) const SOSEO: Suffix = suffix("소서", &["ending.entreaty-soseo"]);
const SOIDA: Suffix = suffix("소이다", &["ending.hao-final-soida"]);
const CONSONANT_HAO_FINALS: &[Suffix] = &[HAO_SO, SOIDA];
macro_rules! prefinal_so_endings {
    ($prefix:literal, [$($prior:literal),*] $(, $plain:expr)*) => {
        &[
            $($plain,)*
            suffix(concat!($prefix, "소"), &[$($prior,)* "ending.hao-final-so"]),
            suffix(concat!($prefix, "소다"), &[$($prior,)* "ending.hao-final-soda"]),
            suffix(concat!($prefix, "소이까"), &[$($prior,)* "ending.hao-final-soida"]),
            suffix(concat!($prefix, "소이다"), &[$($prior,)* "ending.hao-final-soida"]),
        ]
    };
}
const PAST_HAO_FINALS: &[Suffix] = prefinal_so_endings!("", [], HAO_U);
const FUTURE_HAO_FINALS: &[Suffix] = prefinal_so_endings!("", []);
const HONORIFIC_PAST_HAO_FINALS: &[Suffix] = prefinal_so_endings!(
    "셨",
    ["ending.honorific", "ending.past"],
    suffix(
        "셨우",
        &["ending.honorific", "ending.past", "ending.hao-final-u"]
    )
);
const HONORIFIC_FUTURE_HAO_FINALS: &[Suffix] =
    prefinal_so_endings!("시겠", ["ending.honorific", "ending.future"]);

fn takes_bare_u(stem: &str) -> bool {
    stem.chars()
        .next_back()
        .and_then(crate::decompose_syllable)
        .is_some_and(|last| {
            matches!(
                last.jongseong,
                crate::hangul::JONG_BIEUP_SIOT | crate::hangul::JONG_SSANGSIOT
            )
        })
}

pub(super) fn additional_endings(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> &'static [Suffix] {
    match state {
        ContinuationState::StemEnding
            if pos != PredicatePos::Copula && following.starts_with('소') =>
        {
            if anchor.chars().next_back().is_some_and(crate::has_final) {
                CONSONANT_HAO_FINALS
            } else {
                std::slice::from_ref(&HAO_SO)
            }
        }
        ContinuationState::StemEnding if following.starts_with('우') && takes_bare_u(anchor) => {
            std::slice::from_ref(&HAO_U)
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula && following.starts_with(['오', '우']) =>
        {
            HAO_ENDINGS
        }
        ContinuationState::Eu
            if following.starts_with('오')
                || (following.starts_with('우')
                    && !anchor.strip_suffix('으').is_some_and(takes_bare_u)) =>
        {
            HAO_ENDINGS
        }
        ContinuationState::Eu
            if (following.starts_with("시오") || following.starts_with("시우")) =>
        {
            if pos.is_action() {
                HONORIFIC_HAO_AFTER_ACTION
            } else {
                HONORIFIC_HAO_ENDINGS
            }
        }
        ContinuationState::Past if following.starts_with(['소', '우']) => PAST_HAO_FINALS,
        ContinuationState::Future if following.starts_with('소') => FUTURE_HAO_FINALS,
        ContinuationState::Eu if following.starts_with("소서") => std::slice::from_ref(&SOSEO),
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으오") => {
            PREFINAL_HAO_ENDINGS
        }
        ContinuationState::Eu
            if (following.starts_with("셨소") || following.starts_with("셨우")) =>
        {
            HONORIFIC_PAST_HAO_FINALS
        }
        ContinuationState::Eu if following.starts_with("시겠소") => HONORIFIC_FUTURE_HAO_FINALS,
        ContinuationState::Eu if following.starts_with("셨으오") => HONORIFIC_PAST_HAO_ENDINGS,
        ContinuationState::Eu if following.starts_with("시겠으오") => {
            HONORIFIC_FUTURE_HAO_ENDINGS
        }
        _ => &[],
    }
}
