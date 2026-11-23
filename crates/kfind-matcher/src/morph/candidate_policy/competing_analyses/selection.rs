use std::borrow::Cow;

use kfind_data::NominalRole;
use kfind_morph::{CoarsePos, ConstraintResolver, verify_copula_surface_after_nominal};
use kfind_query::{AtomPlan, CandidateProgram};

use crate::boundary::{is_sentence_end_after, surrounding_token_span};

use super::{ExecutedCandidate, context};

pub(super) fn has_incompatible_argument(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let mut analyses = branch.origins.iter().map(|origin| {
        let analysis = &atom.analyses[usize::from(origin.analysis_index)];
        (analysis.lemma.as_ref(), analysis.coarse_pos)
    });
    let Some((lemma, CoarsePos::Verb)) = analyses.next() else {
        return false;
    };
    if !matches!(
        lemma,
        "살다" | "나서다" | "있다" | "밝다" | "늦다" | "크다" | "가다" | "빌다" | "날다" | "되다"
    ) || !analyses.all(|analysis| analysis == (lemma, CoarsePos::Verb))
    {
        return false;
    }
    let core = &candidate.verified.core;
    let whole = surrounding_token_span(haystack, core.clone());
    if core.start != whole.start {
        return false;
    }
    let Some(word) = context::normalized_token(haystack, whole.clone()) else {
        return false;
    };
    let Some((start, previous)) =
        context::previous_non_adverb_with_start_on_line(haystack, whole.start, resolver)
    else {
        return false;
    };
    let prior = |at| preceding_words(haystack, at).next();
    let marked_subject = |text: &str, role| {
        text.strip_suffix(['이', '가'])
            .or_else(|| text.strip_suffix(['은', '는']))
            .is_some_and(|noun| {
                resolver.has_nominal_role(noun, role)
                    && (resolver.has_subject_marked_nominal(text)
                        || resolver.has_topic_marked_nominal(text))
            })
    };
    let standard_nominal_clause = |word: &str| {
        word.strip_suffix("에는")
            .or_else(|| word.strip_suffix("엔"))
            .is_some_and(|host| resolver.has_ki_nominalization(host))
    };
    let shares_stative_ending = |stem: &str| {
        word.strip_prefix(stem)
            .is_some_and(|ending| matches!(ending, "지" | "지요" | "죠" | "습니다" | "아서야"))
    };
    match lemma {
        // 살다 licenses an accusative for the life, term or capacity lived
        // (KRDict 69579). A concrete food object in a purchase frame selects
        // 사다 (49419). Keep clauses without an overt object ambiguous.
        "살다" => {
            for (_, argument) in preceding_words(haystack, whole.start) {
                if argument.strip_suffix(['을', '를']).is_some_and(|noun| {
                    resolver.has_nominal_role(noun, NominalRole::Food)
                        && resolver.has_object_marked_nominal(&argument)
                }) {
                    return ConstraintResolver::has_purchase_inflection(&word);
                }
                if !resolver.has_nominal_oblique(&argument)
                    && !resolver.has_exact_adverb_surface(&argument)
                {
                    break;
                }
            }
            false
        }
        // The secretion subject belongs to the emission frame of 나다
        // (62210), not the active or sought-entity frames of 나서다 (61183).
        "나서다" => {
            word.strip_prefix('나').is_some_and(|ending| {
                ending.starts_with('서') && resolver.has_connective_after_predicate("나", ending)
            }) && marked_subject(&previous, NominalRole::BodySecretion)
        }
        // A sound is the subject of occurrence 나다 (62210). Preserve
        // motion with an overt path or goal in the predicate phrase.
        "날다" if word.starts_with("난다") => {
            marked_subject(&previous, NominalRole::AudiblePhenomenon)
                && !has_direction_before_subject(haystack, start, resolver)
        }
        // The container or occupancy subject takes 비다 (KRDict 15680).
        // A preceding temporal measure clause can modify its rate of change.
        "빌다"
            if matches!(
                word.as_ref(),
                "비나이다" | "비옵나이다" | "비나이까" | "비옵나이까"
            ) =>
        {
            let mut has_measure_predicate = false;
            for (start, word) in preceding_words(haystack, whole.start) {
                if marked_subject(&word, NominalRole::Emptyable) {
                    return !prior(start).is_some_and(|(_, leading)| {
                        resolver.has_subject_marked_nominal(&leading)
                            || resolver.has_topic_marked_nominal(&leading)
                    });
                }
                if resolver.has_exact_adverb_surface(&word) {
                    has_measure_predicate |= word
                        .strip_suffix('게')
                        .is_some_and(|stem| resolver.has_exact_adjective_surface(stem));
                } else if !has_measure_predicate
                    || !word
                        .strip_suffix(['이', '가'])
                        .is_some_and(|host| resolver.has_elapsed_measure(host, None))
                {
                    break;
                }
            }
            false
        }
        // The subject of the possibility adjective 있다 is the bound noun 수.
        "있다"
            if prior(start).is_some_and(|(_, modifier)| {
                resolver.has_possibility_nominal(&modifier, &previous)
            }) =>
        {
            true
        }
        // A bare cardinality after a nominative subject and a preceding -에(는)
        // supplies the existential count. Distributive or duration forms
        // can instead qualify someone staying and are excluded.
        "있다" if shares_stative_ending("있") && previous.as_ref() == "그루" => prior(start)
            .is_some_and(|(start, count)| {
                resolver.has_standalone_counted_unit(&count, &previous)
                    && prior(start).is_some_and(|(start, subject)| {
                        resolver.has_subject_marked_nominal(&subject)
                            && locative_before_subject(haystack, start, resolver)
                    })
            }),
        // A degree reading on a display surface describes emitted light
        // (밝다 VA, 62105). Keep unmarked changes such as 화면이 밝아 왔다.
        "밝다" if shares_stative_ending("밝") => {
            marked_subject(&previous, NominalRole::DisplaySurface)
                && context::previous_word(haystack, whole.start).is_some_and(|degree| {
                    matches!(
                        degree.as_ref(),
                        "아주" | "매우" | "너무" | "꽤" | "그렇게" | "이렇게"
                    ) && resolver.has_exact_adverb_surface(&degree)
                })
        }
        // The -기에는 clause sets a standard (suitability for the action).
        // With a time or physical article as subject, the predicate states
        // a time/size property relative to that standard.
        "늦다" if shares_stative_ending("늦") => {
            let time = previous.strip_suffix(['이', '가', '은', '는']);
            (time == Some("시간") || time.is_some_and(|noun| resolver.is_temporal_nominal(noun)))
                && (resolver.has_subject_marked_nominal(&previous)
                    || resolver.has_topic_marked_nominal(&previous))
                && context::previous_non_adverb_on_line(haystack, start, resolver)
                    .is_some_and(|standard| standard_nominal_clause(&standard))
        }
        "크다" if shares_stative_ending("크") && standard_nominal_clause(&previous) => {
            for (_, word) in preceding_words(haystack, start) {
                if marked_subject(&word, NominalRole::HouseholdArticle) {
                    return true;
                }
                if !resolver.has_nominal_oblique(&word)
                    && !resolver.has_object_marked_nominal(&word)
                    && !resolver.has_nominal_genitive_surface(&word)
                    && !resolver.has_pronominal_genitive_surface(&word)
                    && !resolver.has_exact_adverb_surface(&word)
                    && !resolver.has_predicate_adnominal_surface(&word)
                {
                    break;
                }
            }
            false
        }
        // 가다 takes an accusative route, while 가지다 takes a possessed object
        // and can combine with an independent motion verb (KRDict 27500, 65234).
        "가다" if word.starts_with("가지") => {
            if !branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .iter()
                    .chain(&candidate.suffix_rules)
                    .any(|rule| rule.as_str() == "ending.auxiliary-jida")
            }) || !ConstraintResolver::has_possession_inflection(&word)
            {
                return false;
            }
            let marked_object = resolver.has_object_marked_nominal(&previous);
            if marked_object
                && previous.strip_suffix(['을', '를']).is_some_and(|noun| {
                    resolver.has_nominal_role(noun, NominalRole::OwnershipAsset)
                })
            {
                return true;
            }
            let is_motion = |word: &str| {
                ConstraintResolver::has_motion_go_inflection(word)
                    && !ConstraintResolver::has_possession_inflection(word)
            };
            word.ends_with('고')
                && (marked_object
                    || (resolver.has_exact_nominal_surface(&previous)
                        && !resolver.has_exact_non_copula_predicate_surface(&previous)))
                && context::next_word_on_line(haystack, whole.end).is_some_and(|(end, next)| {
                    is_motion(&next)
                        || (((next.ends_with(['에', '로']) && resolver.has_nominal_oblique(&next))
                            || (resolver.has_exact_adverb_surface(&next)
                                && !resolver.has_exact_noun_with_optional_particles(&next)))
                            && context::next_word_on_line(haystack, end)
                                .is_some_and(|(_, motion)| is_motion(&motion)))
                })
        }
        // A subject that denotes an orthographic unit can be identified
        // by a syllable followed by reduced 이다. Motion requires separate
        // evidence when the subject names a character rather than an agent.
        "가다" => {
            word.strip_prefix('가').is_some_and(|ending| {
                matches!(ending, "랴" | "렷다") && verify_copula_surface_after_nominal('가', ending)
            }) && marked_subject(&previous, NominalRole::OrthographicUnit)
                && !has_direction_before_subject(haystack, start, resolver)
        }
        "되다" => {
            branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .iter()
                    .chain(&candidate.suffix_rules)
                    .any(|rule| rule.as_str() == "ending.auxiliary-jida")
            }) && has_parallel_animal_labels(haystack, whole.end, &word, &previous, resolver)
        }
        _ => false,
    }
}

fn has_parallel_animal_labels(
    haystack: &[u8],
    end: usize,
    word: &str,
    previous: &str,
    resolver: &ConstraintResolver,
) -> bool {
    // NIKL 327663: a vowel-final nominal can omit 이- before -고.
    // Parallel deictic topics and two animal labels anchor that reading.
    let is_deictic_topic = |word: &str| {
        word.strip_suffix(['은', '는']).is_some_and(|host| {
            context::is_locative_pronoun(host)
                || host
                    .strip_suffix('쪽')
                    .is_some_and(|stem| matches!(stem, "이" | "그" | "저"))
        }) && resolver.has_topic_marked_nominal(word)
    };
    if !is_deictic_topic(previous)
        || haystack.get(end) != Some(&b',')
        || !["고요", "고"].into_iter().any(|ending| {
            word.strip_suffix(ending).is_some_and(|host| {
                resolver.has_nominal_role(host, NominalRole::Animal)
                    && resolver.has_exact_nominal_surface(host)
                    && host
                        .chars()
                        .next_back()
                        .is_some_and(|last| !kfind_morph::hangul::has_final(last))
            })
        })
    {
        return false;
    }
    context::next_word_on_line(haystack, end + 1).is_some_and(|(topic_end, topic)| {
        topic != previous
            && is_deictic_topic(&topic)
            && context::next_word_on_line(haystack, topic_end).is_some_and(|(end, predicate)| {
                is_sentence_end_after(haystack, end)
                    && predicate.char_indices().skip(1).any(|(split, _)| {
                        let (label, copula) = predicate.split_at(split);
                        resolver.has_nominal_role(label, NominalRole::Animal)
                            && matches!(copula, "예요" | "이에요" | "입니다" | "이다")
                            && label.chars().next_back().is_some_and(|last| {
                                verify_copula_surface_after_nominal(last, copula)
                            })
                    })
            })
    })
}

fn has_direction_before_subject(
    haystack: &[u8],
    start: usize,
    resolver: &ConstraintResolver,
) -> bool {
    for (_, word) in preceding_words(haystack, start) {
        if (word.ends_with('로') || word.ends_with("까지"))
            && (resolver.has_nominal_oblique(&word)
                || resolver.has_nominal_with_optional_particles(&word))
        {
            return true;
        }
        if !resolver.has_whole_determiner(&word)
            && !resolver.has_nominal_genitive_surface(&word)
            && !resolver.has_pronominal_genitive_surface(&word)
            && !resolver.has_predicate_adnominal_surface(&word)
            && !resolver.has_exact_adverb_surface(&word)
        {
            break;
        }
    }
    false
}

fn locative_before_subject(haystack: &[u8], start: usize, resolver: &ConstraintResolver) -> bool {
    for (_, word) in preceding_words(haystack, start) {
        let locative = word.strip_suffix('는').unwrap_or(&word);
        if locative.ends_with('에') && resolver.has_nominal_oblique(locative) {
            return true;
        }
        if !resolver.has_predicate_adnominal_surface(&word)
            && !resolver.has_nominal_genitive_surface(&word)
        {
            break;
        }
    }
    false
}

pub(super) fn has_content_topic(
    haystack: &[u8],
    predicate: &std::ops::Range<usize>,
    resolver: &ConstraintResolver,
) -> bool {
    // A specified part of a text or statement can be identified by a
    // sentence-final nominal predicate with omitted 이다.
    if !is_sentence_end_after(haystack, predicate.end) {
        return false;
    }
    let mut words = preceding_words(haystack, predicate.start);
    while let Some((_, word)) = words.next() {
        if word.strip_suffix(['은', '는']).is_some_and(|host| {
            resolver.has_nominal_role(host, NominalRole::ContentUnit)
                && resolver.has_topic_marked_nominal(&word)
        }) {
            return words.next().is_some_and(|(_, modifier)| {
                resolver.has_predicate_adnominal_surface(&modifier)
                    || resolver.has_derived_predicate_adnominal_surface(&modifier)
            });
        }
        if !resolver.has_exact_adverb_surface(&word)
            && !resolver.has_exact_conjunctive_adverb(&word)
        {
            break;
        }
    }
    false
}

fn preceding_words(haystack: &[u8], at: usize) -> impl Iterator<Item = (usize, Cow<'_, str>)> {
    let limit = at.saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    let mut before = at;
    std::iter::from_fn(move || {
        let (start, word) = context::previous_word_with_start(haystack, before)?;
        if start < limit
            || haystack[start..before]
                .iter()
                .any(|byte| matches!(byte, b'\r' | b'\n'))
        {
            return None;
        }
        before = start;
        Some((start, word))
    })
}
