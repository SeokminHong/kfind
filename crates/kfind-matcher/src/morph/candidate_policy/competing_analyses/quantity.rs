use kfind_morph::{CoarsePos, ConstraintResolver, PredicatePos};
use kfind_query::{AtomPlan, CandidateConsumption, CandidateProgram};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{
    is_boundary_after, is_boundary_before, is_sentence_end_after, next_character_with_end,
    previous_character, surrounding_token_span,
};

use super::{ExecutedCandidate, context, has_counted_unit_copula_ending};

const COMPARATIVE_ADVERBS: &[(&str, &str)] =
    include!("../../../../../../data/rules/krdict-comparative-adverbs.rs");

pub(super) fn has_object_before_han_and_final_copula(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    matches!(haystack.get(core.clone()), Some(surface)
        if surface == "한".as_bytes() || surface == "한".as_bytes())
        && *core == candidate.consumed
        && !branch.origins.is_empty()
        && branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == CoarsePos::Determiner)
        })
        && is_boundary_before(haystack, core.start)
        && is_boundary_after(haystack, core.end)
        && context::previous_word(haystack, core.start)
            .is_some_and(|object| resolver.has_object_marked_nominal(&object))
        && context::next_word_with_end(haystack, core.end).is_some_and(|(end, predicate)| {
            is_sentence_end_after(haystack, end)
                && resolver.has_final_copula_after_nominal(&predicate)
                && !resolver.has_exact_nominal_surface(&predicate)
        })
}

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

pub(super) fn has_spaced_dictionary_numeric_unit(
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
    let span = &candidate.consumed;
    if pos == PredicatePos::Copula
        || !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
    {
        return false;
    }
    context::previous_word_with_start(haystack, span.start).is_some_and(|(start, previous)| {
        previous.bytes().all(|byte| byte.is_ascii_digit())
            && !haystack[start + previous.len()..span.start]
                .iter()
                .any(|byte| matches!(byte, b'\r' | b'\n'))
            && context::normalized_token(haystack, span.clone())
                .is_some_and(|token| resolver.has_complete_dictionary_numeric_unit_path(&token))
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

pub(super) fn has_comparative_adverb_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let Some(&(lemma, _)) = COMPARATIVE_ADVERBS.iter().find(|(nfc, nfd)| {
        branch.anchor.as_ref() == nfc.as_bytes() || branch.anchor.as_ref() == nfd.as_bytes()
    }) else {
        return false;
    };
    let core = &candidate.verified.core;
    if !matches!(
        branch.consumption,
        CandidateConsumption::NominalParticleChain { .. }
    ) || *core != candidate.consumed
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            let analysis = &atom.analyses[usize::from(origin.analysis_index)];
            analysis.coarse_pos == CoarsePos::Noun && analysis.lemma.as_ref() == lemma
        })
        || !is_boundary_before(haystack, core.start)
        || !is_boundary_after(haystack, core.end)
    {
        return false;
    }
    (has_marked_comparison_participants(haystack, core.start, resolver)
        && context::next_word_on_line(haystack, core.end).is_some_and(|(_, next)| {
            resolver.has_exact_predicate_ending_path(&next, PredicatePos::Adjective)
                || resolver.has_final_adjective_by_ending(&next)
                || resolver.has_adjective_adnominal_surface(&next)
        }))
        || (context::previous_non_adverb_on_line(haystack, core.start, resolver)
            .is_some_and(|word| resolver.has_object_marked_nominal(&word))
            && context::next_word_on_line(haystack, core.end).is_some_and(|(_, next)| {
                resolver.has_independent_final_verb_with_object(&next)
                    || resolver.has_accusative_verb_connective(&next)
            }))
}

pub(super) fn has_concessive_particle_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if !(branch.consumption.composes_copula_host()
        || matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation {
                pos: PredicatePos::Copula,
                ..
            } | CandidateConsumption::StructuralPredicateEnding {
                pos: PredicatePos::Copula,
                ..
            }
        ))
    {
        return false;
    }
    let span = surrounding_token_span(haystack, candidate.verified.core.clone());
    if candidate.consumed.end != span.end {
        return false;
    }
    let Some(token) = context::normalized_token(haystack, span.clone()) else {
        return false;
    };
    let Some(host) = token
        .strip_suffix("이라도")
        .or_else(|| token.strip_suffix("라도"))
    else {
        return false;
    };
    if !resolver.has_complete_nominal_surface(host)
        || context::has_preceding_subject_or_topic(haystack, span.start, resolver)
    {
        return false;
    }
    let Some((end, count)) = context::next_word_on_line(haystack, span.end) else {
        return false;
    };
    if is_sentence_end_after(haystack, end)
        && ConstraintResolver::has_propositive_do_inflection(&count)
        && resolver.is_attested_do_object(host)
    {
        return true;
    }
    let Some((end, unit)) = context::next_word_on_line(haystack, end) else {
        return false;
    };
    resolver.has_postnominal_object_count(&count, Some(&unit))
        && context::next_word_on_line(haystack, end).is_some_and(|(end, verb)| {
            is_sentence_end_after(haystack, end) && resolver.has_final_verb_after_object(&verb)
        })
}

pub(super) fn has_quoted_name_instead_of_degree(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let Some(&(word, _)) = COMPARATIVE_ADVERBS.iter().find(|(nfc, nfd)| {
        branch.anchor.as_ref() == nfc.as_bytes() || branch.anchor.as_ref() == nfd.as_bytes()
    }) else {
        return false;
    };
    let core = &candidate.verified.core;
    if *core != candidate.consumed
        || !is_boundary_before(haystack, core.start)
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses[usize::from(origin.analysis_index)].coarse_pos == CoarsePos::Adverb
        })
    {
        return false;
    }
    let Some((end, name)) = context::next_word_on_line(haystack, core.end) else {
        return false;
    };
    if !resolver.has_exact_nominal_surface(word)
        || !resolver.has_unambiguous_proper_name_surface(&name)
    {
        return false;
    }
    let Some((_, close)) = next_character_with_end(haystack, end) else {
        return false;
    };
    let open = match close {
        '>' => '<',
        '〉' => '〈',
        '》' => '《',
        '’' => '‘',
        '”' => '“',
        '\'' | '"' => close,
        _ => return false,
    };
    let limit = core
        .start
        .saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    let mut before = core.start;
    while before > limit {
        let Some(character) = previous_character(haystack, before) else {
            break;
        };
        if character == open {
            return true;
        }
        if character == close || matches!(character, '\r' | '\n' | '.' | '!' | '?') {
            break;
        }
        before -= character.len_utf8();
    }
    false
}

fn has_marked_comparison_participants(
    haystack: &[u8],
    end: usize,
    resolver: &ConstraintResolver,
) -> bool {
    let limit = end.saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    let mut before = end;
    let mut subject = false;
    let mut comparison = false;
    while let Some((start, word)) = context::previous_word_with_start(haystack, before) {
        if start < limit
            || haystack[start..before]
                .iter()
                .any(|byte| matches!(byte, b'\r' | b'\n'))
        {
            return false;
        }
        if !comparison
            && word
                .strip_suffix("보다")
                .or_else(|| word.strip_suffix("보다는"))
                .is_some_and(|nominal| resolver.has_complete_nominal_surface(nominal))
        {
            comparison = true;
        } else if !subject && resolver.has_subject_marked_nominal(&word) {
            subject = true;
        } else if !(resolver.has_quantity_expression(&word, None)
            || ((resolver.has_exact_adverb_surface(&word)
                || ((subject || comparison) && resolver.has_predicate_adnominal_surface(&word)))
                && !resolver.has_exact_nominal_surface(&word)))
        {
            return false;
        }
        if subject && comparison {
            return true;
        }
        before = start;
    }
    false
}
