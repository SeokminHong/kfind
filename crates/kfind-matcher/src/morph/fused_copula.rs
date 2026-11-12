use std::borrow::Cow;
use std::iter::Peekable;

use kfind_morph::hangul::{JONG_BIEUP, JONG_NIEUN, JONG_RIEUL};
use kfind_morph::{decompose_syllable, is_omitted_copula_ra_ending};
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
    if omitted_ra_hosts(&normalized).next().is_some() {
        return true;
    }
    token.char_indices().rev().any(|(start, character)| {
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
        let suffix = &token[start + character.len_utf8()..];
        if suffix.is_empty() {
            return false;
        }
        let suffix = if normalization == NormalizationMode::Canonical && !is_nfc(suffix) {
            Cow::Owned(suffix.nfc().collect::<String>())
        } else {
            Cow::Borrowed(suffix)
        };
        is_coda_initial_ending(coda, &suffix)
    })
}

pub(super) fn omitted_ra_hosts(token: &str) -> impl Iterator<Item = (&str, &str)> {
    token.rfind('라').into_iter().flat_map(move |start| {
        let (host, suffix) = token.split_at(start);
        [Some(host), host.strip_suffix('시')]
            .into_iter()
            .flatten()
            .filter(move |host| {
                is_omitted_copula_ra_ending(suffix)
                    && host.chars().next_back().is_some_and(|last| {
                        decompose_syllable(last).is_some_and(|syllable| syllable.jongseong == 0)
                    })
            })
            .map(move |host| (host, suffix))
    })
}
