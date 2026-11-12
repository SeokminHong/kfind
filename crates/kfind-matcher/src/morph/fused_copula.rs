use std::borrow::Cow;
use std::iter::Peekable;

use kfind_morph::hangul::{JONG_BIEUP, JONG_NIEUN, JONG_RIEUL};
use kfind_morph::{
    ConstraintResolver, decompose_syllable, drop_final, is_omitted_copula_ending,
    replace_last_final, starts_omitted_copula_ending,
};
use kfind_query::{NormalizationMode, is_coda_initial_ending};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::boundary::{is_boundary_before, is_token_character, next_character_with_end};
use crate::{AnchorHit, AnchorHits};

use super::{MAX_CONSUMPTION_BYTES, MorphMatcher};

pub(super) struct MorphAnchorHits<'matcher, 'text> {
    literal: Peekable<AnchorHits<'matcher, 'text>>,
    fused: Option<FusedCopulaHits<'text>>,
}

struct FusedCopulaHits<'text> {
    haystack: &'text [u8],
    cursor: usize,
    anchor_index: usize,
    normalization: NormalizationMode,
    pending: Option<AnchorHit>,
}

impl MorphMatcher {
    pub(super) fn anchor_hits<'matcher, 'text>(
        &'matcher self,
        haystack: &'text [u8],
        at: usize,
    ) -> MorphAnchorHits<'matcher, 'text> {
        MorphAnchorHits {
            literal: self.anchor_engine.hits(haystack, at).peekable(),
            fused: self
                .fused_copula_anchor
                .map(|anchor_index| FusedCopulaHits {
                    haystack,
                    cursor: at,
                    anchor_index,
                    normalization: self.plan.normalization,
                    pending: None,
                }),
        }
    }
}

impl Iterator for MorphAnchorHits<'_, '_> {
    type Item = AnchorHit;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(fused) = &mut self.fused {
            let before = self
                .literal
                .peek()
                .map_or(fused.haystack.len(), |hit| hit.span.end);
            if let Some(hit) = fused.next_through(before) {
                return Some(hit);
            }
        }
        self.literal.next()
    }
}

impl FusedCopulaHits<'_> {
    fn next_through(&mut self, limit: usize) -> Option<AnchorHit> {
        while self.pending.is_none() && self.cursor < limit {
            let start = self.cursor;
            let Some((next, character)) = next_character_with_end(self.haystack, start) else {
                self.cursor += 1;
                continue;
            };
            self.cursor = next;
            if !is_token_character(character) {
                continue;
            }
            while let Some((next, character)) = next_character_with_end(self.haystack, self.cursor)
                && is_token_character(character)
            {
                self.cursor = next;
            }
            if self.cursor - start <= MAX_CONSUMPTION_BYTES
                && is_boundary_before(self.haystack, start)
                && std::str::from_utf8(&self.haystack[start..self.cursor])
                    .is_ok_and(|token| has_fused_ending(token, self.normalization))
            {
                self.pending = Some(AnchorHit {
                    anchor_index: self.anchor_index,
                    span: start..self.cursor,
                });
            }
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|hit| hit.span.end <= limit)
        {
            return self.pending.take();
        }
        None
    }
}

fn has_fused_ending(token: &str, normalization: NormalizationMode) -> bool {
    let normalized = if normalization == NormalizationMode::Canonical && !is_nfc(token) {
        Cow::Owned(token.nfc().collect::<String>())
    } else {
        Cow::Borrowed(token)
    };
    normalized.char_indices().rev().any(|(start, character)| {
        if ((starts_omitted_copula_ending(character)
            && is_omitted_copula_ending(&normalized[start..]))
            || (character == '야' && start + character.len_utf8() == normalized.len()))
            && vowel_final(&normalized[..start])
        {
            return true;
        }
        let coda = match character {
            'ᆫ' => 'ㄴ',
            'ᆯ' => 'ㄹ',
            'ᆸ' => 'ㅂ',
            _ => match decompose_syllable(character).map(|syllable| syllable.jongseong) {
                Some(JONG_NIEUN) => 'ㄴ',
                Some(JONG_RIEUL) => 'ㄹ',
                Some(JONG_BIEUP) => 'ㅂ',
                _ => return false,
            },
        };
        let suffix = &normalized[start + character.len_utf8()..];
        !suffix.is_empty() && is_coda_initial_ending(coda, suffix)
    })
}

fn omitted_copula_hosts(token: &str) -> impl Iterator<Item = (&str, &str)> {
    token
        .rmatch_indices(starts_omitted_copula_ending)
        .filter(move |(start, _)| is_omitted_copula_ending(&token[*start..]))
        .flat_map(move |(start, _)| {
            let (host, suffix) = token.split_at(start);
            [Some(host), host.strip_suffix('시')]
                .into_iter()
                .flatten()
                .filter(|host| vowel_final(host))
                .map(move |host| (host, suffix))
        })
}

fn vowel_final(host: &str) -> bool {
    host.chars()
        .next_back()
        .and_then(decompose_syllable)
        .is_some_and(|syllable| syllable.jongseong == 0)
}

pub(super) fn accepts_omitted_copula(
    token: &str,
    resolver: &ConstraintResolver,
    is_sentence_end: impl Fn() -> bool,
    follows_adnominal: impl Fn() -> bool,
    supports_counted_unit: impl Fn(&str) -> bool,
) -> bool {
    let mut nominal_frame = false;
    let accepted = omitted_copula_hosts(token).any(|(host, suffix)| {
        if (matches!(suffix, "죠" | "지요" | "냐") && supports_counted_unit(host))
            || (resolver.has_numeric_unit_surface(host)
                && (!matches!(suffix, "게" | "네" | "네요") || is_sentence_end()))
        {
            nominal_frame = true;
            return true;
        }
        if matches!(suffix, "라면" | "라도" | "랴" | "렷다") {
            return !host
                .strip_suffix('이')
                .is_some_and(|prefix| resolver.has_complete_nominal_surface(prefix))
                && (resolver.has_exact_nominal_surface(host)
                    || (suffix == "라면" && resolver.has_copula_conditional_adverbial_host(host)));
        }
        if matches!(suffix, "라고" | "라고요" | "라는")
            && resolver.has_exact_nominal_surface(host)
            && resolver.has_exact_non_copula_predicate_surface(host)
            && follows_adnominal()
        {
            nominal_frame = true;
            return true;
        }
        if matches!(suffix, "예요" | "여요") {
            return !resolver.has_exact_non_copula_predicate_surface(host)
                && (resolver.has_exact_nominal_surface(host)
                    || host.strip_suffix('이').is_some_and(|name| {
                        name.chars().next_back().is_some_and(kfind_morph::has_final)
                            && resolver.has_exact_proper_name_surface(name)
                    }));
        }
        if matches!(suffix, "게" | "네" | "네요") && !is_sentence_end() {
            return false;
        }
        if suffix == "게"
            && (resolver.has_exact_nominal_surface(token) || resolver.has_whole_modifier(token))
        {
            return false;
        }
        !host
            .strip_suffix('이')
            .is_some_and(|prefix| resolver.has_complete_nominal_surface(prefix))
            && !resolver.has_exact_action_predicate_surface(host)
            && !(matches!(suffix, "라고" | "라고요" | "라는") && resolver.has_whole_modifier(host))
            && (resolver.has_exact_nominal_surface(host)
                || (matches!(suffix, "라고" | "라고요" | "라는")
                    && resolver.has_compound_noun_surface(host)
                    && !resolver.has_independent_final_predicate(host)
                    && !resolver.has_final_copula_after_nominal(host)))
            && !(matches!(
                suffix,
                "니" | "니까"
                    | "니까는"
                    | "니깐"
                    | "면"
                    | "면서"
                    | "네"
                    | "네요"
                    | "지요"
                    | "죠"
                    | "구나"
                    | "구려"
                    | "구먼"
                    | "군"
            ) && host
                .strip_suffix('시')
                .is_some_and(|stem| resolver.has_exact_non_copula_predicate_surface(stem)))
            && !(suffix == "래요"
                && token
                    .strip_suffix('요')
                    .is_some_and(|nominal| resolver.has_exact_nominal_surface(nominal)))
            && !(matches!(
                suffix,
                "라" | "니" | "니까" | "니까는" | "니깐" | "면" | "면서" | "네" | "네요"
            ) && host.char_indices().skip(1).any(|(split, _)| {
                resolver.has_exact_non_copula_predicate_surface(&token[..split])
                    && resolver.has_exact_ending_surface(&token[split..])
            }))
    });
    accepted
        && !resolver.has_whole_modifier(token)
        && !resolver.has_exact_nominal_surface(token)
        && (nominal_frame || !resolver.has_exact_non_copula_predicate_surface(token))
}

pub(super) fn accepts_fused_retrospective(token: &str, resolver: &ConstraintResolver) -> bool {
    let Some(stem) = token
        .strip_suffix("디까")
        .or_else(|| token.strip_suffix("디다"))
    else {
        return false;
    };
    let Some(last) = stem.chars().next_back() else {
        return false;
    };
    if !decompose_syllable(last).is_some_and(|syllable| syllable.jongseong == JONG_BIEUP)
        || resolver.has_exact_non_copula_predicate_surface(token)
        || resolver.has_exact_nominal_surface(token)
        || resolver.has_whole_modifier(token)
    {
        return false;
    }
    let Some(vowel) = drop_final(last) else {
        return false;
    };
    let mut host = stem[..stem.len() - last.len_utf8()].to_owned();
    host.push(vowel);
    if host
        .strip_suffix('이')
        .is_some_and(|nominal| resolver.has_complete_nominal_surface(nominal))
    {
        return false;
    }
    let sibling = format!("{stem}니다");
    if resolver.has_exact_lost_span_copula_ending_path(&sibling)
        && !resolver.has_exact_non_copula_predicate_surface(&sibling)
    {
        return true;
    }
    [Some(host.as_str()), host.strip_suffix('시')]
        .into_iter()
        .flatten()
        .any(|nominal| {
            resolver.has_exact_nominal_surface(nominal)
                && !resolver.has_exact_non_copula_predicate_surface(nominal)
                && !replace_last_final(nominal, JONG_RIEUL)
                    .is_some_and(|rieul| resolver.has_exact_non_copula_predicate_surface(&rieul))
        })
}
