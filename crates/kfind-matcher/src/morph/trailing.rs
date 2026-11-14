use std::ops::Range;

use kfind_morph::{ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, RuleId};
use kfind_query::{CandidateConsumption, CandidateProgram};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::boundary::{is_sentence_end_after, surrounding_token_span};

use super::{
    ADNOMINAL_RULE_IDS, ExecutedCandidate, MorphMatcher, context, is_short_source_particle,
    source_aligned_compound_predicate_position,
};

impl MorphMatcher {
    pub(super) fn licensed_structural_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> (Option<Range<usize>>, bool) {
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) {
            return (
                self.licensed_nominal_copula_trailing(haystack, candidate, resolver),
                false,
            );
        }
        if !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation { .. }
        ) {
            return (None, false);
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.consumed.end >= whole.end {
            return (None, false);
        }
        let Some(trailing) = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return (None, false);
        };
        let has_rule = |expected: &str| {
            branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == expected)
        };
        let source_predicate_continuation = self.supports_source_predicate_trailing(
            haystack, candidate, &whole, trailing, branch, resolver,
        );
        let source_attached_auxiliary = candidate.suffix_rules.is_empty()
            && has_rule("ending.aoeo")
            && match branch.consumption {
                CandidateConsumption::PredicateContinuation {
                    source_positions, ..
                } => {
                    let visible_connective = candidate.consumed.end > candidate.verified.core.end;
                    let token = haystack
                        .get(whole.clone())
                        .and_then(|token| std::str::from_utf8(token).ok());
                    let complete_attached_path = token.is_some_and(|token| {
                        resolver
                            .has_complete_attached_auxiliary_path(token, DEFAULT_LATTICE_NODE_LIMIT)
                    });
                    let source_auxiliary_suffix =
                        resolver.supports_auxiliary_sequence(trailing, DEFAULT_LATTICE_NODE_LIMIT);
                    let unambiguous_contraction = haystack
                        .get(candidate.verified.core.start..candidate.consumed.end)
                        .and_then(|surface| std::str::from_utf8(surface).ok())
                        .is_some_and(|surface| {
                            resolver.has_unambiguous_predicate_surface(surface, source_positions)
                        });
                    let extended_predicate_prefix = token.is_some_and(|token| {
                        resolver.has_extended_predicate_prefix_path(
                            token,
                            candidate.verified.core.len(),
                            source_positions,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    });
                    let unambiguous_whole_path = token.is_some_and(|token| {
                        resolver.has_unambiguous_attached_auxiliary_whole_path(token)
                    });
                    let resultative_contraction = starts_with_resultative_auxiliary(trailing);
                    let aligned_auxiliary = complete_attached_path
                        && resolver.supports_auxiliary_with_fused_prefinal(
                            trailing,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                        && haystack
                            .get(whole.start..candidate.consumed.end)
                            .and_then(|surface| std::str::from_utf8(surface).ok())
                            .is_some_and(|surface| {
                                resolver
                                    .has_exact_predicate_connective_path(surface, source_positions)
                            });
                    let contracted_host = whole.start == candidate.verified.core.start
                        && has_rule("contraction.ha-yeo");
                    let supported_connective = source_auxiliary_suffix
                        && trailing.starts_with(['주', '줘', '줬', '준', '줄', '줌', '줍'])
                        && token.is_some_and(|token| {
                            !resolver.has_competing_nominal_after_connective(
                                token,
                                candidate.consumed.end - whole.start,
                            ) || (contracted_host
                                && has_contracted_auxiliary_host_context(
                                    haystack, &whole, trailing, resolver,
                                ))
                        })
                        && haystack
                            .get(whole.start..candidate.consumed.end)
                            .and_then(|surface| std::str::from_utf8(surface).ok())
                            .is_some_and(|surface| {
                                resolver.has_derived_predicate_connective_surface(
                                    surface,
                                    source_positions,
                                ) || (contracted_host
                                    && resolver.has_exact_predicate_connective_path(
                                        surface,
                                        source_positions,
                                    ))
                            });
                    (complete_attached_path
                        && (visible_connective
                            || unambiguous_contraction
                            || resultative_contraction))
                        || (source_auxiliary_suffix
                            && (unambiguous_contraction
                                || aligned_auxiliary
                                || extended_predicate_prefix
                                || unambiguous_whole_path
                                || resultative_contraction))
                        || supported_connective
                        || token.is_some_and(|token| {
                            let auxiliary_start = candidate.consumed.end - whole.start;
                            let host = &token[..auxiliary_start];
                            if !resolver.has_exact_predicate_connective_path(host, source_positions)
                                || branch.origins.iter().all(|origin| {
                                    origin
                                        .rule_path
                                        .iter()
                                        .any(|rule| rule.as_str() == "lexical.dictionary-voice")
                                })
                            {
                                return false;
                            }
                            trailing.char_indices().skip(1).any(|(split, _)| {
                                let (auxiliary, particles) = trailing.split_at(split);
                                self.particle_verifier
                                    .verify_exact(auxiliary, particles)
                                    .is_some()
                                    && resolver.has_nominalized_attached_auxiliary_path(
                                        &token[..auxiliary_start + split],
                                        DEFAULT_LATTICE_NODE_LIMIT,
                                    )
                            })
                        })
                }
                _ => false,
            };
        let licensed = (matches!(trailing, "까" | "까요") && has_rule("ending.future-adnominal"))
            || (trailing == "서도" && has_rule("ending.connective-go"))
            || (matches!(trailing, "가" | "가")
                && has_rule("ending.aoeo-seo")
                && context::next_word(haystack, whole.end)
                    .is_some_and(|next| resolver.has_negative_copula_after_focus(&next)))
            || (trailing == "도" && has_rule("ending.connective-neunde"))
            || (trailing.starts_with("잖") && has_rule("ending.past"))
            || (trailing.starts_with(['아', '어', '여'])
                && has_rule("ending.past")
                && resolver.supports_ending_suffix_path(trailing, 0, DEFAULT_LATTICE_NODE_LIMIT))
            || (ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule))
                && (trailing.starts_with(['때', '게'])
                    || context::normalized_token(haystack, candidate.consumed.end..whole.end)
                        .is_some_and(|tail| {
                            resolver.has_adnominal_auxiliary_tail(
                                &tail,
                                has_rule("ending.future-adnominal"),
                                has_rule("ending.present-adnominal")
                                    || has_rule("ending.past-adnominal"),
                            )
                        })))
            || source_attached_auxiliary
            || source_predicate_continuation;
        (
            licensed.then_some(candidate.consumed.start..whole.end),
            source_attached_auxiliary,
        )
    }

    pub(super) fn supports_source_predicate_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        whole: &Range<usize>,
        trailing: &str,
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
        let declarative_adnominal = trailing.nfc().eq("는".chars())
            && haystack
                .get(candidate.consumed.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(|consumed| consumed.nfc().last())
                == Some('다');
        let has_rule = |expected: &str| {
            branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == expected)
        };
        let forbids_ending_auxiliary_particles = [
            "ending.declarative",
            "ending.interrogative-neunya",
            "ending.connective-ni",
            "ending.conditional",
            "ending.prospective-final",
            "ending.hao-final-so",
            "ending.hao-final",
            "ending.hao-final-u",
        ]
        .iter()
        .any(|rule| has_rule(rule));
        let prospective_quotative_topic = has_rule("ending.prospective-quotative");
        let ending_auxiliary_particles = !declarative_adnominal
            && !forbids_ending_auxiliary_particles
            && haystack
                .get(candidate.consumed.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .is_some_and(|ending| {
                    let ending = ending.nfc().collect::<String>();
                    let trailing = trailing.nfc().collect::<String>();
                    let matched = self
                        .particle_verifier
                        .verify_non_nominal_prefix(&ending, &trailing);
                    matched.consumed_bytes == trailing.len()
                        && (!prospective_quotative_topic
                            || matched
                                .rule_path
                                .first()
                                .is_some_and(|rule| rule.as_str() == "particle.topic"))
                        && matched.rule_path.first().is_some_and(|rule| {
                            contains_rule(&self.plan.predicate_ending_initial_particle_rules, rule)
                        })
                        && matched
                            .rule_path
                            .iter()
                            .all(|rule| contains_rule(&self.plan.auxiliary_particle_rules, rule))
                });
        let has_adnominal_rule = ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule));
        let adnominal_dependent_noun_particle =
            trailing.nfc().next() == Some('지') && has_adnominal_rule;
        let adnominal_interrogative = trailing.nfc().eq("가".chars()) && has_adnominal_rule;
        let source_continuation = matches!(
            continuation,
            kfind_morph::ContinuationState::Past
                | kfind_morph::ContinuationState::Future
                | kfind_morph::ContinuationState::Eu
        ) && !has_adnominal_rule
            && licensed_source_ending_surface(trailing);
        let normalized_trailing = trailing.nfc().collect::<String>();
        let source_ending_trailing = source_continuation
            && resolver.supports_ending_suffix_path(
                &normalized_trailing,
                0,
                DEFAULT_LATTICE_NODE_LIMIT,
            );
        let source_nominalizer_particle_split = source_continuation.then_some(()).and_then(|()| {
            trailing.char_indices().skip(1).find_map(|(split, _)| {
                let ending = trailing.get(..split)?;
                let particles = trailing.get(split..)?;
                (resolver.has_exact_nominalizing_ending_surface(ending)
                    && self
                        .particle_verifier
                        .verify_exact(ending, particles)
                        .is_some())
                .then_some(split)
            })
        });
        let licensed_non_ending_trailing = declarative_adnominal
            || ending_auxiliary_particles
            || adnominal_dependent_noun_particle
            || adnominal_interrogative;
        let source_aligned_compound = source_aligned_compound_predicate_position(
            haystack,
            &candidate.verified.core,
            whole,
            source_positions,
            resolver,
        );
        if !licensed_non_ending_trailing
            && !source_ending_trailing
            && source_nominalizer_particle_split.is_none()
            && !source_aligned_compound
        {
            return false;
        }
        let valid_position = if pos == kfind_morph::PredicatePos::Copula {
            candidate.verified.core.start > whole.start
        } else {
            (continuation != kfind_morph::ContinuationState::Terminal
                || licensed_non_ending_trailing
                || source_ending_trailing
                || source_nominalizer_particle_split.is_some())
                && (candidate.verified.core.start == whole.start || source_aligned_compound)
        };
        if !valid_position
            || (pos != kfind_morph::PredicatePos::Copula
                && !licensed_non_ending_trailing
                && self
                    .particle_verifier
                    .model()
                    .allomorphs
                    .iter()
                    .any(|form| trailing.starts_with(form.surface.as_ref())))
        {
            return false;
        }

        let token = haystack
            .get(candidate.verified.core.start..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let core = haystack
            .get(candidate.verified.core.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let ending = haystack
            .get(candidate.verified.core.start..candidate.consumed.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        token
            .zip(core)
            .zip(ending)
            .is_some_and(|((token, core), ending)| {
                if is_nfc(token) {
                    if resolver.has_whole_modifier(token) && !adnominal_interrogative {
                        return false;
                    }
                    return if ending_auxiliary_particles {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_particle_path(
                                token,
                                core.len(),
                                ending.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    } else if let Some(split) = source_nominalizer_particle_split {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_particle_path(
                                token,
                                core.len(),
                                ending.len() + split,
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    } else if adnominal_dependent_noun_particle {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_adnominal_dependent_noun_particle_path(
                                token,
                                core.len(),
                                ending.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            ) || resolver.has_exact_predicate_ending_path(token, source_pos)
                        })
                    } else {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_path(
                                token,
                                core.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    };
                }
                let normalized = token.nfc().collect::<String>();
                let core_len = core.nfc().map(char::len_utf8).sum();
                let ending_len = ending.nfc().map(char::len_utf8).sum();
                if resolver.has_whole_modifier(&normalized) && !adnominal_interrogative {
                    return false;
                }
                if ending_auxiliary_particles {
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_predicate_ending_particle_path(
                            &normalized,
                            core_len,
                            ending_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    })
                } else if let Some(split) = source_nominalizer_particle_split {
                    let nominalizer_len = trailing
                        .get(..split)
                        .map(|surface| surface.nfc().map(char::len_utf8).sum::<usize>())
                        .unwrap_or_default();
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_predicate_ending_particle_path(
                            &normalized,
                            core_len,
                            ending_len + nominalizer_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    })
                } else if adnominal_dependent_noun_particle {
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_adnominal_dependent_noun_particle_path(
                            &normalized,
                            core_len,
                            ending_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        ) || resolver.has_exact_predicate_ending_path(&normalized, source_pos)
                    })
                } else {
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_predicate_ending_path(
                            &normalized,
                            core_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    })
                }
            })
    }

    pub(super) fn has_rejected_structural_suffix(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        consumed: &Range<usize>,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> bool {
        if !matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
                | CandidateConsumption::PredicateContinuation { .. }
        ) {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if consumed.end >= whole.end {
            return false;
        }
        let Some(suffix) = haystack
            .get(consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return true;
        };
        let normalized = suffix.nfc().collect::<String>();
        let particle_shaped = self
            .particle_verifier
            .model()
            .allomorphs
            .iter()
            .any(|form| {
                normalized.starts_with(form.surface.as_ref())
                    && is_structural_suffix_particle(form, branch)
            });
        if particle_shaped {
            return true;
        }
        let CandidateConsumption::PredicateContinuation {
            source_positions,
            nominal_particle_transition,
            ..
        } = branch.consumption
        else {
            return false;
        };
        if nominal_particle_transition {
            return false;
        }
        let trailing = std::str::from_utf8(&haystack[consumed.end..whole.end]).unwrap_or_default();
        trailing.char_indices().skip(1).any(|(offset, _)| {
            let split = consumed.end + offset;
            let remainder = trailing.get(offset..).unwrap_or_default();
            let particle_remainder = self
                .particle_verifier
                .model()
                .allomorphs
                .iter()
                .any(|form| {
                    remainder.starts_with(form.surface.as_ref())
                        && is_structural_suffix_particle(form, branch)
                });
            if !particle_remainder {
                return false;
            }
            let Some(prefix) = haystack
                .get(candidate.verified.core.start..split)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            else {
                return false;
            };
            let normalized_prefix = prefix.nfc().collect::<String>();
            let Some(core) = haystack
                .get(candidate.verified.core.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            else {
                return false;
            };
            let normalized_core_len = core.nfc().collect::<String>().len();
            source_positions.iter().any(|source_pos| {
                resolver.supports_predicate_ending_path(
                    &normalized_prefix,
                    normalized_core_len,
                    source_pos,
                    DEFAULT_LATTICE_NODE_LIMIT,
                )
            })
        })
    }

    pub(super) fn has_terminal_particle_conflict(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        whole: &Range<usize>,
    ) -> bool {
        let model = self.particle_verifier.model();
        let is_terminal = |rule_id: &RuleId| {
            model
                .transitions
                .iter()
                .find(|transition| &transition.rule_id == rule_id)
                .is_some_and(|transition| transition.next.is_empty())
        };
        if candidate.suffix_rules.iter().any(is_terminal) {
            return true;
        }
        let Some(trailing) = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let trailing = trailing.nfc().collect::<String>();
        model
            .allomorphs
            .iter()
            .any(|form| is_terminal(&form.rule_id) && trailing.starts_with(form.surface.as_ref()))
    }
}

fn has_contracted_auxiliary_host_context(
    haystack: &[u8],
    token: &Range<usize>,
    auxiliary: &str,
    resolver: &ConstraintResolver,
) -> bool {
    context::previous_word(haystack, token.start).is_some_and(|previous| {
        resolver.has_exact_predicate_connective_path(
            &previous,
            kfind_morph::PredicatePosSet::one(kfind_morph::PredicatePos::Verb),
        ) || (previous
            .strip_suffix("도록")
            .is_some_and(|stem| resolver.has_exact_non_copula_predicate_surface(stem)))
            || (is_sentence_end_after(haystack, token.end)
                && (resolver.has_exact_adverb_surface(&previous)
                    || resolver.has_object_marked_nominal(&previous)))
            || (resolver.has_predicate_adnominal_surface(auxiliary)
                && (resolver.has_subject_marked_nominal(&previous)
                    || resolver.has_nominal_oblique(&previous))
                && context::next_word(haystack, token.end)
                    .is_some_and(|next| resolver.has_independent_nominal_head(&next)))
    })
}

fn contains_rule(rules: &[RuleId], rule: &RuleId) -> bool {
    rules
        .binary_search_by_key(&rule.as_str(), |known| known.as_str())
        .is_ok()
}

pub(super) fn licensed_source_ending_surface(surface: &str) -> bool {
    const EU_INITIAL_ENDINGS: &[&str] = &[
        "으니까는",
        "으니까",
        "으니깐",
        "으니",
        "으면서",
        "으면",
        "으며",
        "으나",
        "으므로",
        "으려고",
        "으려는",
        "으리라고",
        "으리라",
        "으되",
        "을",
        "음",
    ];

    !surface.starts_with('으')
        || EU_INITIAL_ENDINGS
            .iter()
            .any(|ending| surface.starts_with(ending))
}

fn is_structural_suffix_particle(
    form: &kfind_morph::ParticleAllomorph,
    branch: &CandidateProgram,
) -> bool {
    !is_short_source_particle(form)
        && !(form.rule_id.as_str() == "particle.comitative.hago"
            && matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation { .. }
            ))
}

fn starts_with_resultative_auxiliary(surface: &str) -> bool {
    surface
        .chars()
        .next()
        .is_some_and(|first| matches!(first, '지' | '졌' | '진' | '질' | '져' | '집' | '짐'))
}
