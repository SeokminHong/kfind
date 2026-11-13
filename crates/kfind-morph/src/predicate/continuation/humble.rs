use super::{JIMAN, JIMANEUN, PredicateContinuationMatch, Suffix, matched, suffix};
use crate::hangul::JONG_RIEUL;
use crate::{ContinuationState, PredicatePos, RuleId, decompose_syllable, has_final};

const HUMBLE: &str = "ending.archaic-humble";
const NAIDA: &str = "ending.archaic-naida";
const HONORIFIC: &str = "ending.honorific";

const FINALS: &[Suffix] = &[suffix("나이다", &[NAIDA]), suffix("나이까", &[NAIDA])];
const HONORIFIC_FINALS: &[Suffix] = &[
    suffix("시나이다", &[HONORIFIC, NAIDA]),
    suffix("시나이까", &[HONORIFIC, NAIDA]),
    suffix("셨나이다", &[HONORIFIC, "ending.past", NAIDA]),
    suffix("셨나이까", &[HONORIFIC, "ending.past", NAIDA]),
    suffix("시겠나이다", &[HONORIFIC, "ending.future", NAIDA]),
    suffix("시겠나이까", &[HONORIFIC, "ending.future", NAIDA]),
];
const OP_PREFIXES: &[Suffix] = &[
    suffix("옵", &[HUMBLE]),
    suffix("시옵", &[HONORIFIC, HUMBLE]),
];
const CONSONANT_PREFIXES: &[Suffix] = &[
    suffix("사오", &[HUMBLE]),
    suffix("사옵", &[HUMBLE]),
    suffix("삽", &[HUMBLE]),
];
// These forms survive with the lexicalized object-honorific stems.
const OBJECT_HONORIFIC_PREFIXES: &[Suffix] = &[
    suffix("자오", &[HUMBLE]),
    suffix("자옵", &[HUMBLE]),
    suffix("잡", &[HUMBLE]),
];
const OP_FOLLOWING: &[Suffix] = &[
    suffix("소서", &["ending.entreaty-soseo"]),
    suffix("니다", &["ending.polite-declarative"]),
    suffix("니까", &["ending.formal-interrogative"]),
    suffix("디다", &["ending.retrospective-declarative"]),
    suffix("디까", &["ending.retrospective-interrogative"]),
    suffix("고", &["ending.connective-go"]),
    suffix("더니", &["ending.retrospective-connective"]),
    JIMAN,
    JIMANEUN,
    suffix("건대", &["ending.connective-geondae"]),
    suffix("시고", &[HONORIFIC, "ending.connective-go"]),
    suffix("시니", &[HONORIFIC, "ending.connective-ni"]),
    suffix("시니까", &[HONORIFIC, "ending.connective-ni"]),
    suffix("시면", &[HONORIFIC, "ending.conditional"]),
    suffix("시어", &[HONORIFIC, "ending.aoeo"]),
];
const O_FOLLOWING: &[Suffix] = &[
    suffix("니", &["ending.connective-ni"]),
    suffix("니까", &["ending.connective-ni"]),
    suffix("면", &["ending.conditional"]),
    suffix("시면", &[HONORIFIC, "ending.conditional"]),
    suffix("리다", &["ending.prospective-final"]),
    suffix("리까", &["ending.prospective-final"]),
    suffix("리이다", &["ending.prospective-final"]),
    suffix("리이까", &["ending.prospective-final"]),
    suffix("이다", &["ending.archaic-ida"]),
];

pub(super) fn verify(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    use ContinuationState::{Eu, Future, Past, StemEnding};
    let first = following.chars().next()?;
    if state == StemEnding && anchor.ends_with('옵') {
        let ending = OP_FOLLOWING
            .iter()
            .chain(FINALS)
            .filter(|ending| following.starts_with(ending.surface))
            .max_by_key(|ending| ending.surface.len())?;
        return Some(matched(anchor.len(), Some(ending)));
    }
    let finals = match (first, state) {
        ('나', StemEnding)
            if following.starts_with("나이")
                && !anchor
                    .chars()
                    .next_back()
                    .and_then(decompose_syllable)
                    .is_some_and(|syllable| syllable.jongseong == JONG_RIEUL) =>
        {
            FINALS
        }
        ('나', Past | Future) => FINALS,
        ('시' | '셨', Eu) => HONORIFIC_FINALS,
        _ => &[],
    };
    if let Some(ending) = finals
        .iter()
        .find(|ending| following.starts_with(ending.surface))
    {
        return Some(matched(anchor.len(), Some(ending)));
    }

    let prefixes = match (first, state) {
        ('옵' | '시', Eu) => OP_PREFIXES,
        ('옵', StemEnding) if pos == PredicatePos::Copula || anchor.ends_with('시') => {
            OP_PREFIXES
        }
        ('자' | '잡', StemEnding) if matches!(anchor, "듣" | "묻" | "받" | "좇") => {
            OBJECT_HONORIFIC_PREFIXES
        }
        ('사' | '삽', StemEnding | Past | Future)
            if anchor.chars().next_back().is_some_and(has_final) =>
        {
            CONSONANT_PREFIXES
        }
        _ => return None,
    };
    let (prefix, tail, consumed) = prefixes.iter().find_map(|prefix| {
        let rest = following.strip_prefix(prefix.surface)?;
        let tails = if prefix.surface.ends_with('오') {
            O_FOLLOWING
        } else {
            OP_FOLLOWING
        };
        let tail = tails
            .iter()
            .chain(if prefix.surface.ends_with('오') {
                &[]
            } else {
                FINALS
            })
            .filter(|tail| rest.starts_with(tail.surface))
            .max_by_key(|tail| tail.surface.len())?;
        Some((prefix, tail, prefix.surface.len() + tail.surface.len()))
    })?;
    let rule_path = prefix
        .rules
        .iter()
        .chain(tail.rules)
        .map(|rule| RuleId::from(*rule))
        .collect();
    Some(PredicateContinuationMatch {
        token_end: anchor.len() + consumed,
        consumed_bytes: consumed,
        rule_path,
    })
}
