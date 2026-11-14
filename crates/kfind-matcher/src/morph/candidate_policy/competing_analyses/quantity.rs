use kfind_morph::ConstraintResolver;
use kfind_query::{AtomPlan, CandidateConsumption, CandidateProgram};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{is_boundary_after, is_boundary_before, surrounding_token_span};

use super::{ExecutedCandidate, context, has_counted_unit_copula_ending};

pub(super) fn has_counted_unit_copula(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let (CandidateConsumption::PredicateContinuation { pos, .. }
    | CandidateConsumption::StructuralPredicateEnding { pos, .. }) = branch.consumption
    else {
        return false;
    };
    let core = &candidate.verified.core;
    if pos == kfind_morph::PredicatePos::Copula
        || !has_counted_unit_copula_ending(haystack, core.end..candidate.consumed.end)
        || !is_boundary_before(haystack, core.start)
    {
        return false;
    }
    let Some(previous) = context::previous_word(haystack, core.start) else {
        return false;
    };
    haystack
        .get(core.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .is_some_and(|unit| {
            resolver.has_counted_unit_frame(&previous, &unit.nfc().collect::<String>())
        })
}

pub(super) fn has_elapsed_measure_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    let consumed = &candidate.consumed;
    if (branch.anchor.as_ref() != "만".as_bytes()
        && branch.anchor.as_ref() != "\u{1106}\u{1161}\u{11ab}".as_bytes())
        || !candidate
            .suffix_rules
            .first()
            .is_some_and(|rule| matches!(rule.as_str(), "particle.locative" | "particle.genitive"))
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == kfind_morph::CoarsePos::Numeral)
        })
        || !is_boundary_before(haystack, core.start)
        || !is_boundary_after(haystack, consumed.end)
    {
        return false;
    }
    context::previous_word_with_start(haystack, core.start).is_some_and(|(start, measure)| {
        let count = context::previous_word(haystack, start);
        resolver.has_elapsed_measure(&measure, count.as_deref())
    })
}

pub(super) fn has_quantifying_adverb_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    if (branch.anchor.as_ref() != "불과".as_bytes()
        && branch.anchor.as_ref() != "\u{1107}\u{116e}\u{11af}\u{1100}\u{116a}".as_bytes())
        || *core != candidate.consumed
        || !is_boundary_before(haystack, core.start)
        || !is_boundary_after(haystack, core.end)
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == kfind_morph::CoarsePos::Noun)
        })
    {
        return false;
    }
    context::next_word_with_end(haystack, core.end).is_some_and(|(end, next)| {
        let unit = context::next_word(haystack, end);
        resolver.has_quantity_expression(&next, unit.as_deref())
    })
}

pub(super) fn has_postnominal_count_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| matches!(rule.as_str(), "ending.aoeo" | "ending.connective-ji"))
        })
        || candidate.suffix_rules.iter().any(|rule| {
            !rule.as_str().starts_with("particle.") && rule.as_str() != "structural.ending-path"
        })
        || !is_boundary_before(haystack, candidate.consumed.start)
    {
        return false;
    }
    let span = surrounding_token_span(haystack, candidate.consumed.clone());
    let Some((mut next_end, mut first)) = context::next_word_on_line(haystack, span.end) else {
        return false;
    };
    if ConstraintResolver::is_quantity_modifier(&first) {
        let Some(next) = context::next_word_on_line(haystack, next_end) else {
            return false;
        };
        (next_end, first) = next;
    }
    let second = context::next_word_on_line(haystack, next_end);
    let complete_aoeo = span == candidate.consumed
        && branch.origins.iter().all(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.aoeo")
        });
    (resolver.has_postnominal_object_count(&first, second.as_ref().map(|(_, word)| word.as_ref()))
        || second.is_some_and(|(end, second)| {
            ConstraintResolver::split_counter_quantity(&first, &second).is_some_and(|count| {
                context::next_word_on_line(haystack, end).is_some_and(|(_, unit)| {
                    resolver.has_postnominal_object_count(count, Some(&unit))
                })
            })
        }))
        && context::normalized_token(haystack, span).is_some_and(|token| {
            resolver.has_exact_noun_with_optional_particles(&token)
                && !resolver.has_predicate_adnominal_surface(&token)
        })
        && !context::previous_word(haystack, candidate.consumed.start).is_some_and(|word| {
            resolver.has_object_marked_nominal(&word)
                || (complete_aoeo && resolver.has_nominal_oblique(&word))
        })
}
