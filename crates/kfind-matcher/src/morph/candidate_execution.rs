use std::borrow::Cow;
use std::ops::Range;

use kfind_data::DataFinePos;
use kfind_morph::{
    ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, ParticleVerifier, PredicateStemClass, RuleId,
    verify_copula_surface_after_nominal, verify_predicate_continuation,
    verify_predicate_continuation_with_prefinal_context,
};
use kfind_query::{
    AtomPlan, CandidateConsumption, CandidateLeftContext, CandidateProgram, CoreMapping, Origin,
    VerifiedSpan,
};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::AnchorHit;
use crate::boundary::{
    is_boundary_after, is_boundary_before, is_sentence_end_after, surrounding_token_span,
};

use super::{
    ExecutedCandidate, MAX_CONSUMPTION_BYTES, MatchMetadata, MorphMatcher, StructuralCache,
    context, has_nominal_component_before_copula, source_aligned_compound_predicate_position,
    valid_utf8_prefix, valid_utf8_suffix,
};

impl MorphMatcher {
    pub(super) fn execute_program_with_metadata<'a>(
        &self,
        haystack: &'a [u8],
        hit: &AnchorHit,
        branch: &CandidateProgram,
        atom: &AtomPlan,
        metadata: MatchMetadata,
        structural_cache: &mut StructuralCache<'a>,
    ) -> Option<VerifiedSpan> {
        let mut candidate =
            self.execute_program_without_decision(haystack, hit, branch, metadata)?;
        if !self.accepts_program(haystack, &mut candidate, branch, atom, structural_cache) {
            return None;
        }
        let mut verified = candidate.verified;
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding) {
            let digits = haystack[hit.span.clone()]
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            verified.core.start += digits;
            verified.token.start += digits;
        }
        Some(verified)
    }

    pub(super) fn execute_program_without_decision(
        &self,
        haystack: &[u8],
        hit: &AnchorHit,
        branch: &CandidateProgram,
        metadata: MatchMetadata,
    ) -> Option<ExecutedCandidate> {
        let anchor = std::str::from_utf8(haystack.get(hit.span.clone())?).ok()?;
        let core = mapped_core(&hit.span, branch.core_mapping, anchor)?;
        let (consumed_bytes, suffix_rules) = match &branch.consumption {
            CandidateConsumption::Anchor
            | CandidateConsumption::CopulaHostEndingCompose
            | CandidateConsumption::FusedCopulaEnding => (0, Vec::new()),
            CandidateConsumption::PredicateContinuation {
                continuation,
                pos,
                source_positions,
                nominal_particle_transition,
                left_context,
                ..
            } => {
                if !accepts_left_context(left_context, haystack, hit.span.start) {
                    return None;
                }
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (consumption_anchor, consumption_following) =
                    normalized_consumption_text(anchor, following);
                let matched = verify_predicate_continuation_with_prefinal_context(
                    *continuation,
                    *pos,
                    &consumption_anchor,
                    &consumption_following,
                    || {
                        !branch.decision.is_structural()
                            || is_boundary_before(haystack, core.start)
                            || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                let whole = surrounding_token_span(haystack, core.clone());
                                if *pos == kfind_morph::PredicatePos::Copula {
                                    return context::normalized_token(
                                        haystack,
                                        whole.start..core.start,
                                    )
                                    .is_some_and(|host| {
                                        resolver.has_complete_nominal_surface(&host)
                                    });
                                }
                                source_aligned_compound_predicate_position(
                                    haystack,
                                    &core,
                                    &whole,
                                    *source_positions,
                                    resolver,
                                )
                            })
                    },
                )?;
                if *pos == kfind_morph::PredicatePos::Copula
                    && *continuation == kfind_morph::ContinuationState::StemEnding
                    && matched.rule_path.iter().any(|rule| {
                        rule.as_str().starts_with("ending.copula-")
                            || rule.as_str().starts_with("ending.hao-")
                    })
                    && is_boundary_before(haystack, hit.span.start)
                {
                    return None;
                }
                let mut normalized_consumed = matched.consumed_bytes;
                let mut rule_path = matched.rule_path;
                if consumption_following[normalized_consumed..]
                    .strip_prefix('도')
                    .is_some_and(|tail| !tail.starts_with(crate::is_token_character))
                    && rule_path.last().map_or_else(
                        || {
                            !branch.origins.is_empty()
                                && branch.origins.iter().all(|origin| {
                                    origin.rule_path.last().is_some_and(|rule| {
                                        rule.as_str() == "ending.simultaneous-myeonseo"
                                    })
                                })
                        },
                        |rule| rule.as_str() == "ending.simultaneous-myeonseo",
                    )
                {
                    normalized_consumed += '도'.len_utf8();
                    rule_path.push(RuleId::from("particle.additive"));
                }
                if !branch.consumption.allows_rule_path(&rule_path) {
                    return None;
                }
                let remaining = &consumption_following[normalized_consumed..];
                if let Some(resolver) = self.constraint_resolver.as_ref()
                    && matches!(
                        rule_path.as_slice(),
                        [.., connective, particle]
                            if connective.as_str() == "ending.connective-go"
                                && matches!(particle.as_str(), "particle.topic" | "particle.additive")
                    )
                {
                    let suffix = &consumption_following[..normalized_consumed];
                    if let Some((particle_start, _)) = suffix.char_indices().next_back()
                        && suffix[..particle_start].ends_with('고')
                    {
                        let token = format!("{consumption_anchor}{suffix}");
                        let ending_len = consumption_anchor.len() + particle_start;
                        let core_len = if is_nfc(anchor) {
                            core.len()
                        } else {
                            anchor.get(..core.len())?.nfc().map(char::len_utf8).sum()
                        };
                        if !source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_particle_path(
                                &token,
                                core_len,
                                ending_len,
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        }) {
                            return None;
                        }
                    }
                }
                if let Some(tail) = remaining.strip_prefix("부터")
                    && rule_path.last().map_or_else(
                        || {
                            !branch.origins.is_empty()
                                && branch.origins.iter().all(|origin| {
                                    origin.rule_path.last().is_some_and(|rule| {
                                        rule.as_str() == "ending.simultaneous-myeonseo"
                                    })
                                })
                        },
                        |rule| rule.as_str() == "ending.simultaneous-myeonseo",
                    )
                {
                    let end = tail
                        .find(|character| !crate::is_token_character(character))
                        .unwrap_or(tail.len());
                    let ending = &tail[..end];
                    if matches!(ending, "" | "다" | "이다") {
                        normalized_consumed += "부터".len() + end;
                        rule_path.push(RuleId::from("particle.from"));
                        if !ending.is_empty() {
                            rule_path.push(RuleId::from("ending.final-da"));
                        }
                    }
                }
                if let Some(host) = continuation_nominal_host(
                    &rule_path,
                    &consumption_following[..normalized_consumed],
                )
                .or_else(|| nominal_particle_transition.then_some(consumption_anchor.as_ref()))
                {
                    let particle = self
                        .particle_verifier
                        .verify_prefix(host, &consumption_following[normalized_consumed..]);
                    normalized_consumed =
                        normalized_consumed.checked_add(particle.consumed_bytes)?;
                    rule_path.extend(particle.rule_path);
                }
                let consumed_bytes = match &consumption_following {
                    Cow::Borrowed(_) => normalized_consumed,
                    Cow::Owned(normalized) => {
                        map_normalized_prefix(following, normalized, normalized_consumed)?
                    }
                };
                if matches!(
                    rule_path.as_slice(),
                    [.., connective, particle]
                        if connective.as_str() == "ending.connective-ji"
                            && matches!(particle.as_str(), "particle.topic" | "particle.object" | "particle.additive")
                ) {
                    let end = hit.span.end + consumed_bytes;
                    if !context::next_word(haystack, end).is_some_and(|next| {
                        has_negative_auxiliary(&next, *pos, self.constraint_resolver.as_ref())
                    }) {
                        return None;
                    }
                }
                (consumed_bytes, rule_path)
            }
            CandidateConsumption::StructuralPredicateEnding {
                pos,
                source_positions,
                flags,
                base_state,
                validate_anchor,
                stem_class,
                ..
            } => {
                let resolver = self.constraint_resolver.as_ref()?;
                let whole = surrounding_token_span(haystack, hit.span.clone());
                if whole.start != hit.span.start
                    || whole.end <= hit.span.end
                    || whole.len() > MAX_CONSUMPTION_BYTES
                {
                    return None;
                }
                let token = std::str::from_utf8(haystack.get(whole.clone())?).ok()?;
                let normalized_token = token.nfc().collect::<String>();
                if *pos == kfind_morph::PredicatePos::Copula
                    && resolver.has_exact_pronoun_case_surface(&normalized_token)
                    && !resolver.has_exact_predicate_ending_path(&normalized_token, *pos)
                {
                    return None;
                }
                let normalized_anchor = anchor.nfc().collect::<String>();
                let normalized_core_len = std::str::from_utf8(haystack.get(core.clone())?)
                    .ok()?
                    .nfc()
                    .collect::<String>()
                    .len();
                let suffix = normalized_token.get(normalized_anchor.len()..)?;
                // This fallback starts at the token boundary. A causal copula needs a host.
                if *pos == kfind_morph::PredicatePos::Copula && suffix.ends_with("므로") {
                    return None;
                }
                let ending_path = branch.consumption.allows_structural_suffix(suffix)
                    && stem_accepts_ending(
                        *pos,
                        *flags,
                        *base_state,
                        *stem_class,
                        &normalized_anchor,
                        suffix,
                    )
                    && if *validate_anchor {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_path(
                                &normalized_token,
                                normalized_anchor.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    } else {
                        resolver.supports_ending_suffix_path(
                            &normalized_token,
                            normalized_anchor.len(),
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    };
                let auxiliary_path = normalized_anchor.ends_with(['아', '어', '여'])
                    && resolver.auxiliary_splits(suffix).into_iter().any(|split| {
                        split == suffix.len()
                            || suffix.get(split..).is_some_and(|ending| {
                                branch.consumption.allows_structural_suffix(ending)
                            })
                    });
                if (!ending_path && !auxiliary_path)
                    || (!auxiliary_path
                        && source_positions.iter().any(|source_pos| {
                            resolver.whole_predicate_conflicts(
                                &normalized_token,
                                normalized_core_len,
                                source_pos,
                            )
                        }))
                {
                    return None;
                }
                (
                    whole.end.checked_sub(hit.span.end)?,
                    vec![RuleId::from("structural.ending-path")],
                )
            }
            CandidateConsumption::NominalParticleChain {
                allowed_rule_ids, ..
            }
            | CandidateConsumption::NominalCopulaEndingChain {
                allowed_rule_ids, ..
            } => 'nominal: {
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (consumption_anchor, consumption_following) =
                    normalized_consumption_text(anchor, following);
                let adverb_host = *allowed_rule_ids == self.plan.auxiliary_particle_rules;
                if adverb_host
                    && branch.origins.iter().any(|origin| {
                        origin
                            .rule_path
                            .iter()
                            .any(|rule| rule.as_str() == "structural.adverb-copula-nieun")
                    })
                {
                    if !is_boundary_before(haystack, hit.span.start) {
                        return None;
                    }
                    let tail = consumption_following
                        .split(|character: char| !crate::is_token_character(character))
                        .next()
                        .filter(|tail| !tail.is_empty())?;
                    let host = kfind_morph::drop_final(consumption_anchor.chars().next_back()?)?;
                    if !verify_copula_surface_after_nominal(host, &format!("인{tail}")) {
                        return None;
                    }
                    let consumed_bytes = match &consumption_following {
                        Cow::Borrowed(_) => tail.len(),
                        Cow::Owned(normalized) => {
                            map_normalized_prefix(following, normalized, tail.len())?
                        }
                    };
                    break 'nominal (
                        consumed_bytes,
                        vec![RuleId::from("structural.adverb-copula")],
                    );
                }
                if adverb_host
                    && consumption_following.starts_with(['이', '인', '일', '임', '입', '였'])
                    && is_boundary_before(haystack, hit.span.start)
                    && let Some(suffix) = consumption_following
                        .split(|character: char| !crate::is_token_character(character))
                        .next()
                    && consumption_anchor
                        .chars()
                        .next_back()
                        .is_some_and(|host| verify_copula_surface_after_nominal(host, suffix))
                {
                    let consumed_bytes = match &consumption_following {
                        Cow::Borrowed(_) => suffix.len(),
                        Cow::Owned(normalized) => {
                            map_normalized_prefix(following, normalized, suffix.len())?
                        }
                    };
                    let token_end = hit.span.end.checked_add(consumed_bytes)?;
                    if !self.constraint_resolver.as_ref().is_some_and(|resolver| {
                        context::normalized_token(haystack, hit.span.start..token_end)
                            .is_some_and(|word| resolver.has_whole_modifier(&word))
                    }) {
                        break 'nominal (
                            consumed_bytes,
                            vec![RuleId::from("structural.adverb-copula")],
                        );
                    }
                }
                let mut matched = if adverb_host {
                    // A trailing jamo can belong to the last syllable of the adverb.
                    if matches!(&consumption_anchor, Cow::Owned(_))
                        && consumption_anchor
                            .chars()
                            .next_back()
                            .zip(consumption_following.chars().next())
                            .is_some_and(|(last, next)| {
                                unicode_normalization::char::compose(last, next).is_some()
                            })
                    {
                        return None;
                    }
                    self.particle_verifier
                        .verify_non_nominal_prefix(&consumption_anchor, &consumption_following)
                } else {
                    self.particle_verifier
                        .verify_prefix(&consumption_anchor, &consumption_following)
                };
                let mut consumed_name_suffix = false;
                if !adverb_host
                    && matches!(
                        branch.consumption,
                        CandidateConsumption::NominalParticleChain { .. }
                    )
                    && let Some(tail) = consumption_following.strip_prefix('이')
                    && tail.starts_with(crate::is_token_character)
                    && consumption_anchor
                        .chars()
                        .next_back()
                        .is_some_and(|last| kfind_morph::drop_final(last).is_some())
                    && is_boundary_before(haystack, core.start)
                    && let Some(resolver) = self.constraint_resolver.as_ref()
                    && resolver.has_exact_proper_name_surface(&consumption_anchor)
                {
                    let name = format!("{consumption_anchor}이");
                    if !resolver.has_exact_nominal_surface(&name) {
                        let with_suffix = self.particle_verifier.verify_prefix(&name, tail);
                        let consumed = '이'.len_utf8() + with_suffix.consumed_bytes;
                        if consumed > matched.consumed_bytes
                            && with_suffix.consumed_bytes > 0
                            && !tail[with_suffix.consumed_bytes..]
                                .starts_with(crate::is_token_character)
                            && branch.consumption.allows_rule_path(&with_suffix.rule_path)
                        {
                            matched = with_suffix;
                            matched.consumed_bytes = consumed;
                            consumed_name_suffix = true;
                        }
                    }
                }
                if let Some((prefix, trailing)) =
                    consumption_following.split_at_checked(matched.consumed_bytes)
                    && trailing
                        .chars()
                        .next()
                        .is_some_and(crate::is_token_character)
                    && let Some(uncontracted_prefix) =
                        ParticleVerifier::before_contracted_case_topic(prefix, &matched.rule_path)
                {
                    matched = self
                        .particle_verifier
                        .verify_prefix(&consumption_anchor, uncontracted_prefix);
                }
                if matches!(
                    branch.consumption,
                    CandidateConsumption::NominalParticleChain { .. }
                ) && consumption_following.starts_with('서')
                    && matched
                        .rule_path
                        .first()
                        .is_some_and(|rule| rule.as_str() == "particle.source")
                {
                    let tail = &consumption_following[matched.consumed_bytes..];
                    let competing_lexeme = branch.decision.is_structural()
                        && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                            let whole = surrounding_token_span(haystack, core.clone());
                            context::normalized_token(haystack, whole).is_some_and(|token| {
                                let aligned_pronoun =
                                    branch.structural_patterns().iter().any(|pattern| {
                                        pattern.fine_pos == DataFinePos::Np
                                            && resolver
                                                .has_complete_nominal_surface(&consumption_anchor)
                                    });
                                resolver.has_exact_predicate_ending_path(
                                    &token,
                                    kfind_morph::PredicatePos::Verb,
                                ) || (!aligned_pronoun
                                    && (resolver.has_whole_modifier(&token)
                                        || resolver.has_exact_nominal_surface(&token)
                                        || token.strip_suffix('의').is_some_and(|host| {
                                            resolver.has_exact_nominal_surface(host)
                                        })))
                            })
                        });
                    if competing_lexeme || tail.starts_with(crate::is_token_character) {
                        // An incomplete shortened case chain can also start a compound component.
                        matched = self
                            .particle_verifier
                            .verify_prefix(&consumption_anchor, "");
                    }
                }
                if !branch.consumption.allows_rule_path(&matched.rule_path) {
                    return None;
                }

                if let [direction, genitive, ..] = matched.rule_path.as_slice()
                    && direction.as_str() == "particle.direction"
                    && genitive.as_str() == "particle.genitive"
                    && let Some(direction_surface) = ["으로", "로"]
                        .into_iter()
                        .find(|form| consumption_following.starts_with(form))
                {
                    let accepts = branch.decision.is_structural()
                        && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                            let host = format!("{consumption_anchor}{direction_surface}");
                            !resolver.has_exact_nominal_surface(&host)
                                && !resolver.has_whole_modifier(&host)
                        });
                    if !accepts {
                        let prefix = if branch.decision.is_structural() {
                            ""
                        } else {
                            direction_surface
                        };
                        matched = self
                            .particle_verifier
                            .verify_prefix(&consumption_anchor, prefix);
                    }
                }

                let subject_starts_copula = branch.decision.is_structural()
                    && matches!(
                        branch.consumption,
                        CandidateConsumption::NominalParticleChain { .. }
                    )
                    && matches!(matched.rule_path.as_slice(), [rule] if rule.as_str() == "particle.subject")
                    && consumption_following.get(..matched.consumed_bytes) == Some("이")
                    && consumption_following[matched.consumed_bytes..]
                        .chars()
                        .next()
                        .is_some_and(crate::is_token_character)
                    && core == hit.span
                    && consumption_anchor
                        .chars()
                        .next_back()
                        .is_some_and(|preceding| {
                            let whole = surrounding_token_span(haystack, core.clone());
                            (whole.start == core.start
                                || (haystack[whole.start..core.start]
                                    .iter()
                                    .all(u8::is_ascii_digit)
                                    && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                        context::normalized_token(haystack, whole.start..core.end)
                                            .is_some_and(|host| {
                                                resolver.has_numeric_unit_surface(&host)
                                            })
                                    }))
                                || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                    has_nominal_component_before_copula(
                                        haystack,
                                        &whole,
                                        &core,
                                        branch.structural_patterns(),
                                        resolver,
                                    )
                                }))
                                && context::normalized_token(haystack, core.end..whole.end)
                                    .is_some_and(|ending| {
                                        verify_copula_surface_after_nominal(preceding, &ending)
                                    })
                        });
                if subject_starts_copula {
                    (0, Vec::new())
                } else {
                    let consumed_bytes = match &consumption_following {
                        Cow::Borrowed(_) => matched.consumed_bytes,
                        Cow::Owned(normalized) => {
                            map_normalized_prefix(following, normalized, matched.consumed_bytes)?
                        }
                    };
                    if consumed_name_suffix {
                        matched
                            .rule_path
                            .insert(0, RuleId::from("suffix.proper-name-i"));
                    }
                    (consumed_bytes, matched.rule_path)
                }
            }
            CandidateConsumption::DirectParticleHost { rule_id } => {
                if requires_direct_particle_host(branch)
                    && !self.accepts_direct_particle(haystack, &hit.span, rule_id)
                {
                    return None;
                }
                (0, Vec::new())
            }
        };
        let token_end = hit.span.end.checked_add(consumed_bytes)?;
        if token_end > haystack.len() || !is_utf8_boundary(haystack, token_end) {
            return None;
        }
        if suffix_rules.last().is_some_and(|rule| match rule.as_str() {
            "ending.copula-ya"
            | "ending.copula-ge"
            | "ending.final-ge"
            | "ending.rhetorical-ryeogo" => !is_sentence_end_after(haystack, token_end),
            "ending.interrogative-ni" => {
                branch.boundary().require_right && !is_boundary_after(haystack, token_end)
            }
            _ => false,
        }) {
            return None;
        }
        let token = hit.span.start..token_end;
        let origins = match metadata {
            MatchMetadata::SpanOnly => Vec::new(),
            MatchMetadata::Provenance => extend_origins(&branch.origins, &suffix_rules),
        };
        Some(ExecutedCandidate {
            anchor: hit.span.clone(),
            consumed: token.clone(),
            suffix_rules,
            verified: VerifiedSpan {
                core,
                token,
                origins,
            },
        })
    }
}

fn stem_accepts_ending(
    pos: kfind_morph::PredicatePos,
    flags: kfind_morph::PredicateFlags,
    base_state: kfind_morph::ContinuationState,
    class: PredicateStemClass,
    anchor: &str,
    suffix: &str,
) -> bool {
    if suffix.is_empty() {
        return false;
    }
    let Some(first) = suffix.chars().next() else {
        return false;
    };
    if kfind_morph::decompose_syllable(first).is_some_and(|syllable| syllable.choseong == 11) {
        return false;
    }
    if matches!(suffix.chars().next(), Some('이' | '인' | '일' | '임')) {
        return false;
    }
    if !pos.is_action()
        && ["거라", "고자", "느냐", "너라", "려", "자"]
            .iter()
            .any(|prefix| suffix.starts_with(prefix))
    {
        return false;
    }
    if pos.is_action()
        && suffix == "답시고"
        && !matches!(
            base_state,
            kfind_morph::ContinuationState::Past | kfind_morph::ContinuationState::Future
        )
    {
        return false;
    }
    let declarative_continuation = ["다면", "다며", "다면서", "다니", "다는데", "다지"]
        .iter()
        .any(|prefix| suffix.starts_with(prefix));
    if ((pos.is_action()
        && !matches!(
            base_state,
            kfind_morph::ContinuationState::Past | kfind_morph::ContinuationState::Future
        ))
        || flags.contains(kfind_morph::PredicateFlags::NO_DECLARATIVE_CONTINUATION))
        && declarative_continuation
    {
        return false;
    }
    if suffix.starts_with("너라") && !anchor.ends_with('오') {
        return false;
    }
    if class != PredicateStemClass::Consonant
        && matches!(suffix, "습니다" | "습니까" | "습디까" | "습디다")
    {
        return false;
    }
    if class == PredicateStemClass::Consonant
        && [
            "니", "니까", "니깐", "라", "며", "면서", "면", "려", "리", "세", "셔", "시", "십",
        ]
        .iter()
        .any(|prefix| suffix.starts_with(prefix))
    {
        return false;
    }
    if class == PredicateStemClass::Rieul
        && ["느", "는", "니", "세", "셔", "시", "십"]
            .iter()
            .any(|prefix| suffix.starts_with(prefix))
    {
        return false;
    }
    true
}

fn normalized_consumption_text<'a>(
    anchor: &'a str,
    following: &'a str,
) -> (Cow<'a, str>, Cow<'a, str>) {
    if is_nfc(anchor) {
        return (Cow::Borrowed(anchor), Cow::Borrowed(following));
    }
    (
        Cow::Owned(anchor.nfc().collect()),
        Cow::Owned(following.nfc().collect()),
    )
}

fn requires_direct_particle_host(branch: &CandidateProgram) -> bool {
    let boundary = branch.boundary();
    !boundary.require_left && boundary.require_right
}

fn has_negative_auxiliary(
    word: &str,
    host: kfind_morph::PredicatePos,
    resolver: Option<&ConstraintResolver>,
) -> bool {
    use kfind_morph::{ContinuationState, PredicatePos};
    let (anchor, state) = match word.chars().next() {
        Some('않') if word.starts_with("않았") => ("않았", ContinuationState::Past),
        Some('않') if word.starts_with("않아") => ("않아", ContinuationState::AOrEo),
        Some('않') => ("않", ContinuationState::StemEnding),
        Some('못') => match word.chars().nth(1) {
            Some('하') => ("못하", ContinuationState::StemEnding),
            Some('했') => ("못했", ContinuationState::Past),
            Some('해') => ("못해", ContinuationState::AOrEo),
            _ => return false,
        },
        Some('마') if host.is_action() => {
            return matches!(word, "마" | "마요" | "마라" | "마세요" | "마십시오");
        }
        Some('말') if host.is_action() => ("말", ContinuationState::StemEnding),
        _ => return false,
    };
    let suffix = &word[anchor.len()..];
    let (anchor, suffix, state) = match (state, suffix.strip_prefix('는')) {
        (ContinuationState::StemEnding, Some(tail)) if tail.starts_with('데') => (
            &word[..anchor.len() + '는'.len_utf8()],
            tail,
            ContinuationState::NieunEnding,
        ),
        _ => (anchor, suffix, state),
    };
    (state == ContinuationState::AOrEo || !suffix.is_empty())
        && (verify_predicate_continuation(state, PredicatePos::AuxiliaryVerb, anchor, suffix)
            .is_some_and(|matched| matched.token_end == word.len())
            || resolver.is_some_and(|resolver| {
                !resolver.has_subject_marked_nominal(word)
                    && resolver.supports_ending_suffix_path(
                        word,
                        anchor.len(),
                        DEFAULT_LATTICE_NODE_LIMIT,
                    )
            }))
}

fn accepts_left_context(
    left_context: &CandidateLeftContext,
    haystack: &[u8],
    anchor_start: usize,
) -> bool {
    match left_context {
        CandidateLeftContext::Any => true,
        CandidateLeftContext::ContractedAfterVowel {
            uncontracted_prefix,
        } => {
            let Some(left_bytes) = haystack.get(..anchor_start) else {
                return false;
            };
            let left = valid_utf8_suffix(left_bytes);
            let normalized_left = left.nfc().collect::<String>();
            let normalized_prefix = uncontracted_prefix.nfc().collect::<String>();
            accepts_contracted_after_vowel(&normalized_left, &normalized_prefix)
        }
    }
}

fn accepts_contracted_after_vowel(left: &str, uncontracted_prefix: &str) -> bool {
    let Some(previous) = left.chars().next_back() else {
        return false;
    };
    if kfind_morph::has_final(previous) {
        return false;
    }

    left.strip_suffix(uncontracted_prefix)
        .and_then(|host| host.chars().next_back())
        .is_none_or(|host_final| !kfind_morph::has_final(host_final))
}

fn mapped_core(
    anchor_span: &Range<usize>,
    mapping: CoreMapping,
    anchor: &str,
) -> Option<Range<usize>> {
    let length = match mapping {
        CoreMapping::WholeAnchor => anchor.len(),
        CoreMapping::PrefixBytes(length) => length,
    };
    if length == 0 || length > anchor.len() || !anchor.is_char_boundary(length) {
        return None;
    }
    Some(anchor_span.start..anchor_span.start + length)
}

fn map_normalized_prefix(
    original: &str,
    normalized: &str,
    normalized_bytes: usize,
) -> Option<usize> {
    if normalized_bytes == 0 {
        return Some(0);
    }
    let expected = normalized.get(..normalized_bytes)?;
    original
        .char_indices()
        .map(|(start, character)| start + character.len_utf8())
        .find(|&end| original[..end].nfc().eq(expected.chars()))
}

fn is_utf8_boundary(bytes: &[u8], at: usize) -> bool {
    at <= bytes.len() && (at == 0 || at == bytes.len() || bytes[at] & 0b1100_0000 != 0b1000_0000)
}

fn extend_origins(origins: &[Origin], suffix_rules: &[RuleId]) -> Vec<Origin> {
    let mut extended = origins
        .iter()
        .map(|origin| {
            let mut rule_path = origin.rule_path.clone();
            rule_path.extend_from_slice(suffix_rules);
            Origin {
                analysis_index: origin.analysis_index,
                rule_path,
            }
        })
        .collect::<Vec<_>>();
    extended.sort();
    extended.dedup();
    extended
}

fn continuation_nominal_host<'a>(rule_path: &[RuleId], consumed: &'a str) -> Option<&'a str> {
    let nominalized = matches!(
        rule_path.last().map(RuleId::as_str),
        Some("ending.nominalizer" | "ending.nominalizer-gi")
    );
    nominalized
        .then(|| {
            consumed
                .char_indices()
                .next_back()
                .map(|(start, _)| &consumed[start..])
        })
        .flatten()
}
