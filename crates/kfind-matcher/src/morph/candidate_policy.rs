use kfind_data::DataFinePos;
use kfind_morph::{ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, verify_predicate_continuation};
use kfind_query::{
    AtomPlan, BoundaryPolicy, CandidateConsumption, CandidateDecision, CandidateProgram,
    QueryComposition,
};
use unicode_normalization::UnicodeNormalization;

use crate::boundary::{is_boundary_after, is_boundary_before, surrounding_token_span};

use super::{
    ExecutedCandidate, MorphMatcher, StructuralCache, context, has_quotative_adnominal_head,
};

mod competing_analyses;

impl MorphMatcher {
    pub(super) fn accepts_program<'a>(
        &self,
        haystack: &'a [u8],
        candidate: &mut ExecutedCandidate,
        branch: &CandidateProgram,
        atom: &AtomPlan,
        structural_cache: &mut StructuralCache<'a>,
    ) -> bool {
        let boundary = atom.boundary;
        if boundary == BoundaryPolicy::Smart
            && candidate
                .suffix_rules
                .last()
                .or_else(|| {
                    let (first, rest) = branch.origins.split_first()?;
                    let rule = first.rule_path.last()?;
                    rest.iter()
                        .all(|origin| origin.rule_path.last() == Some(rule))
                        .then_some(rule)
                })
                .is_some_and(|rule| rule.as_str() == "ending.archaic-honorific-sa")
            && (!is_boundary_after(haystack, candidate.consumed.end)
                || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                    let span = &candidate.consumed;
                    context::normalized_token(haystack, span.clone()).is_some_and(|word| {
                        resolver.has_exact_nominal_surface(&word)
                            && context::next_word_on_line(haystack, span.end).is_some_and(
                                |(_, next)| resolver.has_nominal_with_optional_particles(&next),
                            )
                    })
                }))
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && candidate
                .suffix_rules
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.directive-ram")
            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                let token = &candidate.consumed;
                context::normalized_token(haystack, token.clone())
                    .is_some_and(|word| resolver.has_exact_nominal_surface(&word))
                    && context::next_word_on_line(haystack, token.end).is_some_and(|(end, next)| {
                        resolver.has_nominal_with_optional_particles(&next)
                            || (resolver.has_predicate_adnominal_surface(&next)
                                && context::next_word_on_line(haystack, end).is_some_and(
                                    |(_, head)| resolver.has_nominal_with_optional_particles(&head),
                                ))
                    })
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && candidate
                .suffix_rules
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.propositive-se")
            && !is_boundary_before(haystack, candidate.verified.core.start)
            && !self.constraint_resolver.as_ref().is_some_and(|resolver| {
                context::normalized_token(
                    haystack,
                    surrounding_token_span(haystack, candidate.verified.core.clone()),
                )
                .is_some_and(|token| {
                    resolver
                        .has_complete_attached_auxiliary_path(&token, DEFAULT_LATTICE_NODE_LIMIT)
                })
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && candidate.suffix_rules.iter().any(|rule| {
                matches!(
                    rule.as_str(),
                    "ending.concessive-nieundeul" | "ending.counterfactual-deondeul"
                )
            })
            && !branch
                .structural_patterns()
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Vcp)
            && (!is_boundary_before(haystack, candidate.verified.core.start)
                || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                    context::normalized_token(haystack, candidate.consumed.clone()).is_some_and(
                        |word| {
                            let plural_nominal = word.strip_suffix('들').is_some_and(|host| {
                                resolver.has_exact_nominal_surface(host)
                                    && !resolver.has_predicate_adnominal_surface(host)
                                    && !resolver.has_exact_predicate_connective_path(
                                        &word,
                                        kfind_morph::PredicatePosSet::one(
                                            kfind_morph::PredicatePos::Verb,
                                        ) | kfind_morph::PredicatePosSet::one(
                                            kfind_morph::PredicatePos::Adjective,
                                        ),
                                    )
                            });
                            plural_nominal
                                || (resolver.has_exact_non_copula_predicate_surface(&word)
                                    && context::next_word(haystack, candidate.consumed.end)
                                        .is_some_and(|next| {
                                            resolver.has_adnominal_dependent_noun(&word, &next)
                                                || (resolver.has_predicate_adnominal_surface(&word)
                                                    && resolver.has_independent_nominal_head(&next))
                                        }))
                        },
                    )
                }))
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && !is_boundary_before(haystack, candidate.verified.core.start)
            && !is_boundary_after(haystack, candidate.consumed.end)
            && branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == "ending.connective-doe")
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && !branch.origins.is_empty()
            && branch.origins.iter().all(|origin| {
                origin.rule_path.last().is_some_and(|rule| {
                    matches!(
                        rule.as_str(),
                        "ending.prohibitive-short" | "ending.prohibitive-short-polite"
                    )
                })
            })
        {
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::previous_word(haystack, candidate.verified.core.start).is_some_and(
                    |previous| {
                        previous.ends_with('지')
                            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                resolver.has_negative_connective_host(&previous)
                            })
                    },
                );
        }
        if boundary == BoundaryPolicy::Smart
            && let Some(resolver) = self.constraint_resolver.as_ref()
            && let Some(accepts) = decide_request_dalla(haystack, candidate, branch, atom, resolver)
        {
            return accepts;
        }
        if boundary == BoundaryPolicy::Smart
            && matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation {
                    pos: kfind_morph::PredicatePos::Copula,
                    ..
                }
            )
            && is_boundary_before(haystack, candidate.verified.core.start)
            && candidate
                .suffix_rules
                .iter()
                .any(|rule| rule.as_str() == "ending.connective-meuro")
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation {
                    continuation: kfind_morph::ContinuationState::NieunEnding,
                    ..
                }
            )
            && candidate
                .suffix_rules
                .iter()
                .any(|rule| rule.as_str() == "ending.connective-neunde")
            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                rejects_background_conjunction(haystack, candidate, resolver)
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                competing_analyses::rejects(haystack, candidate, branch, atom, resolver)
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
            && let Some(resolver) = self.constraint_resolver.as_ref()
            && has_dependent_quotative_frame(haystack, candidate, resolver)
        {
            candidate.consumed.end = candidate.verified.core.end;
            candidate.verified.token.end = candidate.verified.core.end;
            if !candidate.verified.origins.is_empty() {
                candidate.verified.origins.clone_from(&branch.origins);
            }
            candidate.suffix_rules.clear();
        }
        let accepted = match &branch.decision {
            CandidateDecision::Boundary(_) => {
                self.accepts_token_boundary(haystack, &candidate.verified, branch)
                    || self.accepts_boundary_adnominal_interrogative(haystack, candidate, branch)
            }
            CandidateDecision::Structural(_) => {
                self.accepts_structural(haystack, candidate, branch, structural_cache)
            }
        };
        accepted
            || (self.plan.composition == QueryComposition::Phrase
                && self.plan.atoms.len() > 1
                && self.accepts_joined_nominal_phrase_component(haystack, candidate, branch, atom))
    }
}

fn rejects_background_conjunction(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let consumed = &candidate.consumed;
    haystack.get(consumed.end) == Some(&b',')
        && haystack[..consumed.start]
            .iter()
            .rev()
            .copied()
            .find(|byte| !matches!(byte, b' ' | b'\t'))
            .is_none_or(|byte| matches!(byte, b'.' | b'!' | b'?' | b'\n' | b'\r'))
        && std::str::from_utf8(&haystack[consumed.clone()]).is_ok_and(|token| {
            resolver.has_exact_conjunctive_adverb(&token.nfc().collect::<String>())
        })
}

fn decide_request_dalla(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> Option<bool> {
    use kfind_morph::CoarsePos;

    let start = candidate.verified.core.start;
    let tail = haystack.get(start..)?;
    if !(tail.starts_with("달라".as_bytes()) || tail.starts_with("달라".as_bytes())) {
        return None;
    }
    let (first, rest) = branch.origins.split_first()?;
    let analysis = atom.analyses.get(usize::from(first.analysis_index))?;
    let is_request = match (analysis.lemma.as_ref(), analysis.coarse_pos) {
        ("달다", CoarsePos::Verb) => true,
        ("다르다", CoarsePos::Adjective) => false,
        _ => return None,
    };
    if rest.iter().any(|origin| {
        atom.analyses
            .get(usize::from(origin.analysis_index))
            .is_none_or(|other| {
                other.lemma != analysis.lemma || other.coarse_pos != analysis.coarse_pos
            })
    }) {
        return None;
    }

    let span = surrounding_token_span(haystack, candidate.verified.core.clone());
    let token = context::normalized_token(haystack, start..span.end)?;
    let request_quote = || {
        token.strip_prefix('달').is_some_and(|ending| {
            verifies_directive("달", ending, |rule| rule == "ending.directive-quotative")
        })
    };
    if span.start < start {
        return (!is_request
            && (token == "달라" || request_quote())
            && context::normalized_token(haystack, span.start..start)
                .is_some_and(|host| resolver.has_request_auxiliary_host(&host)))
        .then_some(false);
    }
    if token != "달라" {
        return (!is_request && request_quote()).then_some(false);
    }
    let next = context::next_word(haystack, span.end);
    if next
        .as_deref()
        .is_some_and(|word| resolver.has_boi_predicate(word))
    {
        return is_request.then_some(false);
    }
    if !is_request {
        let mut before = start;
        let context_start =
            start.saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
        while before > context_start {
            let (word_start, word) = context::previous_word_with_start(haystack, before)?;
            if word_start < context_start {
                break;
            }
            if resolver.has_object_marked_nominal(&word) {
                return Some(false);
            }
            if !resolver.has_exact_adverb_surface(&word) {
                break;
            }
            before = word_start;
        }
        return None;
    }
    if candidate.consumed == span
        && matches!(candidate.suffix_rules.as_slice(), [rule] if rule.as_str() == "ending.directive-ra")
        && next
            .as_deref()
            .and_then(|word| word.strip_prefix('하'))
            .is_some_and(|ending| {
                verifies_directive("하", ending, |rule| {
                    matches!(rule, "ending.directive-ra" | "ending.directive-quotative")
                })
            })
    {
        return Some(true);
    }
    None
}

fn verifies_directive(stem: &str, ending: &str, accepts_rule: impl FnOnce(&str) -> bool) -> bool {
    use kfind_morph::{ContinuationState, PredicatePos};

    verify_predicate_continuation(ContinuationState::Eu, PredicatePos::Verb, stem, ending)
        .is_some_and(|matched| {
            matched.consumed_bytes == ending.len()
                && matches!(matched.rule_path.as_slice(), [rule] if accepts_rule(rule.as_str()))
        })
}

fn has_dependent_quotative_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    matches!(candidate.suffix_rules.as_slice(), [rule] if rule.as_str() == "particle.topic.iran-ran")
        && is_boundary_before(haystack, core.start)
        && is_boundary_after(haystack, candidate.consumed.end)
        && context::normalized_token(haystack, core.clone()).is_some_and(|nominal| {
            context::previous_word(haystack, core.start)
                .is_some_and(|previous| resolver.has_adnominal_dependent_noun(&previous, &nominal))
        })
        && has_quotative_adnominal_head(haystack, candidate.consumed.end, resolver)
}
