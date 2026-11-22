use std::borrow::Cow;
use std::ops::Range;

use kfind_data::DataFinePos;
use kfind_morph::{ConstraintResolver, verify_copula_surface_after_nominal};
use kfind_query::{CandidateConsumption, CandidateProgram};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::boundary::{is_sentence_end_after, surrounding_token_span};

use super::{
    ExecutedCandidate, MorphMatcher, context, follows_predicate_adnominal, fused_copula,
    has_quotative_adnominal_head,
};

impl MorphMatcher {
    pub(super) fn accepts_product_copula_frame(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
        patterns: &[kfind_morph::QueryMorphPattern],
    ) -> bool {
        let CandidateConsumption::PredicateContinuation { pos, .. } = branch.consumption else {
            return false;
        };
        if pos != kfind_morph::PredicatePos::Copula
            || !patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Vcp)
        {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.verified.core.start <= whole.start {
            return false;
        }
        if candidate.consumed.end != whole.end {
            return candidate.consumed.end < whole.end
                && candidate
                    .suffix_rules
                    .last()
                    .is_some_and(|rule| rule.as_str() == "ending.interrogative-nga")
                && context::normalized_token(haystack, candidate.consumed.end..whole.end)
                    .is_some_and(|quote| matches!(quote.as_ref(), "라고" | "라고요" | "라는"))
                && context::normalized_token(haystack, whole.start..candidate.verified.core.start)
                    .is_some_and(|host| resolver.has_complete_nominal_surface(&host));
        }
        let Some(token) = haystack
            .get(whole.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let token = token.nfc().collect::<String>();
        if resolver.has_whole_modifier(&token) {
            return false;
        }
        let Some(host) = haystack
            .get(whole.start..candidate.verified.core.start)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let host = host.nfc().collect::<String>();
        if branch.anchor.as_ref() == "다".as_bytes()
            && host == "하"
            && context::previous_word_with_start(haystack, whole.start).is_some_and(
                |(start, word)| {
                    !haystack[start..whole.start]
                        .iter()
                        .any(|byte| matches!(byte, b'\r' | b'\n'))
                        && self.has_copula_nominalizer_particles(&word, resolver)
                },
            )
        {
            return false;
        }
        if branch.anchor.as_ref() == "다".as_bytes()
            && !resolver.has_exact_nominal_surface(&host)
            && (resolver.has_exact_non_copula_predicate_surface(&token)
                || resolver.has_independent_final_predicate(&token)
                || resolver.has_nominal_with_optional_particles(&token))
        {
            return false;
        }
        if host.ends_with('하')
            && context::previous_word(haystack, whole.start)
                .is_some_and(|previous| resolver.has_object_marked_nominal(&previous))
            && token
                .get(host.len()..)
                .is_some_and(|ending| resolver.has_derived_ha_past(&host, ending))
        {
            return false;
        }
        ((host.bytes().all(|byte| byte.is_ascii_digit())
            || resolver.has_numeric_unit_surface(&host))
            && token.get(host.len()..).is_some_and(|ending| {
                host.chars()
                    .next_back()
                    .is_some_and(|preceding| verify_copula_surface_after_nominal(preceding, ending))
            }))
            || resolver.has_complete_nominal_surface(&host)
            || host
                .char_indices()
                .map(|(offset, _)| offset)
                .skip(1)
                .any(|split| {
                    let nominal = &host[..split];
                    let particles = &host[split..];
                    resolver.has_complete_nominal_surface(nominal)
                        && self
                            .particle_verifier
                            .verify_exact(nominal, particles)
                            .is_some()
                })
    }

    pub(super) fn accepts_nominal_copula_ending(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
        patterns: &[kfind_morph::QueryMorphPattern],
    ) -> bool {
        let dependent = branch.is_dependent_nominal_copula_ending()
            || patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nnb
                    && pattern.continuation == kfind_morph::MorphContinuation::NominalCopulaEnding
            });
        let pronoun = patterns.iter().any(|pattern| {
            pattern.fine_pos == DataFinePos::Np
                && pattern.continuation == kfind_morph::MorphContinuation::NominalCopulaEnding
        });
        if !matches!(
            branch.consumption,
            CandidateConsumption::NominalCopulaEndingChain { .. }
        ) || !(dependent || pronoun)
        {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.anchor != candidate.verified.core || candidate.consumed != whole {
            return false;
        }
        let Some(anchor) = haystack
            .get(candidate.anchor.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let normalized = anchor.nfc().collect::<String>();
        self.accepts_token_boundary(haystack, &candidate.verified, branch)
            && ((pronoun && resolver.has_exact_pronoun_copula_ending_path(&normalized))
                || (dependent
                    && follows_predicate_adnominal(haystack, whole.start, resolver)
                    && resolver.has_exact_dependent_copula_ending_path(&normalized)))
    }

    pub(super) fn accepts_lost_span_copula_ending(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> bool {
        let Some(token) = haystack
            .get(candidate.verified.core.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let normalized = if is_nfc(token) {
            Cow::Borrowed(token)
        } else {
            Cow::Owned(token.nfc().collect::<String>())
        };
        if !self.accepts_token_boundary(haystack, &candidate.verified, branch) {
            return false;
        }
        let fused_rieul_question = normalized
            .strip_suffix("까요")
            .or_else(|| normalized.strip_suffix('까'))
            .and_then(|stem| stem.chars().next_back())
            .is_some_and(kfind_morph::hangul::has_rieul_final);
        if fused_rieul_question
            && context::previous_word(haystack, candidate.verified.token.start)
                .is_some_and(|word| resolver.has_object_marked_nominal(&word))
            && resolver.has_independent_final_verb_with_object(&normalized)
        {
            return false;
        }
        let final_ending = normalized
            .strip_suffix('야')
            .map(|host| (host, "야"))
            .or_else(|| normalized.strip_suffix("라니").map(|host| (host, "라니")));
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
            && let Some((host, ending)) = final_ending
            && host.chars().next_back().is_some_and(|last| {
                kfind_morph::decompose_syllable(last)
                    .is_some_and(|syllable| syllable.jongseong == 0)
            })
            && is_sentence_end_after(haystack, candidate.verified.token.end)
            && !resolver.has_exact_non_copula_predicate_surface(&normalized)
        {
            let pronoun = resolver.has_exact_pronoun_surface(host);
            let subject = || {
                context::has_preceding_subject_or_topic(
                    haystack,
                    candidate.verified.token.start,
                    resolver,
                )
            };
            let accepted = if ending == "라니" {
                pronoun && subject()
            } else {
                resolver.has_exact_nominal_surface(host)
                    && (pronoun
                        && (haystack[candidate.verified.token.end..].starts_with(b"?")
                            || haystack[candidate.verified.token.end..]
                                .starts_with("？".as_bytes()))
                        || subject())
            };
            if accepted {
                return true;
            }
        }
        if (normalized.ends_with("더군") || normalized.ends_with("더군요"))
            && context::previous_word(haystack, candidate.verified.token.start)
                .is_some_and(|previous| resolver.has_object_marked_nominal(&previous))
        {
            return false;
        }
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
            && let Some(host) = normalized.strip_suffix('라')
            && host.chars().next_back().is_some_and(|last| {
                kfind_morph::decompose_syllable(last)
                    .is_some_and(|syllable| syllable.jongseong == 0)
            })
            && resolver.has_exact_nominal_surface(host)
            && !resolver.has_exact_non_copula_predicate_surface(&normalized)
            && if is_sentence_end_after(haystack, candidate.verified.token.end) {
                context::previous_word_with_start(haystack, candidate.verified.token.start)
                    .is_some_and(|(start, modifier)| {
                        resolver.has_exact_nominal_surface(&modifier)
                            && context::previous_word(haystack, start)
                                .is_some_and(|word| resolver.has_nominal_genitive_surface(&word))
                    })
            } else {
                !resolver.has_exact_action_predicate_surface(host)
                    && context::previous_word(haystack, candidate.verified.token.start)
                        .is_some_and(|word| resolver.has_subject_marked_nominal(&word))
                    && context::has_following_final_predicate(
                        haystack,
                        candidate.verified.token.end,
                        resolver,
                    )
            }
        {
            return true;
        }
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
            && let Some(host) = normalized.strip_suffix("라느니")
            && host.chars().next_back().is_some_and(|last| {
                kfind_morph::decompose_syllable(last)
                    .is_some_and(|syllable| syllable.jongseong == 0)
            })
            && resolver.has_exact_nominal_surface(host)
            && context::previous_word(haystack, candidate.verified.token.start)
                .is_some_and(|previous| resolver.has_surname_title(&previous, host))
        {
            return true;
        }
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
            && let Some(host) = normalized
                .strip_suffix("라면서")
                .or_else(|| normalized.strip_suffix("라며"))
            && host.chars().next_back().is_some_and(|last| {
                kfind_morph::decompose_syllable(last)
                    .is_some_and(|syllable| syllable.jongseong == 0)
            })
            && haystack
                .get(candidate.verified.token.end..)
                .is_some_and(|following| {
                    following.starts_with(b"?") || following.starts_with("？".as_bytes())
                })
            && resolver.has_exact_nominal_surface(host)
            && context::previous_word_with_start(haystack, candidate.verified.token.start)
                .is_some_and(|(at, noun)| {
                    resolver.has_exact_nominal_surface(&noun)
                        && context::previous_word_with_start(haystack, at).is_some_and(
                            |(before, modifier)| {
                                resolver.has_adjective_adnominal_surface(&modifier)
                                    && context::has_preceding_subject_or_topic(
                                        haystack, before, resolver,
                                    )
                            },
                        )
                })
        {
            return true;
        }
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
            && let Some(host) = normalized.strip_suffix("려니")
            && host.chars().next_back().is_some_and(|last| {
                kfind_morph::decompose_syllable(last)
                    .is_some_and(|syllable| syllable.jongseong == 0)
            })
            && resolver.has_complete_nominal_surface(host)
            && !host
                .strip_suffix('이')
                .is_some_and(|nominal| resolver.has_complete_nominal_surface(nominal))
            && !resolver.has_exact_non_copula_predicate_surface(host)
            && !resolver.has_exact_non_copula_predicate_surface(&normalized)
            && !resolver.has_exact_nominal_surface(&normalized)
        {
            return true;
        }
        if let Some(stem) = normalized
            .strip_suffix("렷다")
            .or_else(|| normalized.strip_suffix('랴'))
            && resolver.has_exact_action_predicate_surface(stem)
            && has_directional_adverbial_frame(haystack, candidate.verified.token.start, resolver)
        {
            return false;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::CopulaHostEndingCompose
        ) && branch.origins.iter().any(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "contraction.geos-copula-nieun-go")
        }) {
            return is_sentence_end_after(haystack, candidate.verified.token.end)
                && context::previous_word(haystack, candidate.verified.token.start).is_some_and(
                    |previous| resolver.has_adnominal_dependent_noun(&previous, "것"),
                );
        }
        let continues_enumeration = |host: &str| {
            let end = candidate.verified.token.end;
            let next = if haystack.get(end) == Some(&b',') {
                end + 1
            } else {
                end
            };
            context::next_word_on_line(haystack, next).is_some_and(|(_, word)| {
                word != host
                    && !resolver.has_object_marked_nominal(&word)
                    && (resolver.has_nominal_with_optional_particles(&word)
                        || resolver.has_predicate_adnominal_surface(&word))
            })
        };
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
            && haystack.get(candidate.verified.token.end) == Some(&b',')
            && let Some(host) = normalized.strip_suffix('요')
            && host
                .chars()
                .next_back()
                .and_then(kfind_morph::decompose_syllable)
                .is_some_and(|last| last.jongseong == 0)
            && resolver.has_exact_nominal_surface(host)
            && !context::previous_word(haystack, candidate.verified.token.start)
                .is_some_and(|word| resolver.has_object_marked_nominal(&word))
            && context::next_word_on_line(haystack, candidate.verified.token.end + 1).is_some_and(
                |(end, word)| {
                    self.has_enumerated_copula(haystack, end, &word, resolver)
                        || (resolver.has_independent_nominal_head(&word)
                            && context::next_word_on_line(haystack, end).is_some_and(
                                |(end, next)| {
                                    self.has_enumerated_copula(haystack, end, &next, resolver)
                                },
                            ))
                },
            )
        {
            return true;
        }
        ((fused_rieul_question
            || normalized
                .strip_suffix('요')
                .is_none_or(continues_enumeration))
            && resolver.has_exact_lost_span_copula_ending_path(&normalized))
            || (matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
                && (fused_copula::accepts_fused_retrospective(&normalized, resolver)
                    || (normalized.ends_with('걸')
                        && is_sentence_end_after(haystack, candidate.verified.token.end)
                        && fused_copula::accepts_fused_explanatory(&normalized, resolver))
                    || fused_copula::accepts_omitted_copula(
                        &normalized,
                        resolver,
                        fused_copula::OmittedCopulaFollowing {
                            is_sentence_end: || {
                                is_sentence_end_after(haystack, candidate.verified.token.end)
                            },
                            has_quotative_head: || {
                                has_quotative_adnominal_head(
                                    haystack,
                                    candidate.consumed.end,
                                    resolver,
                                )
                            },
                            continues_enumeration,
                        },
                        || {
                            follows_predicate_adnominal(
                                haystack,
                                candidate.verified.token.start,
                                resolver,
                            )
                        },
                        |unit| {
                            context::previous_word(haystack, candidate.verified.token.start)
                                .is_some_and(|previous| {
                                    resolver.has_counted_unit_frame(&previous, unit)
                                })
                        },
                        |noun| {
                            context::previous_word_with_start(
                                haystack,
                                candidate.verified.token.start,
                            )
                            .is_some_and(|(start, previous)| {
                                resolver.has_quotative_adnominal_dependent_noun(&previous, noun)
                                    || (context::previous_word(haystack, start).is_some_and(
                                        |object| resolver.has_object_marked_nominal(&object),
                                    ) && resolver.has_adnominal_dependent_noun(&previous, noun))
                            })
                        },
                    )))
    }

    fn has_enumerated_copula(
        &self,
        haystack: &[u8],
        end: usize,
        word: &str,
        resolver: &ConstraintResolver,
    ) -> bool {
        if resolver.has_explicit_copula_after_nominal(word) {
            return true;
        }
        self.has_copula_nominalizer_particles(word, resolver)
            && context::next_word_on_line(haystack, end).is_some_and(|(_, auxiliary)| {
                auxiliary.chars().next().is_some_and(|initial| {
                    matches!(
                        kfind_morph::drop_final(initial).unwrap_or(initial),
                        '하' | '해'
                    )
                }) && resolver.supports_auxiliary_sequence(
                    &auxiliary,
                    kfind_morph::DEFAULT_LATTICE_NODE_LIMIT,
                )
            })
    }

    fn has_copula_nominalizer_particles(&self, word: &str, resolver: &ConstraintResolver) -> bool {
        word.rmatch_indices('기').any(|(start, ending)| {
            let (predicate, particles) = word.split_at(start + ending.len());
            !particles.is_empty()
                && resolver.has_explicit_copula_after_nominal(predicate)
                && self
                    .particle_verifier
                    .verify_auxiliary_exact("기", particles)
                    .is_some()
        })
    }

    pub(super) fn licensed_nominal_copula_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        resolver: &ConstraintResolver,
    ) -> Option<Range<usize>> {
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.verified.core.start != whole.start {
            return None;
        }
        self.nominal_copula_trailing(haystack, candidate, whole, resolver)
    }

    pub(super) fn nominal_copula_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        whole: Range<usize>,
        resolver: &ConstraintResolver,
    ) -> Option<Range<usize>> {
        if candidate.consumed.end < candidate.verified.core.end
            || candidate.consumed.end >= whole.end
        {
            return None;
        }
        let suffix = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
        let suffix = if is_nfc(suffix) {
            Cow::Borrowed(suffix)
        } else {
            Cow::Owned(suffix.nfc().collect::<String>())
        };
        let token = haystack
            .get(whole.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
        let token = if is_nfc(token) {
            Cow::Borrowed(token)
        } else {
            Cow::Owned(token.nfc().collect::<String>())
        };
        if resolver.has_whole_modifier(&token) {
            return None;
        }
        if kfind_morph::is_omitted_copula_ending(&suffix) {
            if (matches!(suffix.as_ref(), "게" | "네" | "네요")
                && !is_sentence_end_after(haystack, whole.end))
                || (suffix == "게" && resolver.has_exact_nominal_surface(&token))
            {
                return None;
            }
            let host = std::str::from_utf8(
                haystack.get(candidate.consumed.start..candidate.consumed.end)?,
            )
            .ok()?
            .nfc()
            .collect::<String>();
            if resolver.has_exact_action_predicate_surface(&host) {
                return None;
            }
        }
        let preceding = haystack
            .get(candidate.consumed.start..candidate.consumed.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|consumed| consumed.nfc().last());
        if !preceding.is_some_and(|preceding| {
            verify_copula_surface_after_nominal(preceding, &suffix)
                || suffix
                    .strip_suffix('요')
                    .filter(|form| form.ends_with("니다"))
                    .is_some_and(|form| verify_copula_surface_after_nominal(preceding, form))
        }) {
            return None;
        }
        Some(candidate.consumed.start..whole.end)
    }
}

fn has_directional_adverbial_frame(
    haystack: &[u8],
    at: usize,
    resolver: &ConstraintResolver,
) -> bool {
    let Some((start, adjacent)) = context::previous_word_with_start(haystack, at) else {
        return false;
    };
    let is_directional = |word: &str| word.ends_with('로') && resolver.has_nominal_oblique(word);
    context::previous_word(haystack, start).is_some_and(|previous| {
        (is_directional(&adjacent) && resolver.has_exact_adverb_surface(&previous))
            || (is_directional(&previous) && resolver.has_exact_adverb_surface(&adjacent))
    })
}
