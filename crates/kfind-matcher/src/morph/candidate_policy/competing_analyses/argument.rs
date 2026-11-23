use kfind_morph::{CoarsePos, ConstraintResolver};
use kfind_query::{AtomPlan, CandidateConsumption, CandidateProgram};

use crate::boundary::{
    bounded_surrounding_token_span, is_sentence_end_after, surrounding_token_span,
};

use super::{ExecutedCandidate, MAX_CONSUMPTION_BYTES, context};

pub(super) fn has_counted_circuit_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    // 돌다 takes an accusative route and a lap count (KRDict 75006).
    // 돌아서다 has no accusative frame (Standard Korean Dictionary 417035).
    let core = &candidate.verified.core;
    if (!haystack[core.start..].starts_with("돌아서".as_bytes())
        && !haystack[core.start..].starts_with("돌아서".as_bytes()))
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses[usize::from(origin.analysis_index)]
                .lemma
                .as_ref()
                == "돌아서다"
        })
    {
        return false;
    }
    let whole = surrounding_token_span(haystack, core.clone());
    if core.start != whole.start
        || !context::normalized_token(haystack, whole.clone()).is_some_and(|word| {
            word.strip_prefix("돌").is_some_and(|ending| {
                ending.starts_with("아서") && resolver.has_connective_after_predicate("돌", ending)
            })
        })
    {
        return false;
    }
    let Some((unit_start, unit)) =
        context::previous_non_adverb_with_start_on_line(haystack, whole.start, resolver)
    else {
        return false;
    };
    context::previous_word_with_start(haystack, unit_start).is_some_and(|(start, count)| {
        // 바퀴 is the dictionary's lap unit. Keep it distinct from a
        // general occurrence count, which can also modify 돌아서다.
        matches!(unit.as_ref(), "바퀴" | "바퀴씩")
            && resolver.has_standalone_counted_unit(&count, "바퀴")
            && context::previous_word_with_start(haystack, start).is_some_and(
                |(route_start, route)| {
                    !haystack[route_start..unit_start]
                        .iter()
                        .any(|byte| matches!(byte, b'\r' | b'\n'))
                        && resolver.has_object_marked_nominal(&route)
                },
            )
    })
}

pub(super) fn has_independent_entry_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    if !haystack[core.start..].starts_with("들어".as_bytes())
        && !haystack[core.start..].starts_with("들어".as_bytes())
    {
        return false;
    }
    // 듣다 (KRDict 74104) does not take a spatial destination. An object
    // supports its independent reading even when a -로 instrument is present.
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses[usize::from(origin.analysis_index)]
                .lemma
                .as_ref()
                == "듣다"
        })
    {
        return false;
    }
    let Ok(whole) = bounded_surrounding_token_span(haystack, core.clone(), MAX_CONSUMPTION_BYTES)
    else {
        return false;
    };
    if whole.start != core.start {
        return false;
    }
    let Some((destination_start, destination)) =
        context::previous_non_adverb_with_start_on_line(haystack, whole.start, resolver)
    else {
        return false;
    };
    if !destination.ends_with(['에', '로']) {
        return false;
    }
    let Some(predicate) = context::normalized_token(haystack, whole.clone()) else {
        return false;
    };
    let previous = || context::previous_non_adverb_on_line(haystack, destination_start, resolver);
    if resolver.has_spatial_destination(&destination)
        && ConstraintResolver::has_enter_inflection(&predicate)
        && previous().is_none_or(|word| {
            resolver.has_subject_marked_nominal(&word) || resolver.has_topic_marked_nominal(&word)
        })
    {
        return true;
    }
    // 듣다 + continuative 오다 cannot supply the completed change
    // denoted by -어 있다, even when the -에 nominal lacks a spatial role.
    destination.ends_with('에')
        && resolver.has_nominal_oblique(&destination)
        && resolver.has_entry_result_state(
            &predicate,
            context::next_word_on_line(haystack, whole.end)
                .as_ref()
                .map(|(_, word)| word.as_ref()),
        )
        && previous().is_none_or(|word| !resolver.has_object_marked_nominal(&word))
}

pub(super) fn has_clause_final_go_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if !matches!(
        branch.consumption,
        CandidateConsumption::NominalParticleChain { .. }
    ) || *span != candidate.verified.core
        || *span != candidate.verified.token
        || !matches!(haystack.get(span.clone()), Some(word)
            if word == "가".as_bytes() || word == "가".as_bytes())
        || !is_sentence_end_after(haystack, span.end)
    {
        return false;
    }
    // 가 is a complete inflection of 가다, which takes a destination (KRDict 27500).
    let Some((adverb_start, adverb)) = context::previous_word_on_line(haystack, span.start) else {
        return false;
    };
    if !resolver.has_exact_adverb_surface(&adverb)
        || resolver.has_exact_noun_with_optional_particles(&adverb)
    {
        return false;
    }
    context::previous_word_on_line(haystack, adverb_start).is_some_and(|(start, destination)| {
        resolver.has_spatial_destination(&destination)
            || (resolver.has_exact_proper_name_surface(&destination)
                && context::previous_word_on_line(haystack, start).is_some_and(|(_, means)| {
                    means.ends_with('로') && resolver.has_nominal_oblique(&means)
                }))
    })
}

pub(super) fn has_late_schedule_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    if (!haystack[core.start..].starts_with("늦".as_bytes())
        && !haystack[core.start..].starts_with("늦".as_bytes()))
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            let analysis = &atom.analyses[usize::from(origin.analysis_index)];
            analysis.coarse_pos == CoarsePos::Adjective && analysis.lemma.as_ref() == "늦다"
        })
    {
        return false;
    }
    let whole = surrounding_token_span(haystack, core.clone());
    if core.start != whole.start
        || !context::normalized_token(haystack, whole.clone())
            .is_some_and(|word| ConstraintResolver::has_late_arrival_inflection(&word))
    {
        return false;
    }
    let Some((mut start, complement)) =
        context::previous_non_adverb_with_start_on_line(haystack, whole.start, resolver)
    else {
        return false;
    };
    let Some(host) = complement.strip_suffix('에') else {
        return false;
    };
    let scheduled = if host == "시간" {
        context::previous_word_with_start(haystack, start).is_some_and(|(offset, modifier)| {
            start = offset;
            resolver.is_scheduled_nominal(&modifier)
        })
    } else {
        resolver.is_scheduled_nominal(host)
    };
    scheduled
        && resolver.has_nominal_oblique(&complement)
        && context::previous_non_adverb_on_line(haystack, start, resolver).is_none_or(|word| {
            if !(resolver.has_subject_marked_nominal(&word)
                || resolver.has_topic_marked_nominal(&word))
            {
                return true;
            }
            resolver.has_subject_or_topic_marked_proper_name(&word)
                || word
                    .strip_suffix(['이', '가', '은', '는'])
                    .is_some_and(|noun| {
                        resolver.is_person_nominal(noun) || resolver.has_exact_pronoun_surface(noun)
                    })
        })
}

pub(super) fn has_directive_with_origin_and_direction(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if !branch.consumption.composes_copula_host() {
        return false;
    }
    let span = &candidate.consumed;
    if !context::normalized_token(haystack, span.clone()).is_some_and(|word| {
        word.strip_suffix("라면")
            .is_some_and(|stem| resolver.has_exact_action_predicate_surface(stem))
    }) {
        return false;
    }
    let Some((start, direction)) =
        context::previous_non_adverb_with_start_on_line(haystack, span.start, resolver)
    else {
        return false;
    };
    direction.ends_with('로')
        && resolver.has_nominal_oblique(&direction)
        && context::previous_word(haystack, start).is_some_and(|origin| {
            (origin.ends_with("에서") && resolver.has_nominal_oblique(&origin))
                || origin
                    .strip_suffix('서')
                    .is_some_and(context::is_locative_pronoun)
        })
}

pub(super) fn has_volitional_clause(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if !haystack[span.clone()].ends_with("세".as_bytes())
        && !haystack[span.clone()].ends_with("세".as_bytes())
    {
        return false;
    }
    if candidate.verified.core != *span
        || !is_sentence_end_after(haystack, span.end)
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses[usize::from(origin.analysis_index)].coarse_pos == CoarsePos::Noun
        })
        || !context::normalized_token(haystack, span.clone())
            .is_some_and(|word| resolver.has_volitional_verb_surface(&word))
    {
        return false;
    }
    let is_speaker = |word: &str| {
        word.strip_suffix(['가', '는', '도'])
            .is_some_and(|pronoun| matches!(pronoun, "나" | "내" | "저" | "제" | "우리" | "저희"))
    };
    let is_locative = |word: &str| {
        word.strip_suffix("에는")
            .is_some_and(|noun| resolver.is_spatial_nominal(noun))
            || word.ends_with("에서") && resolver.has_nominal_oblique(word)
            || word
                .strip_suffix('서')
                .is_some_and(context::is_locative_pronoun)
    };
    let mut before = span.start;
    let mut locative = false;
    let limit = span
        .start
        .saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    let start = loop {
        let Some((start, previous)) = context::previous_word_with_start(haystack, before) else {
            return false;
        };
        if start < limit
            || haystack[start..before]
                .iter()
                .any(|byte| matches!(byte, b'\r' | b'\n'))
        {
            return false;
        }
        if is_speaker(&previous) {
            break start;
        }
        if is_locative(&previous) {
            locative = true;
        } else if !resolver.has_exact_adverb_surface(&previous) {
            return false;
        }
        before = start;
    };
    if locative {
        return true;
    }
    if context::previous_non_adverb_on_line(haystack, start, resolver)
        .is_some_and(|word| is_locative(&word))
    {
        return true;
    }
    let limit = start.saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    haystack[limit..start]
        .iter()
        .rposition(|byte| matches!(byte, b'.' | b'!' | b'?'))
        .is_some_and(|offset| {
            !haystack[limit + offset..start]
                .iter()
                .any(|byte| matches!(byte, b'\r' | b'\n'))
                && context::previous_word(haystack, limit + offset).is_some_and(|word| {
                    word.strip_suffix('게').is_some_and(|stem| {
                        resolver.has_exact_action_predicate_surface(stem)
                            || resolver.has_independent_final_predicate(&word)
                    })
                })
        })
}
