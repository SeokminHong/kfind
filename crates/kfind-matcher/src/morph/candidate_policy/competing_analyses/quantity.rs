use std::borrow::Cow;

use kfind_morph::{CoarsePos, ConstraintResolver, PredicatePos, PredicatePosSet};
use kfind_query::{AtomPlan, CandidateConsumption, CandidateProgram};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{
    bounded_surrounding_token_span, is_boundary_after, is_boundary_before, is_sentence_end_after,
    next_character_with_end, previous_character, surrounding_token_span,
};

use super::{ExecutedCandidate, MAX_CONSUMPTION_BYTES, context, has_counted_unit_copula_ending};

const COMPARATIVE_ADVERBS: &[(&str, &str)] =
    include!("../../../../../../data/rules/krdict-comparative-adverbs.rs");

pub(super) fn has_quantity_nominal_conflict(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if !matches!(
        branch.consumption,
        CandidateConsumption::NominalParticleChain { .. }
    ) {
        return false;
    }
    let core = &candidate.verified.core;
    let starts_word = is_boundary_before(haystack, core.start);
    if starts_word && is_boundary_after(haystack, core.end) {
        return false;
    }
    let Some(host) = context::normalized_token(haystack, core.clone()) else {
        return false;
    };
    if !starts_word && !ConstraintResolver::is_quantity_approximation_suffix(&host) {
        return false;
    }
    let Ok(whole) = bounded_surrounding_token_span(haystack, core.clone(), MAX_CONSUMPTION_BYTES)
    else {
        return false;
    };
    let (unit_start, unit) = if !starts_word
        && context::normalized_token(haystack, core.end..whole.end)
            .is_some_and(|tail| resolver.has_quantity_approximation_particle_tail(&tail))
    {
        let Some(unit) = context::normalized_token(haystack, whole.start..core.start) else {
            return false;
        };
        (whole.start, unit)
    } else if starts_word
        && context::normalized_token(haystack, whole.clone())
            .is_some_and(|token| resolver.has_comparative_nominal_expansion(&token, &host))
    {
        let Some(preceding) = context::previous_word_on_line(haystack, whole.start) else {
            return false;
        };
        preceding
    } else {
        return false;
    };
    let count = context::previous_word_on_line(haystack, unit_start).map(|(_, count)| count);
    resolver.has_quantity_amount(&unit, count.as_deref())
        || (starts_word
            && resolver.has_whole_determiner(&unit)
            && (context::next_word_on_line(haystack, whole.end).is_some_and(|(_, next)| {
                let adjectives = PredicatePosSet::one(PredicatePos::Adjective);
                resolver.has_exact_predicate_connective_path(&next, adjectives)
                    || resolver.has_derived_predicate_connective_surface(&next, adjectives)
            }) || count.is_some_and(|argument| {
                resolver.has_object_marked_nominal(&argument)
                    || resolver.has_subject_marked_nominal(&argument)
                    || resolver.has_topic_marked_nominal(&argument)
            })))
}

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
    let Some((next_end, next)) = context::next_word_on_line(haystack, core.end) else {
        return false;
    };
    let mut measured_adverb_homograph = false;
    let previous = context::previous_matching_word_on_line(haystack, core.start, |start, word| {
        if !resolver.has_exact_adverb_surface(word) {
            return true;
        }
        measured_adverb_homograph =
            measured_subject_start(haystack, start, word, resolver).is_some();
        measured_adverb_homograph
    });
    let adnominal = resolver.has_adjective_adnominal_surface(&next);
    let preceding_subject = previous.as_ref().is_some_and(|(start, word)| {
        let nominal_rival = resolver.has_exact_nominal_surface(&next);
        (resolver.has_subject_marked_nominal(word) && !nominal_rival)
            || ((measured_adverb_homograph
                || measured_subject_start(haystack, *start, word, resolver).is_some())
                && (!nominal_rival
                    || (adnominal
                        && context::next_word_on_line(haystack, next_end).is_some_and(
                            |(_, head)| resolver.has_nominal_with_optional_particles(&head),
                        ))))
    });
    ((has_marked_comparison_participants(haystack, core.start, resolver) || preceding_subject)
        && (adnominal
            || resolver.has_exact_predicate_ending_path(&next, PredicatePos::Adjective)
            || resolver.has_final_adjective_by_ending(&next)
            || (preceding_subject && resolver.has_adjective_change_of_state(&next))))
        || (previous.is_some_and(|(_, word)| resolver.has_object_marked_nominal(&word))
            && (resolver.has_independent_final_verb_with_object(&next)
                || resolver.has_accusative_verb_connective(&next)))
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
        let mut preceding = start;
        if !comparison
            && word
                .strip_suffix("보다")
                .or_else(|| word.strip_suffix("보다는"))
                .is_some_and(|nominal| resolver.has_complete_nominal_surface(nominal))
        {
            comparison = true;
        } else if !subject
            && let Some(subject_start) = resolver
                .has_subject_marked_nominal(&word)
                .then_some(start)
                .or_else(|| measured_subject_start(haystack, start, &word, resolver))
        {
            subject = true;
            preceding = subject_start;
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
        before = preceding;
    }
    false
}

fn measured_subject_start(
    haystack: &[u8],
    start: usize,
    word: &str,
    resolver: &ConstraintResolver,
) -> Option<usize> {
    let host = word.strip_suffix(['이', '가'])?;
    let particle = &word[host.len()..];
    let (start, host) = if let Some(symbol) = previous_character(haystack, start).filter(|&ch| {
        (host.is_empty() && matches!(ch, '%' | '％' | '℃' | '\u{3300}'..='\u{33ff}'))
            || (host == "C" && ch == '°')
    }) {
        let symbol_start = start - symbol.len_utf8();
        let symbol_text = std::str::from_utf8(&haystack[symbol_start..start]).ok()?;
        if previous_character(haystack, symbol_start)
            .is_some_and(|ch| ch.is_ascii_alphabetic() || matches!(ch, 'µ' | 'μ'))
            && let Some((offset, prefix)) =
                context::previous_word_with_start(haystack, symbol_start)
        {
            (offset, Cow::Owned(format!("{prefix}{symbol_text}{host}")))
        } else {
            let unit = if host.is_empty() { symbol_text } else { "°C" };
            (symbol_start, Cow::Borrowed(unit))
        }
    } else if host.is_empty() {
        return None;
    } else {
        (start, Cow::Borrowed(host))
    };
    let digit_end = host.bytes().take_while(u8::is_ascii_digit).count();
    let unit = &host[digit_end..];
    let unit = if unit
        .chars()
        .any(|ch| matches!(ch, '％' | '℃' | '\u{3300}'..='\u{33ff}'))
    {
        Cow::Owned(unit.nfkc().collect::<String>())
    } else {
        Cow::Borrowed(unit)
    };
    if !resolver.has_nominative_numeric_unit(&unit, particle) {
        return None;
    }
    if digit_end != 0 {
        return number_start(haystack, start);
    }
    let (count_start, count) = context::previous_word_with_start(haystack, start)?;
    if !count.bytes().all(|byte| byte.is_ascii_digit())
        || haystack[count_start..start]
            .iter()
            .any(|byte| matches!(byte, b'\r' | b'\n'))
    {
        return None;
    }
    number_start(haystack, count_start)
}

fn number_start(haystack: &[u8], mut start: usize) -> Option<usize> {
    let limit = start.saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    while start > limit && matches!(haystack[start - 1], b'.' | b',') {
        let group_end = start - 1;
        let preceding_digits = haystack[limit..group_end]
            .iter()
            .rev()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if preceding_digits == 0 {
            break;
        }
        start = group_end - preceding_digits;
    }
    is_boundary_before(haystack, start).then_some(start)
}
