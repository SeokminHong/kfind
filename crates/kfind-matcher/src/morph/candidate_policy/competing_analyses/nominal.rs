use kfind_morph::{ConstraintResolver, RuleId};
use kfind_query::{CandidateConsumption, CandidateProgram};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{is_boundary_after, is_boundary_before, surrounding_token_span};

use super::{ADNOMINAL_RULE_IDS, ExecutedCandidate, context};

pub(super) fn has_competing_nominal_with_gu(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.connective-go")
        && *span == candidate.verified.token
        && haystack
            .get(span.clone())
            .is_some_and(|text| text.ends_with("구".as_bytes()) || text.ends_with("구".as_bytes()))
        && context::normalized_token(haystack, span.clone()).is_some_and(|token| {
            resolver.has_exact_nominal_surface(&token)
                && !context::previous_word(haystack, span.start).is_some_and(|previous| {
                    resolver.has_object_marked_nominal(&previous)
                        || (resolver.has_exact_adverb_surface(&previous)
                            && !resolver.has_exact_proper_name_surface(&token))
                })
        })
}

pub(super) fn has_nominal_interrogative_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let rule = candidate.suffix_rules.last().or_else(|| {
        let (first, rest) = branch.origins.split_first()?;
        let rule = first.rule_path.last()?;
        rest.iter()
            .all(|origin| origin.rule_path.last() == Some(rule))
            .then_some(rule)
    });
    let Some(rule) = rule.map(RuleId::as_str) else {
        return false;
    };
    let span = &candidate.consumed;
    let end = if rule == "ending.interrogative-nga" {
        span.end
    } else if ADNOMINAL_RULE_IDS.contains(&rule) {
        if !haystack.get(span.end..).is_some_and(|tail| {
            tail.starts_with("가".as_bytes()) || tail.starts_with("ᄀ".as_bytes())
        }) {
            return false;
        }
        let token = surrounding_token_span(haystack, span.clone());
        if !context::normalized_token(haystack, span.end..token.end)
            .is_some_and(|suffix| suffix == "가")
        {
            return false;
        }
        token.end
    } else {
        return false;
    };
    if !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, end)
        || !context::normalized_token(haystack, span.start..end)
            .is_some_and(|token| resolver.has_exact_nominal_surface(&token))
    {
        return false;
    }
    let Some((next_end, next)) = context::next_word_with_end(haystack, end) else {
        return false;
    };
    (context::previous_word(haystack, span.start)
        .is_some_and(|previous| resolver.has_nominal_genitive_surface(&previous))
        && resolver.has_complete_nominal_surface(&next))
        || context::next_word(haystack, next_end).is_some_and(|counter| {
            counter.strip_suffix(['을', '를']).is_some_and(|unit| {
                resolver.has_counted_unit_frame(&next, unit)
                    && resolver.has_object_marked_nominal(&counter)
            })
        })
}

pub(super) fn has_comparative_nominal_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if !candidate.suffix_rules.is_empty()
        || !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation {
                continuation: kfind_morph::ContinuationState::AOrEo,
                ..
            }
        )
        || !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
    {
        return false;
    }
    context::next_word(haystack, span.end)
        .is_some_and(|next| resolver.has_comparative_nominal_surface(&next))
        && context::normalized_token(haystack, span.clone())
            .is_some_and(|token| resolver.has_exact_nominal_surface(&token))
        && !context::previous_word(haystack, span.start).is_some_and(|previous| {
            resolver.has_nominal_oblique(&previous) || resolver.has_object_marked_nominal(&previous)
        })
}

pub(super) fn has_nama_proper_name_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let whole = &candidate.verified.token;
    let surface = &haystack[whole.clone()];
    if candidate.consumed.start != whole.start
        || !(surface.ends_with("나마".as_bytes()) || surface.ends_with("나마".as_bytes()))
        || !branch.origins.iter().any(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.concessive-nama")
        }) && !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation {
                continuation: kfind_morph::ContinuationState::AOrEo,
                ..
            }
        )
    {
        return false;
    }
    context::normalized_token(haystack, whole.clone())
        .is_some_and(|name| name.ends_with("나마") && resolver.has_exact_proper_name_surface(&name))
        && context::next_word(haystack, whole.end)
            .is_some_and(|head| resolver.has_nominal_with_optional_particles(&head))
        && !context::previous_word(haystack, whole.start)
            .is_some_and(|object| resolver.has_object_marked_nominal(&object))
}

pub(super) fn has_nominal_frame_for_volitional(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let anchor_rule = branch
        .origins
        .first()
        .and_then(|origin| origin.rule_path.last());
    let Some(rule) = candidate.suffix_rules.last().or(anchor_rule) else {
        return false;
    };
    if !matches!(
        rule.as_str(),
        "ending.volitional-mse" | "ending.nominalizer"
    ) {
        return false;
    }
    let consumed = &candidate.consumed;
    if !is_boundary_before(haystack, consumed.start) {
        return false;
    }
    let whole = if rule.as_str() == "ending.volitional-mse" {
        if !is_boundary_after(haystack, consumed.end) {
            return false;
        }
        consumed.clone()
    } else {
        // The same surface may also be reached by consuming only the nominalizer.
        if !haystack.get(consumed.end..).is_some_and(|suffix| {
            suffix.starts_with("세".as_bytes()) || suffix.starts_with("ᄉ".as_bytes())
        }) {
            return false;
        }
        let whole = surrounding_token_span(haystack, consumed.clone());
        if !haystack
            .get(consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|suffix| suffix.nfc().eq("세".chars()))
        {
            return false;
        }
        whole
    };
    (candidate.suffix_rules.last().is_some()
        || branch
            .origins
            .iter()
            .all(|origin| origin.rule_path.last() == Some(rule)))
        && (context::next_word_on_line(haystack, whole.end)
            .is_some_and(|(_, next)| resolver.has_causal_nominal_surface(&next))
            || context::previous_word(haystack, whole.start).is_some_and(|previous| {
                resolver.has_predicate_adnominal_surface(&previous)
                    || resolver.has_nominal_genitive_surface(&previous)
            }))
        && haystack
            .get(whole)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|token| {
                resolver.has_exact_nominal_surface(&token.nfc().collect::<String>())
            })
}

pub(super) fn has_nonpredicate_report_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let end = candidate.consumed.end;
    let Some(tail) = end
        .checked_sub(3)
        .and_then(|start| haystack.get(start..end))
    else {
        return false;
    };
    if !["대", "래", "재", "냬", "ᅢ", "ᅤ"]
        .iter()
        .any(|ending| tail == ending.as_bytes())
    {
        return false;
    }
    let is_report = |rule: &RuleId| rule.as_str() == "ending.report-contraction";
    if !(candidate.suffix_rules.iter().any(is_report)
        || (!branch.origins.is_empty()
            && branch
                .origins
                .iter()
                .all(|origin| origin.rule_path.iter().any(is_report))))
        || !is_boundary_before(haystack, candidate.consumed.start)
        || !is_boundary_after(haystack, candidate.consumed.end)
    {
        return false;
    }
    let Some((next_end, next)) = context::next_word_with_end(haystack, candidate.consumed.end)
    else {
        return false;
    };
    let Some(token) = context::normalized_token(haystack, candidate.consumed.clone()) else {
        return false;
    };
    if resolver.has_exact_adverb_surface(&token) && resolver.has_independent_final_predicate(&next)
    {
        return true;
    }
    if !resolver.has_exact_nominal_surface(&token) {
        return false;
    }
    context::next_word(haystack, next_end).is_some_and(|following| {
        let unit = following
            .strip_suffix(['이', '가', '을', '를', '은', '는'])
            .unwrap_or(&following);
        resolver.has_counted_unit_frame(&next, unit)
            || (resolver.has_complete_nominal_surface(&next)
                && resolver.has_independent_final_predicate(&following))
    })
}
