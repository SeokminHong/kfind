use std::ops::Range;

use kfind_morph::{
    BoundedTokenContext, CandidateSpans, ConstraintDecision, ConstraintResolver,
    PreparedStructuralContext, QueryMorphPattern,
};
use kfind_query::VerifiedSpan;
use unicode_normalization::UnicodeNormalization;

use crate::{AnalysisWindow, DEFAULT_ANALYSIS_WINDOW_LIMITS, is_token_character};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StructuralContextAnalysis {
    current: AnalysisWindow,
    previous: Option<String>,
    next: Option<String>,
}

#[derive(Debug)]
pub(super) struct PreparedStructuralContextAnalysis {
    current: AnalysisWindow,
    prepared: PreparedStructuralContext,
}

pub(super) struct StructuralRequest<'a> {
    pub(super) candidate: &'a VerifiedSpan,
    pub(super) anchor: Range<usize>,
    pub(super) consumed: Range<usize>,
    pub(super) patterns: &'a [QueryMorphPattern],
}

impl StructuralContextAnalysis {
    pub(super) fn extract(haystack: &[u8], candidate: Range<usize>) -> Option<Self> {
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
        let context_text = std::str::from_utf8(haystack.get(context_span)?).ok()?;
        if context_text.nfc().count() > DEFAULT_ANALYSIS_WINDOW_LIMITS.max_normalized_scalars {
            return None;
        }
        Some(Self {
            current,
            previous: previous_span.and_then(|span| normalized_token(haystack, span)),
            next: next_span.and_then(|span| normalized_token(haystack, span)),
        })
    }

    pub(super) fn prepare(
        self,
        resolver: &ConstraintResolver,
        node_limit: usize,
    ) -> Option<PreparedStructuralContextAnalysis> {
        let context = BoundedTokenContext {
            previous: self.previous.as_deref(),
            current: self.current.normalized(),
            next: self.next.as_deref(),
        };
        let prepared = resolver.prepare_context(context, node_limit).ok()?;
        Some(PreparedStructuralContextAnalysis {
            current: self.current,
            prepared,
        })
    }
}

impl PreparedStructuralContextAnalysis {
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

fn normalized_token(bytes: &[u8], span: Range<usize>) -> Option<String> {
    std::str::from_utf8(bytes.get(span)?)
        .ok()
        .map(|token| token.nfc().collect())
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
