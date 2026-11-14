use std::ops::Range;

use kfind_data::DataFinePos;
use kfind_morph::{ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, RuleId};
use kfind_query::{AtomPlan, CandidateConsumption, CandidateProgram};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{
    is_boundary_after, is_boundary_before, is_sentence_end_after, surrounding_token_span,
};

use super::super::{
    ADNOMINAL_RULE_IDS, ExecutedCandidate, context, follows_predicate_adnominal,
    has_counted_unit_copula_ending, has_jiyo_suffix, has_quotative_adnominal_head,
};

mod nominal;
mod quantity;

pub(super) fn rejects(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    (branch.origins.iter().any(|origin| {
        origin
            .rule_path
            .iter()
            .any(|rule| rule.as_str() == "contraction.ha-gi")
    }) && context::normalized_token(haystack, candidate.anchor.clone())
        .is_some_and(|anchor| resolver.has_exact_nominal_surface(&anchor)))
        || (candidate
            .suffix_rules
            .iter()
            .any(|rule| rule.as_str() == "particle.comitative.hago")
            && has_hago_predicate_frame(haystack, &candidate.verified.token, resolver))
        || has_competing_iran_particle_analysis(haystack, candidate, resolver)
        || lacks_contracted_quotative_head(haystack, candidate, branch, resolver)
        || has_competing_hao_connective(haystack, candidate, branch, resolver)
        || nominal::has_competing_nominal_with_gu(haystack, candidate, resolver)
        || has_nominal_quotative_frame(haystack, candidate, branch, resolver)
        || quantity::has_counted_unit_copula(haystack, candidate, branch, resolver)
        || quantity::has_elapsed_measure_frame(haystack, candidate, branch, atom, resolver)
        || quantity::has_quantifying_adverb_frame(haystack, candidate, branch, atom, resolver)
        || has_jiyo_object_frame(haystack, candidate, branch, atom, resolver)
        || has_whole_alternative_conjunction(haystack, candidate, branch, resolver)
        || nominal::has_determiner_frame_for_alternative(haystack, candidate, resolver)
        || nominal::has_nama_proper_name_frame(haystack, candidate, branch, resolver)
        || has_conflicting_honorific(haystack, candidate, branch, atom, resolver)
        || has_object_inside_adnominal_result(haystack, candidate, branch, resolver)
        || nominal::has_nominal_frame_for_volitional(haystack, candidate, branch, resolver)
        || has_humble_object_frame(haystack, candidate, branch, atom, resolver)
        || has_topic_before_independent_predicate(haystack, candidate, branch, resolver)
        || quantity::has_postnominal_count_frame(haystack, candidate, branch, resolver)
        || nominal::has_nominal_interrogative_frame(haystack, candidate, branch, resolver)
        || nominal::has_nonpredicate_report_frame(haystack, candidate, branch, resolver)
        || nominal::has_comparative_nominal_frame(haystack, candidate, branch, resolver)
        || has_nominal_quotative_rera(haystack, candidate, resolver)
        || has_competing_wa_analysis(haystack, candidate, branch, resolver)
}

fn has_competing_wa_analysis(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if !candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.humble-wa")
        && !branch.origins.iter().any(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.humble-wa")
        })
    {
        return false;
    }
    if !is_boundary_after(haystack, candidate.consumed.end) {
        return true;
    }
    let whole = surrounding_token_span(haystack, candidate.consumed.clone());
    let inside_token = candidate.consumed.start > whole.start;
    let start = whole.start;
    context::normalized_token(haystack, whole).is_some_and(|token| {
        token.strip_suffix('와').is_some_and(|host| {
            let derived_stem = !inside_token
                && host
                    .strip_suffix('하')
                    .is_some_and(|root| resolver.has_complete_nominal_surface(root));
            if derived_stem {
                return false;
            }
            (resolver.has_complete_nominal_surface(host)
                && !context::previous_word(haystack, start)
                    .is_some_and(|previous| resolver.has_object_marked_nominal(&previous)))
                || ((inside_token || resolver.has_vowel_connective_host(&token))
                    && !resolver.has_exact_non_copula_predicate_surface(host))
        })
    })
}

fn has_nominal_quotative_rera(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    if !candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.conjecture-rieulrera")
    {
        return false;
    }
    context::normalized_token(haystack, candidate.consumed.clone()).is_some_and(|token| {
        token.strip_suffix('라').is_some_and(|host| {
            resolver.has_exact_nominal_surface(host)
                && context::next_word(haystack, candidate.consumed.end)
                    .is_some_and(|next| resolver.has_predicate_adnominal_surface(&next))
        })
    })
}

fn lacks_contracted_quotative_head(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: kfind_morph::PredicatePos::Copula,
            ..
        }
    ) && candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.copula-quotative-adnominal")
        && std::str::from_utf8(&haystack[candidate.anchor.end..candidate.consumed.end])
            .is_ok_and(|suffix| suffix.nfc().eq("란".chars()))
        && !has_quotative_adnominal_head(haystack, candidate.consumed.end, resolver)
}

fn has_competing_iran_particle_analysis(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    if !candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "particle.topic.iran-ran")
    {
        return false;
    }
    let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
    let before = &haystack[..whole.start];
    if whole.start == candidate.verified.core.start
        && (before.ends_with(b"\"") || before.ends_with("”".as_bytes()))
    {
        return true;
    }
    context::normalized_token(haystack, whole.clone()).is_some_and(|token| {
        (resolver.has_exact_nominal_surface(&token)
            && context::next_word(haystack, whole.end)
                .is_some_and(|next| resolver.has_nominal_genitive_surface(&next)))
            || (!token.ends_with("이란")
                && resolver.has_predicate_adnominal_surface(&token)
                && has_quotative_adnominal_head(haystack, whole.end, resolver)
                && context::previous_word(haystack, whole.start).is_some_and(|previous| {
                    resolver.has_nominal_oblique(&previous)
                        || resolver.has_object_marked_nominal(&previous)
                }))
    })
}

fn has_jiyo_object_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    let tail = &haystack[core.end..];
    if !(tail.starts_with("지요".as_bytes())
        || tail.starts_with("죠".as_bytes())
        || tail.starts_with("ᄌ".as_bytes()))
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == kfind_morph::CoarsePos::Adjective)
        })
    {
        return false;
    }
    let span = surrounding_token_span(haystack, core.clone());
    if core.start != span.start
        || candidate.consumed.end > span.end
        || !is_sentence_end_after(haystack, span.end)
        || !has_jiyo_suffix(haystack, core.end..span.end)
        || !context::normalized_token(haystack, core.clone())
            .is_some_and(|stem| resolver.has_exact_action_predicate_surface(&stem))
    {
        return false;
    }
    let limit = span
        .start
        .saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    let mut before = span.start;
    while let Some((start, word)) = context::previous_word_with_start(haystack, before) {
        if start < limit || haystack[start..before].contains(&b'\n') {
            break;
        }
        if resolver.has_object_marked_nominal(&word) {
            return true;
        }
        if !resolver.has_exact_adverb_surface(&word) {
            break;
        }
        before = start;
    }
    false
}

fn has_topic_before_independent_predicate(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    let end = match candidate.suffix_rules.as_slice() {
        [prefix @ .., last]
            if last.as_str() == "ending.present-adnominal"
                && prefix
                    .iter()
                    .any(|rule| rule.as_str() == "ending.auxiliary-jida") =>
        {
            span.end
        }
        [] if branch.origins.iter().all(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.connective-ji")
        }) =>
        {
            let token = surrounding_token_span(haystack, span.clone());
            if !context::normalized_token(haystack, span.end..token.end)
                .is_some_and(|suffix| suffix == "는")
            {
                return false;
            }
            token.end
        }
        _ => return false,
    };
    is_boundary_before(haystack, span.start)
        && is_boundary_after(haystack, end)
        && context::normalized_token(haystack, span.start..end).is_some_and(|token| {
            token
                .strip_suffix('는')
                .is_some_and(|host| resolver.has_exact_nominal_surface(host))
        })
        && context::next_word_with_end(haystack, end).is_some_and(|(next_end, next)| {
            resolver.has_independent_final_predicate(&next)
                || (context::previous_word(haystack, span.start).is_none()
                    && resolver.has_exact_nominal_surface(&next)
                    && !resolver.has_exact_non_copula_predicate_surface(&next)
                    && context::next_word_with_end(haystack, next_end).is_some_and(
                        |(predicate_end, predicate)| {
                            is_sentence_end_after(haystack, predicate_end)
                                && (resolver.has_final_copula_after_nominal(&predicate)
                                    || predicate.strip_suffix('다').is_some_and(|noun| {
                                        noun.chars().next_back().is_some_and(|last| {
                                            kfind_morph::decompose_syllable(last)
                                                .is_some_and(|syllable| syllable.jongseong == 0)
                                        }) && resolver.has_exact_nominal_surface(noun)
                                    }))
                        },
                    ))
        })
}

fn has_whole_alternative_conjunction(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if !matches!(candidate.suffix_rules.as_slice(), [rule] if rule.as_str() == "particle.topic")
        || !branch
            .structural_patterns()
            .iter()
            .any(|pattern| matches!(pattern.fine_pos, DataFinePos::Mag | DataFinePos::Maj))
    {
        return false;
    }
    let span = &candidate.consumed;
    is_boundary_before(haystack, span.start)
        && is_boundary_after(haystack, span.end)
        && context::normalized_token(haystack, span.clone())
            .is_some_and(|token| resolver.has_exact_conjunctive_adverb(&token))
        && context::previous_word(haystack, span.start)
            .is_some_and(|previous| resolver.has_alternative_connective(&previous))
}

fn has_object_inside_adnominal_result(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    use kfind_morph::PredicatePos;
    if !matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: PredicatePos::Adjective,
            ..
        } | CandidateConsumption::StructuralPredicateEnding {
            pos: PredicatePos::Adjective,
            ..
        }
    ) || !branch.origins.iter().any(|origin| {
        candidate
            .suffix_rules
            .last()
            .or_else(|| origin.rule_path.last())
            .is_some_and(|rule| ADNOMINAL_RULE_IDS.contains(&rule.as_str()))
    }) {
        return false;
    }
    let span = &candidate.consumed;
    let Some((end, next)) = context::next_word_with_end(haystack, span.end) else {
        return false;
    };
    resolver.has_adnominal_result_complement(&next)
        && is_boundary_before(haystack, span.start)
        && context::previous_word(haystack, span.start)
            .is_some_and(|previous| resolver.has_object_marked_nominal(&previous))
        && context::next_word_with_end(haystack, end).is_some_and(|(next_end, next)| {
            let predicate = if resolver.has_subject_marked_nominal(&next) {
                context::next_word_with_end(haystack, next_end)
            } else {
                Some((next_end, next))
            };
            predicate.is_some_and(|(predicate_end, predicate)| {
                is_sentence_end_after(haystack, predicate_end)
                    && resolver.has_independent_final_predicate_without_object(&predicate)
            })
        })
}

fn has_conflicting_honorific(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            let mut rules = origin.rule_path.iter().map(RuleId::as_str);
            let honorific = rules.any(|rule| rule == "ending.honorific");
            honorific
                && rules
                    .chain(candidate.suffix_rules.iter().map(RuleId::as_str))
                    .any(|rule| {
                        matches!(
                            rule,
                            "ending.polite-imperative"
                                | "ending.final-ne"
                                | "ending.aoeo-seo"
                                | "ending.connective-do"
                        )
                    })
        })
    {
        return false;
    }
    let token = surrounding_token_span(haystack, candidate.verified.core.clone());
    let start = candidate.verified.core.start;
    if token.start < start {
        return context::normalized_token(haystack, token.clone()).is_some_and(|word| {
            resolver.has_exact_non_copula_predicate_surface(&word)
                && context::normalized_token(haystack, token.start..start)
                    .is_some_and(|prefix| !resolver.has_vowel_connective_host(&prefix))
        });
    }
    // Auxiliary 주다 takes an -아/-어 connective. Its honorific overlaps with 줄다.
    branch.origins.iter().all(|origin| {
        atom.analyses
            .get(usize::from(origin.analysis_index))
            .is_some_and(|analysis| analysis.lemma.as_ref() == "줄다")
    }) && context::previous_word(haystack, start)
        .is_some_and(|previous| resolver.has_request_auxiliary_host(&previous))
}

fn has_humble_object_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.lemma.as_ref() == "비다")
        })
    {
        return false;
    }
    let core = &candidate.verified.core;
    let whole = surrounding_token_span(haystack, core.clone());
    if whole.start != core.start
        || !is_sentence_end_after(haystack, whole.end)
        || !context::normalized_token(haystack, core.end..whole.end).is_some_and(|ending| {
            matches!(
                ending.as_ref(),
                "나이다" | "나이까" | "옵나이다" | "옵나이까"
            )
        })
    {
        return false;
    }
    context::previous_word_with_start(haystack, whole.start).is_some_and(|(start, previous)| {
        resolver.has_object_marked_nominal(&previous)
            && previous.strip_suffix(['을', '를']).is_some_and(|host| {
                let count = context::previous_word(haystack, start);
                !resolver.has_elapsed_measure(host, count.as_deref())
            })
    })
}

fn has_nominal_quotative_frame(
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
    if pos == kfind_morph::PredicatePos::Copula {
        return false;
    }
    let Some(suffix) = haystack
        .get(core.end..candidate.consumed.end)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    if !suffix.starts_with(['라', 'ᄅ'])
        || !["라고", "라고요", "라는"]
            .iter()
            .any(|ending| suffix.nfc().eq(ending.chars()))
        || !is_boundary_before(haystack, core.start)
    {
        return false;
    }
    haystack
        .get(core.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .is_some_and(|nominal| {
            resolver.has_exact_nominal_surface(&nominal.nfc().collect::<String>())
        })
        && follows_predicate_adnominal(haystack, core.start, resolver)
}

fn has_competing_hao_connective(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let CandidateConsumption::PredicateContinuation {
        continuation,
        pos,
        source_positions,
        ..
    } = branch.consumption
    else {
        return false;
    };
    let respectful = candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.hao-respectful-connective");
    let contracted_aoeo = continuation == kfind_morph::ContinuationState::AOrEo
        && candidate.consumed == candidate.verified.core
        && ["오", "ᄋ"]
            .iter()
            .any(|syllable| haystack[candidate.consumed.end..].starts_with(syllable.as_bytes()));
    if !respectful && !contracted_aoeo {
        return false;
    }
    let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
    if candidate.verified.core.start != whole.start {
        return false;
    }
    if !respectful
        && !haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|tail| tail.nfc().eq("오면".chars()) || tail.nfc().eq("오니".chars()))
    {
        return false;
    }
    let Some(token) = haystack
        .get(whole)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let normalized = token.nfc().collect::<String>();
    let core_len = token[..candidate.verified.core.len()]
        .nfc()
        .map(char::len_utf8)
        .sum();
    resolver.has_extended_predicate_prefix_path(
        &normalized,
        core_len,
        source_positions | kfind_morph::PredicatePosSet::one(pos),
        DEFAULT_LATTICE_NODE_LIMIT,
    )
}

fn has_hago_predicate_frame(
    haystack: &[u8],
    token: &Range<usize>,
    resolver: &ConstraintResolver,
) -> bool {
    let Some(surface) = context::normalized_token(haystack, token.clone()) else {
        return false;
    };
    let Some(stem) = surface
        .strip_suffix("고요")
        .or_else(|| surface.strip_suffix('고'))
    else {
        return false;
    };
    let previous = context::previous_word(haystack, token.start);
    let next = context::next_word(haystack, token.end);
    (previous.as_deref().is_some_and(|word| {
        resolver.has_object_marked_nominal(word) || resolver.has_negative_connective_host(word)
    }) || next.as_deref().is_some_and(|word| {
        word.starts_with('있') && resolver.has_exact_non_copula_predicate_surface("있")
    })) && resolver.has_exact_action_predicate_surface(stem)
        || (previous
            .as_deref()
            .is_some_and(|word| resolver.has_subject_marked_nominal(word))
            || next
                .as_deref()
                .is_some_and(|word| resolver.has_subject_marked_nominal(word)))
            && resolver.has_exact_adjective_surface(stem)
}
