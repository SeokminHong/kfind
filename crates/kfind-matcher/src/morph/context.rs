use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use kfind_morph::{
    BoundedTokenContext, CandidateSpans, ConstraintDecision, ConstraintResolver,
    PreparedStructuralContext, QueryMorphPattern,
};
use kfind_query::VerifiedSpan;
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::{AnalysisWindow, DEFAULT_ANALYSIS_WINDOW_LIMITS, is_token_character};

const MAX_PREPARED_CONTEXT_CACHE_ENTRIES: usize = 256;

#[derive(Debug)]
pub(super) struct PreparedStructuralContextAnalysis {
    current: AnalysisWindow,
    prepared: Arc<PreparedStructuralContext>,
}

#[derive(Default)]
pub(super) struct PreparedStructuralContextCache {
    fingerprint_builder: RandomState,
    entries: HashMap<u64, Vec<CachedPreparedContext>>,
    entry_count: usize,
}

struct CachedPreparedContext {
    raw_context: Box<[u8]>,
    current: Range<usize>,
    node_limit: usize,
    include_nominal_copula: bool,
    include_nominal_derivation_predicate: bool,
    prepared: Option<Arc<PreparedStructuralContext>>,
}

pub(super) struct StructuralRequest<'a> {
    pub(super) candidate: &'a VerifiedSpan,
    pub(super) anchor: Range<usize>,
    pub(super) consumed: Range<usize>,
    pub(super) patterns: &'a [QueryMorphPattern],
}

impl PreparedStructuralContextAnalysis {
    pub(super) fn extract(
        haystack: &[u8],
        candidate: Range<usize>,
        resolver: &ConstraintResolver,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
        prepared_cache: &mut PreparedStructuralContextCache,
    ) -> Option<Self> {
        let current =
            AnalysisWindow::extract(haystack, candidate, DEFAULT_ANALYSIS_WINDOW_LIMITS).ok()?;
        let current_span = current.raw_span();
        let previous_span = adjacent_token_span(
            haystack,
            current_span.start,
            Direction::Previous,
            DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes,
        )
        .ok()?;
        let next_span = adjacent_token_span(
            haystack,
            current_span.end,
            Direction::Next,
            DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes,
        )
        .ok()?;
        let context_span = previous_span
            .as_ref()
            .map_or(current_span.start, |span| span.start)
            ..next_span.as_ref().map_or(current_span.end, |span| span.end);
        if context_span.len() > DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes {
            return None;
        }
        let raw_context = haystack.get(context_span.clone())?;
        let relative_current = current_span.start.checked_sub(context_span.start)?
            ..current_span.end.checked_sub(context_span.start)?;
        if let Some(prepared) = prepared_cache.get(
            raw_context,
            &relative_current,
            node_limit,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        ) {
            return prepared.map(|prepared| Self { current, prepared });
        }
        let context_text = std::str::from_utf8(raw_context).ok()?;
        if context_text.nfc().count() > DEFAULT_ANALYSIS_WINDOW_LIMITS.max_normalized_scalars {
            return None;
        }
        let previous = previous_span.and_then(|span| normalized_token(haystack, span));
        let next = next_span.and_then(|span| normalized_token(haystack, span));
        let context = BoundedTokenContext {
            previous: previous.as_deref(),
            current: current.normalized(),
            next: next.as_deref(),
        };
        let prepared = resolver
            .prepare_context_for_candidate(
                context,
                node_limit,
                include_nominal_copula,
                include_nominal_derivation_predicate,
            )
            .ok()
            .map(Arc::new);
        prepared_cache.insert(
            raw_context,
            relative_current,
            node_limit,
            include_nominal_copula,
            include_nominal_derivation_predicate,
            prepared.clone(),
        );
        prepared.map(|prepared| Self { current, prepared })
    }

    pub(super) fn resolve(&self, request: StructuralRequest<'_>) -> Option<ConstraintDecision> {
        let core = self
            .current
            .normalized_span(request.candidate.core.clone())?;
        let anchor = self.current.normalized_span(request.anchor)?;
        let consumed = self.current.normalized_span(request.consumed)?;
        let token = 0..self.current.normalized().len();
        Some(self.prepared.resolve_candidate(
            CandidateSpans {
                core,
                anchor,
                consumed,
                token,
            },
            request.patterns,
        ))
    }

    pub(super) fn has_nominal_copula_host(&self, span: Range<usize>) -> bool {
        self.current
            .normalized_span(span)
            .is_some_and(|span| self.prepared.has_nominal_copula_host(&span))
    }
}

impl PreparedStructuralContextCache {
    fn get(
        &self,
        raw_context: &[u8],
        current: &Range<usize>,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Option<Option<Arc<PreparedStructuralContext>>> {
        let fingerprint = self.fingerprint(
            raw_context,
            current,
            node_limit,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        );
        self.entries.get(&fingerprint).and_then(|entries| {
            entries
                .iter()
                .find(|entry| {
                    entry.raw_context.as_ref() == raw_context
                        && entry.current == *current
                        && entry.node_limit == node_limit
                        && entry.include_nominal_copula == include_nominal_copula
                        && entry.include_nominal_derivation_predicate
                            == include_nominal_derivation_predicate
                })
                .map(|entry| entry.prepared.clone())
        })
    }

    fn insert(
        &mut self,
        raw_context: &[u8],
        current: Range<usize>,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
        prepared: Option<Arc<PreparedStructuralContext>>,
    ) {
        if self.entry_count >= MAX_PREPARED_CONTEXT_CACHE_ENTRIES {
            return;
        }
        let fingerprint = self.fingerprint(
            raw_context,
            &current,
            node_limit,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        );
        self.entries
            .entry(fingerprint)
            .or_default()
            .push(CachedPreparedContext {
                raw_context: raw_context.into(),
                current,
                node_limit,
                include_nominal_copula,
                include_nominal_derivation_predicate,
                prepared,
            });
        self.entry_count += 1;
    }

    fn fingerprint(
        &self,
        raw_context: &[u8],
        current: &Range<usize>,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> u64 {
        let mut hasher = self.fingerprint_builder.build_hasher();
        raw_context.hash(&mut hasher);
        current.hash(&mut hasher);
        node_limit.hash(&mut hasher);
        include_nominal_copula.hash(&mut hasher);
        include_nominal_derivation_predicate.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Previous,
    Next,
}

fn adjacent_token_span(
    bytes: &[u8],
    at: usize,
    direction: Direction,
    max_raw_bytes: usize,
) -> Result<Option<Range<usize>>, ()> {
    match direction {
        Direction::Previous => {
            let mut end = at;
            while let Some((start, character)) = previous_character(bytes, end)? {
                if matches!(character, '\r' | '\n') || at.saturating_sub(start) > max_raw_bytes {
                    return Ok(None);
                }
                if is_token_character(character) {
                    break;
                }
                end = start;
            }
            let mut start = end;
            while let Some((previous, character)) = previous_character(bytes, start)? {
                if at.saturating_sub(previous) > max_raw_bytes {
                    return Ok(None);
                }
                if !is_token_character(character) {
                    break;
                }
                start = previous;
            }
            Ok((start < end).then_some(start..end))
        }
        Direction::Next => {
            let mut start = at;
            while let Some((end, character)) = next_character(bytes, start)? {
                if matches!(character, '\r' | '\n') || end.saturating_sub(at) > max_raw_bytes {
                    return Ok(None);
                }
                if is_token_character(character) {
                    break;
                }
                start = end;
            }
            let mut end = start;
            while let Some((next, character)) = next_character(bytes, end)? {
                if next.saturating_sub(at) > max_raw_bytes {
                    return Ok(None);
                }
                if !is_token_character(character) {
                    break;
                }
                end = next;
            }
            Ok((start < end).then_some(start..end))
        }
    }
}

fn normalized_token(bytes: &[u8], span: Range<usize>) -> Option<Cow<'_, str>> {
    let token = std::str::from_utf8(bytes.get(span)?).ok()?;
    Some(if is_nfc(token) {
        Cow::Borrowed(token)
    } else {
        Cow::Owned(token.nfc().collect())
    })
}

fn previous_character(bytes: &[u8], at: usize) -> Result<Option<(usize, char)>, ()> {
    if at == 0 {
        return Ok(None);
    }
    let mut start = at - 1;
    while start > at.saturating_sub(4) && is_utf8_continuation(bytes[start]) {
        start -= 1;
    }
    let text = std::str::from_utf8(bytes.get(start..at).ok_or(())?).map_err(|_| ())?;
    let mut characters = text.chars();
    let character = characters.next().ok_or(())?;
    if characters.next().is_some() {
        return Err(());
    }
    Ok(Some((start, character)))
}

fn next_character(bytes: &[u8], at: usize) -> Result<Option<(usize, char)>, ()> {
    let Some(&first) = bytes.get(at) else {
        return Ok(None);
    };
    let width = utf8_width(first).ok_or(())?;
    let end = at
        .checked_add(width)
        .filter(|&end| end <= bytes.len())
        .ok_or(())?;
    let text = std::str::from_utf8(&bytes[at..end]).map_err(|_| ())?;
    let character = text.chars().next().ok_or(())?;
    Ok(Some((end, character)))
}

fn utf8_width(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

fn is_utf8_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}
