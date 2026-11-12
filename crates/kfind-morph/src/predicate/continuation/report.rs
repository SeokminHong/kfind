use super::{PredicateContinuationMatch, verify_simple_continuation};
use crate::{ContinuationState, PredicatePos, RuleId, has_final, has_rieul_final};

const REPORT_CONTRACTION: &str = "ending.report-contraction";

pub(super) fn verify(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    if !matches!(
        state,
        ContinuationState::StemEnding
            | ContinuationState::Eu
            | ContinuationState::NieunEnding
            | ContinuationState::Prospective
            | ContinuationState::Past
            | ContinuationState::Future
            | ContinuationState::ReportEnding
    ) {
        return None;
    }
    let (honorific, following) = honorific_prefix(state, pos, following);
    let initial = following.chars().next()?;
    let last = anchor.chars().next_back();
    // Report-ending anchors are fused -ㄴ declaratives or stems after ㄹ deletion.
    let fused_declarative =
        state == ContinuationState::ReportEnding && pos.is_action() && last.is_some_and(has_final);
    let (prefix, ending) = match initial {
        '대' | '댔'
            if matches!(state, ContinuationState::Past | ContinuationState::Future)
                || fused_declarative
                || !honorific.is_empty()
                || (state == ContinuationState::Prospective && anchor.ends_with('시'))
                || (state == ContinuationState::NieunEnding
                    && pos.is_action()
                    && anchor.ends_with('신'))
                || (state == ContinuationState::StemEnding
                    && !pos.is_action()
                    && pos != PredicatePos::Copula
                    && anchor != "아니") =>
        {
            ("", initial)
        }
        '래' | '랬'
            if pos.is_action()
                && matches!(
                    state,
                    ContinuationState::Eu | ContinuationState::Prospective
                ) =>
        {
            ("", initial)
        }
        '랬' if state == ContinuationState::StemEnding
            && (pos == PredicatePos::Copula || anchor == "아니") =>
        {
            ("", initial)
        }
        '재' | '쟀' if state == ContinuationState::StemEnding && pos.is_action() => ("", initial),
        '냬' if matches!(state, ContinuationState::Past | ContinuationState::Future)
            || (state == ContinuationState::StemEnding && !last.is_some_and(has_rieul_final))
            || (state == ContinuationState::ReportEnding && !fused_declarative)
            || (state == ContinuationState::Eu && (!pos.is_action() || !honorific.is_empty())) =>
        {
            ("", initial)
        }
        '느' if matches!(state, ContinuationState::Past | ContinuationState::Future)
            || (state == ContinuationState::ReportEnding
                && pos.is_action()
                && !fused_declarative)
            || (state == ContinuationState::StemEnding
                && !last.is_some_and(has_rieul_final)
                && (pos.is_action() || anchor.ends_with(['있', '없']))) =>
        {
            ("느", following["느".len()..].chars().next()?)
        }
        '는' if state == ContinuationState::StemEnding
            && pos.is_action()
            && last.is_some_and(|last| has_final(last) && !has_rieul_final(last)) =>
        {
            ("는", following["는".len()..].chars().next()?)
        }
        _ => return None,
    };
    let past = matches!(ending, '댔' | '랬' | '쟀');
    if (prefix == "느" && ending != '냬') || (prefix == "는" && !matches!(ending, '대' | '댔'))
    {
        return None;
    }
    let contracted_len = prefix.len() + ending.len_utf8();
    let remaining = &following[contracted_len..];
    let mut rule_path = Vec::new();
    if !honorific.is_empty() {
        rule_path.push(RuleId::from("ending.honorific"));
        if honorific.starts_with('셨') {
            rule_path.push(RuleId::from("ending.past"));
        }
        if honorific.ends_with('겠') {
            rule_path.push(RuleId::from("ending.future"));
        }
    }
    rule_path.push(RuleId::from(REPORT_CONTRACTION));
    let trailing = if past {
        let matched = verify_simple_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            &following[..contracted_len],
            remaining,
        )?;
        if matched.consumed_bytes == 0 {
            return None;
        }
        rule_path.push(RuleId::from("ending.past"));
        rule_path.extend(matched.rule_path);
        matched.consumed_bytes
    } else if remaining.starts_with('요') {
        rule_path.push(RuleId::from("ending.polite-yo"));
        '요'.len_utf8()
    } else {
        0
    };
    let consumed_bytes = honorific.len() + contracted_len + trailing;
    Some(PredicateContinuationMatch {
        token_end: anchor.len() + consumed_bytes,
        consumed_bytes,
        rule_path,
    })
}

fn honorific_prefix(state: ContinuationState, pos: PredicatePos, following: &str) -> (&str, &str) {
    if state != ContinuationState::Eu || !following.starts_with(['시', '신', '셨']) {
        return ("", following);
    }
    for prefix in ["셨겠", "시겠", "셨", "신", "시"] {
        if let Some(tail) = following.strip_prefix(prefix) {
            let eligible = match prefix {
                "신" => pos.is_action() && tail.starts_with(['대', '댔']),
                "시" => {
                    tail.starts_with(['대', '댔', '냬'])
                        || (pos.is_action() && tail.starts_with(['래', '랬']))
                }
                _ => tail.starts_with(['대', '댔', '냬']),
            };
            if eligible {
                return (prefix, tail);
            }
        }
    }
    ("", following)
}
