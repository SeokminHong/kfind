use std::{ops::Range, sync::OnceLock};

use kfind_data::DataFinePos;
use kfind_morph::{
    ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, LexicalAlternation, PredicateEntry,
    PredicateFlags, PredicatePos, RuleId, SurfaceBranchSpec, generate_predicate_branches,
    verify_predicate_continuation,
};
use kfind_query::{AtomPlan, CandidateConsumption, CandidateProgram, Morphology};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{
    bounded_surrounding_token_span, is_boundary_after, is_boundary_before, is_sentence_end_after,
    next_character_with_end, previous_character, surrounding_token_span,
};

use super::super::{
    ADNOMINAL_RULE_IDS, ExecutedCandidate, MAX_CONSUMPTION_BYTES, context,
    follows_predicate_adnominal, has_counted_unit_copula_ending, has_jiyo_suffix,
    has_quotative_adnominal_head,
};

mod argument;
mod nominal;
mod partitive;
mod quantity;
mod selection;

const DROP_RIEUL_OBJECT_CONTRASTS: &[(&str, &str)] =
    include!("../../../../../data/rules/krdict-drop-rieul-object-contrasts.rs");

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
        || (candidate
            .suffix_rules
            .last()
            .is_some_and(|rule| rule.as_str() == "ending.prospective-final")
            && !is_boundary_after(haystack, candidate.consumed.end))
        || lacks_contracted_quotative_head(haystack, candidate, branch, resolver)
        || has_topic_instead_of_propositive_adnominal(haystack, candidate, resolver)
        || has_adverb_instead_of_hao_final_u(haystack, candidate, resolver)
        || has_competing_hao_connective(haystack, candidate, branch, resolver)
        || has_purpose_motion_frame(haystack, candidate, branch, atom, resolver)
        || argument::has_independent_entry_frame(haystack, candidate, branch, atom, resolver)
        || argument::has_counted_circuit_frame(haystack, candidate, branch, atom, resolver)
        || argument::has_late_schedule_frame(haystack, candidate, branch, atom, resolver)
        || selection::has_incompatible_argument(haystack, candidate, branch, atom, resolver)
        || argument::has_directive_with_origin_and_direction(haystack, candidate, branch, resolver)
        || argument::has_volitional_clause(haystack, candidate, branch, atom, resolver)
        || nominal::has_competing_nominal_with_gu(haystack, candidate, resolver)
        || has_prohibitive_negation(haystack, candidate, branch, atom, resolver)
        || nominal::has_bound_nominal_prefix(haystack, candidate, branch, resolver)
        || nominal::has_adverb_instead_of_genitive(haystack, candidate, resolver)
        || nominal::has_degree_adverb_after_clause_argument(haystack, candidate, branch, resolver)
        || has_nominal_quotative_frame(haystack, candidate, branch, resolver)
        || quantity::has_counted_unit_copula(haystack, candidate, branch, resolver)
        || quantity::has_spaced_dictionary_numeric_unit(haystack, candidate, branch, resolver)
        || quantity::has_elapsed_measure_frame(haystack, candidate, branch, atom, resolver)
        || quantity::has_quantifying_adverb_frame(haystack, candidate, branch, atom, resolver)
        || quantity::has_quantity_nominal_conflict(haystack, candidate, branch, resolver)
        || quantity::has_comparative_adverb_frame(haystack, candidate, branch, atom, resolver)
        || quantity::has_quoted_name_instead_of_degree(haystack, candidate, branch, atom, resolver)
        || quantity::has_object_before_han_and_final_copula(
            haystack, candidate, branch, atom, resolver,
        )
        || partitive::rejects_han(haystack, candidate, branch, atom, resolver)
        || has_jiyo_object_frame(haystack, candidate, branch, atom, resolver)
        || has_whole_alternative_conjunction(haystack, candidate, branch, resolver)
        || nominal::has_nominal_time_frame_for_alternating(haystack, candidate, branch, resolver)
        || nominal::has_nama_proper_name_frame(haystack, candidate, branch, resolver)
        || has_conflicting_honorific(haystack, candidate, branch, atom, resolver)
        || has_humble_transitive_frame(haystack, candidate, branch, atom, resolver)
        || has_object_inside_adnominal_result(haystack, candidate, branch, resolver)
        || has_nominal_head_instead_of_resultative_adnominal(haystack, candidate, resolver)
        || nominal::has_nominal_frame_for_volitional(haystack, candidate, branch, resolver)
        || overlaps_competing_ending(haystack, candidate, branch, resolver)
        || has_pronoun_topic_instead_of_adnominal(haystack, candidate, branch, atom, resolver)
        || has_pronoun_object_instead_of_adnominal(haystack, candidate, branch, resolver)
        || has_topic_before_independent_predicate(haystack, candidate, branch, resolver)
        || has_adverb_instead_of_inferred_nominalization(haystack, candidate, branch, resolver)
        || quantity::has_postnominal_count_frame(haystack, candidate, branch, resolver)
        || quantity::has_concessive_particle_frame(haystack, candidate, branch, resolver)
        || nominal::has_nominal_interrogative_frame(haystack, candidate, branch, resolver)
        || nominal::has_nonpredicate_report_frame(haystack, candidate, branch, resolver)
        || nominal::has_comparative_nominal_frame(haystack, candidate, branch, resolver)
        || has_nominal_quotative_rera(haystack, candidate, resolver)
        || has_competing_wa_analysis(haystack, candidate, branch, resolver)
        || has_attached_result_state_auxiliary(haystack, candidate, branch, atom, resolver)
        || has_derived_passive_instead_of_independent_verb(
            haystack, candidate, branch, atom, resolver,
        )
        || has_competing_derived_predicate(haystack, candidate, branch, resolver)
}

fn has_derived_passive_instead_of_independent_verb(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    if is_boundary_before(haystack, core.start)
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses[usize::from(origin.analysis_index)]
                .lemma
                .as_ref()
                == "되다"
        })
    {
        return false;
    }
    let Ok(whole) =
        bounded_surrounding_token_span(haystack, candidate.consumed.clone(), MAX_CONSUMPTION_BYTES)
    else {
        return false;
    };
    let end = candidate.consumed.end;
    let Some(prefix) = context::normalized_token(haystack, whole.start..core.start) else {
        return false;
    };
    let Some(stem) = context::normalized_token(haystack, core.clone()) else {
        return false;
    };
    let anchor = prefix.len()..prefix.len() + stem.len();
    context::normalized_token(haystack, whole.clone())
        .is_some_and(|word| resolver.has_conflicting_passive_suffix(&word, anchor.clone()))
        || (end < whole.end
            && context::normalized_token(haystack, whole.start..end).is_some_and(|word| {
                resolver.has_conflicting_passive_suffix(&word, anchor)
                    && resolver.has_complete_connective_path(&word)
            })
            && context::normalized_token(haystack, end..whole.end)
                .is_some_and(|next| resolver.has_topic_marked_nominal(&next)))
}

fn has_attached_result_state_auxiliary(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let CandidateConsumption::PredicateContinuation {
        source_positions, ..
    } = branch.consumption
    else {
        return false;
    };
    let span = &candidate.consumed;
    if source_positions != kfind_morph::PredicatePosSet::one(PredicatePos::Adjective)
        || is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses[usize::from(origin.analysis_index)]
                .lemma
                .as_ref()
                == "있다"
        })
    {
        return false;
    }
    bounded_surrounding_token_span(haystack, span.clone(), MAX_CONSUMPTION_BYTES).is_ok_and(
        |whole| {
            context::normalized_token(haystack, whole.start..span.start).is_some_and(|host| {
                context::normalized_token(haystack, span.clone()).is_some_and(|auxiliary| {
                    resolver.has_result_state_auxiliary_frame(&host, &auxiliary)
                })
            })
        },
    )
}

fn has_purpose_motion_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            // KRDict 갈다 (59937, 62334) takes a replaced or ground object.
            atom.analyses[usize::from(origin.analysis_index)]
                .lemma
                .as_ref()
                == "갈다"
        })
    {
        return false;
    }
    let span = &candidate.consumed;
    if !is_boundary_before(haystack, span.start)
        || !context::previous_non_adverb_on_line(haystack, span.start, resolver)
            .is_some_and(|word| resolver.has_purpose_connective_host(&word))
    {
        return false;
    }
    bounded_surrounding_token_span(haystack, span.clone(), MAX_CONSUMPTION_BYTES).is_ok_and(
        |word| {
            context::normalized_token(haystack, word)
                .is_some_and(|text| ConstraintResolver::has_motion_go_inflection(&text))
        },
    )
}

fn has_prohibitive_negation(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.anchor.as_ref() != "안".as_bytes() && branch.anchor.as_ref() != "안".as_bytes()
    {
        return false;
    }
    let span = &candidate.consumed;
    let competes = match branch.consumption {
        CandidateConsumption::NominalParticleChain { .. } => {
            *span == candidate.verified.core
                && branch.origins.iter().all(|origin| {
                    atom.analyses[usize::from(origin.analysis_index)].coarse_pos
                        == kfind_morph::CoarsePos::Noun
                })
        }
        CandidateConsumption::PredicateContinuation { .. } => {
            *span == candidate.anchor
                && is_boundary_after(haystack, span.end)
                && !branch.origins.is_empty()
                && branch.origins.iter().all(|origin| {
                    origin
                        .rule_path
                        .last()
                        .is_some_and(|rule| rule.as_str() == "ending.past-adnominal")
                })
        }
        _ => false,
    };
    competes
        && is_boundary_before(haystack, span.start)
        && context::previous_word(haystack, span.start).is_some_and(|previous| {
            previous.ends_with("서는")
                && context::next_word_on_line(haystack, span.end).is_some_and(|(_, next)| {
                    resolver.has_prohibitive_negation_frame(&previous, &next)
                })
        })
}

fn has_pronoun_object_instead_of_adnominal(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    !branch.origins.is_empty()
        && branch.origins.iter().all(|origin| {
            candidate
                .suffix_rules
                .last()
                .or_else(|| origin.rule_path.last())
                .is_some_and(|rule| rule.as_str() == "ending.future-adnominal")
        })
        && context::accepts_pronoun_object(haystack, &candidate.consumed, resolver)
}

fn has_adverb_instead_of_inferred_nominalization(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    let is_nominalizer = |rule: &RuleId| rule.as_str() == "ending.nominalizer";
    if !(candidate.suffix_rules.last().is_some_and(is_nominalizer)
        || (candidate.anchor == *span
            && candidate.suffix_rules.is_empty()
            && !branch.origins.is_empty()
            && branch
                .origins
                .iter()
                .all(|origin| origin.rule_path.last().is_some_and(is_nominalizer))))
        || !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
    {
        return false;
    }
    context::normalized_token(haystack, span.clone()).is_some_and(|token| {
        resolver.has_exact_adverb_surface(&token)
            && !resolver.has_exact_predicate_nominalization(&token)
            && context::next_word_on_line(haystack, span.end).is_some_and(|(end, next)| {
                resolver.has_independent_final_predicate(&next)
                    || resolver.has_exact_adverb_surface(&next)
                    || resolver
                        .has_attached_auxiliary_final_predicate(&next, DEFAULT_LATTICE_NODE_LIMIT)
                    || (resolver.has_predicate_adnominal_surface(&next)
                        && context::next_word_on_line(haystack, end).is_some_and(|(_, head)| {
                            resolver.has_nominal_with_optional_particles(&head)
                        }))
                    || resolver.has_adverbial_copula_predicate(&next)
            })
    })
}

fn has_nominal_head_instead_of_resultative_adnominal(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let [prefix @ .., last] = candidate.suffix_rules.as_slice() else {
        return false;
    };
    if !ADNOMINAL_RULE_IDS.contains(&last.as_str())
        || !prefix
            .iter()
            .any(|rule| rule.as_str() == "ending.auxiliary-jida")
    {
        return false;
    }
    let span = &candidate.consumed;
    is_boundary_before(haystack, span.start)
        && is_boundary_after(haystack, span.end)
        && context::normalized_token(haystack, span.clone())
            .is_some_and(|token| resolver.has_exact_nominal_surface(&token))
        && follows_predicate_adnominal(haystack, span.start, resolver)
        && context::next_word_on_line(haystack, span.end).is_some_and(|(end, next)| {
            is_sentence_end_after(haystack, end)
                && (resolver.has_independent_final_predicate(&next)
                    || resolver
                        .has_attached_auxiliary_final_predicate(&next, DEFAULT_LATTICE_NODE_LIMIT))
        })
}

fn overlaps_competing_ending(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if !is_boundary_after(haystack, span.end)
        || is_boundary_before(haystack, span.start)
        || !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation { .. }
        )
    {
        return false;
    }
    let Some(tail) = context::normalized_token(haystack, span.start..span.end) else {
        return false;
    };
    let tail = tail.as_ref();
    let endings: &[&str] = match tail {
        "만" => &["지만", "건만"],
        "마는" => &["지마는", "건마는"],
        "나" => &["으나", "거나"],
        _ if tail.starts_with("는다") => std::slice::from_ref(&tail),
        _ => return false,
    };
    let whole = surrounding_token_span(haystack, span.clone());
    if whole.len() > MAX_CONSUMPTION_BYTES {
        return false;
    }
    let Some(token) = context::normalized_token(haystack, whole) else {
        return false;
    };
    // The candidate covers a competing ending or its final syllables.
    endings
        .iter()
        .find_map(|ending| token.strip_suffix(ending).map(|host| (host, *ending)))
        .is_some_and(|(host, ending)| {
            resolver.has_scope_competing_connective(&token)
                || resolver.has_connective_after_predicate(host, ending)
        })
}

fn has_pronoun_topic_instead_of_adnominal(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    let core = &candidate.verified.core;
    if !matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation { .. }
            | CandidateConsumption::StructuralPredicateEnding { .. }
    ) || core.start != span.start
        || core.end >= span.end
        || !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
        || !context::normalized_token(haystack, core.end..span.end)
            .is_some_and(|suffix| suffix == "는")
        || !context::normalized_token(haystack, core.clone())
            .is_some_and(|host| resolver.has_exact_pronoun_surface(&host))
    {
        return false;
    }
    let independent_predicate = |end, word: &str| {
        is_sentence_end_after(haystack, end)
            && !resolver.has_exact_noun_with_optional_particles(word)
            && !resolver.has_whole_modifier(word)
            && (resolver.has_independent_final_predicate(word)
                || resolver
                    .has_attached_auxiliary_final_predicate(word, DEFAULT_LATTICE_NODE_LIMIT))
            && !resolver.has_explicit_copula_after_nominal(word)
    };
    if context::next_word_on_line(haystack, span.end).is_some_and(|(end, next)| {
        (resolver.has_exact_conjunctive_adverb(&next)
            && !resolver.has_exact_noun_with_optional_particles(&next))
            || independent_predicate(end, &next)
            || (resolver.has_exact_adverb_surface(&next)
                && !resolver.has_exact_noun_with_optional_particles(&next)
                && context::next_word_on_line(haystack, end)
                    .is_some_and(|(end, word)| independent_predicate(end, &word)))
    }) {
        return true;
    }
    !branch.origins.is_empty()
        && branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| {
                    matches!(&analysis.morphology, Morphology::Predicate(predicate)
                    if predicate.flags.contains(PredicateFlags::FORCED_INFLECTED_ALIAS))
                })
        })
        && context::normalized_token(haystack, span.clone()).is_some_and(|token| {
            resolver.has_topic_marked_nominal(&token)
                && !resolver.has_exact_predicate_ending_path(&token, PredicatePos::Verb)
                && !resolver.has_exact_predicate_ending_path(&token, PredicatePos::AuxiliaryVerb)
        })
}

fn has_competing_derived_predicate(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    if !is_boundary_before(haystack, core.start)
        || is_boundary_after(haystack, candidate.consumed.end)
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            origin.rule_path.last().is_some_and(|rule| {
                matches!(
                    rule.as_str(),
                    "ending.aoeo" | "ending.connective-ji" | "ending.connective-go"
                )
            })
        })
    {
        return false;
    }
    let whole = surrounding_token_span(haystack, core.clone());
    context::normalized_token(haystack, whole).is_some_and(|token| {
        context::normalized_token(haystack, core.clone())
            .is_some_and(|prefix| resolver.has_derived_ha_predicate_crossing(&token, prefix.len()))
    })
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

fn has_topic_instead_of_propositive_adnominal(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.propositive-quotative-adnominal")
        && context::normalized_token(haystack, candidate.consumed.clone()).is_some_and(|word| {
            word.strip_suffix('는').is_some_and(|host| {
                let latin_prefix = host.bytes().take_while(u8::is_ascii_alphabetic).count();
                resolver.has_complete_nominal_surface(host)
                    || (latin_prefix > 0
                        && latin_prefix < host.len()
                        && resolver.has_complete_nominal_surface(&host[latin_prefix..]))
            })
        })
        && (!context::normalized_token(haystack, candidate.verified.core.clone())
            .is_some_and(|stem| resolver.has_exact_action_predicate_surface(&stem))
            || context::previous_word(haystack, candidate.consumed.start)
                .is_some_and(|previous| resolver.has_whole_determiner(&previous))
            || !has_quotative_adnominal_head(haystack, candidate.consumed.end, resolver))
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
    let Some(contrast) = accusative_verb_competitor(branch, atom) else {
        return false;
    };
    context::previous_word(haystack, start)
        .is_some_and(|previous| resolver.has_request_auxiliary_host(&previous))
        && context::normalized_token(haystack, token).is_some_and(|word| {
            resolver.has_vowel_auxiliary_inflection(&word, PredicatePos::Verb)
                && has_competitor_inflection(contrast, &word, |base, tail| {
                    base.iter()
                        .chain(tail)
                        .any(|rule| rule.as_str() == "ending.honorific")
                })
        })
}

fn accusative_verb_competitor(branch: &CandidateProgram, atom: &AtomPlan) -> Option<usize> {
    let mut contrasts = branch.origins.iter().map(|origin| {
        let lemma = atom
            .analyses
            .get(usize::from(origin.analysis_index))?
            .lemma
            .as_ref();
        DROP_RIEUL_OBJECT_CONTRASTS
            .binary_search_by_key(&lemma, |&(key, _)| key)
            .ok()
    });
    let contrast = contrasts.next()??;
    contrasts
        .all(|other| other == Some(contrast))
        .then_some(contrast)
}

fn has_humble_transitive_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if !resolver.has_nominal_roles() {
        return false;
    }
    let Some(contrast) = accusative_verb_competitor(branch, atom) else {
        return false;
    };
    if !branch.origins.iter().any(|origin| {
        origin
            .rule_path
            .iter()
            .chain(&candidate.suffix_rules)
            .any(|rule| rule.as_str() == "ending.archaic-naida")
    }) && !matches!(candidate.suffix_rules.as_slice(),
            [rule] if rule.as_str() == "structural.ending-path"
                && context::normalized_token(haystack, candidate.consumed.clone())
                    .is_some_and(|token| token.ends_with("나이다") || token.ends_with("나이까")))
    {
        return false;
    }
    let token = surrounding_token_span(haystack, candidate.verified.core.clone());
    if candidate.verified.core.start != token.start || candidate.consumed.end != token.end {
        return false;
    }
    let Some((start, word)) =
        context::previous_non_adverb_with_start_on_line(haystack, token.start, resolver)
    else {
        return false;
    };
    let Some(host) = word.strip_suffix(['을', '를']) else {
        return false;
    };
    if !resolver.has_object_marked_nominal(&word) {
        return false;
    }
    let count = context::previous_word(haystack, start);
    !resolver.has_elapsed_measure(host, count.as_deref())
        && !resolver.is_temporal_nominal(host)
        && !resolver.has_quantity_expression(host, None)
        && !count
            .as_deref()
            .is_some_and(|count| resolver.has_inflected_counted_unit(count, host))
        && context::normalized_token(haystack, token).is_some_and(|word| {
            has_competitor_inflection(contrast, &word, |base, tail| {
                base.iter()
                    .chain(tail)
                    .any(|rule| rule.as_str() == "ending.archaic-naida")
            })
        })
}

fn has_competitor_inflection(
    index: usize,
    text: &str,
    accepts: impl Fn(&[RuleId], &[RuleId]) -> bool,
) -> bool {
    static COMPETITORS: OnceLock<Vec<Vec<SurfaceBranchSpec>>> = OnceLock::new();
    let forms = COMPETITORS.get_or_init(|| {
        DROP_RIEUL_OBJECT_CONTRASTS
            .iter()
            .map(|&(_, transitive)| {
                let entry = PredicateEntry::new(
                    transitive,
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                generate_predicate_branches(&entry).expect("valid dictionary predicate")
            })
            .collect()
    });
    forms[index].iter().any(|form| {
        text.strip_prefix(form.anchor.as_ref()).is_some_and(|tail| {
            if tail.is_empty() {
                return accepts(&form.rule_path, &[]);
            }
            verify_predicate_continuation(form.continuation, form.pos, &form.anchor, tail)
                .is_some_and(|ending| {
                    ending.token_end == text.len() && accepts(&form.rule_path, &ending.rule_path)
                })
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

fn has_adverb_instead_of_hao_final_u(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if !candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.hao-final-u")
        || !is_boundary_before(haystack, span.start)
        || !context::normalized_token(haystack, span.clone())
            .is_some_and(|word| resolver.has_exact_adverb_surface(&word))
    {
        return false;
    }
    let previous = context::previous_word(haystack, span.start);
    if previous
        .as_deref()
        .is_some_and(|word| resolver.has_object_marked_nominal(word))
    {
        return false;
    }
    context::next_word_on_line(haystack, span.end).is_some_and(|(end, next)| {
        (next.ends_with("이다")
            && is_sentence_end_after(haystack, end)
            && resolver.has_final_copula_after_nominal(&next))
            || resolver.has_independent_final_predicate(&next)
            || (resolver.has_adjective_adnominal_surface(&next)
                && context::next_word_on_line(haystack, end)
                    .is_some_and(|(_, noun)| resolver.has_subject_marked_nominal(&noun)))
            || (previous.as_deref().is_some_and(|word| {
                context::next_word_on_line(haystack, end).is_some_and(|(_, noun)| {
                    resolver.has_copular_adnominal_frame(word, &next, &noun)
                })
            }))
            || (resolver.has_exact_predicate_connective_path(
                &next,
                kfind_morph::PredicatePosSet::one(PredicatePos::Adjective),
            ) && context::next_word_on_line(haystack, end).is_some())
            || (has_direct_quote_with_irago(haystack, span.start, end)
                && resolver.has_jeok_nominal_derivation(&next))
    })
}

fn has_direct_quote_with_irago(haystack: &[u8], start: usize, end: usize) -> bool {
    next_character_with_end(haystack, end).is_some_and(|(after_quote, close)| {
        matches!(
            (previous_character(haystack, start), close),
            (Some('"'), '"') | (Some('\''), '\'') | (Some('“'), '”') | (Some('‘'), '’')
        ) && bounded_surrounding_token_span(
            haystack,
            after_quote..after_quote,
            MAX_CONSUMPTION_BYTES,
        )
        .ok()
        .and_then(|span| context::normalized_token(haystack, span))
        .is_some_and(|word| word.starts_with("이라고"))
    })
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
