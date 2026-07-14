use std::ops::Range;

use kfind_data::{ComponentResource, DataFinePos};
use kfind_morph::{FinePos, LocalComponentEvaluator};
use unicode_normalization::UnicodeNormalization;

use crate::{AnalysisWindow, DEFAULT_ANALYSIS_WINDOW_LIMITS, is_token_character};

#[derive(Clone, Debug, Eq, PartialEq)]
enum LexicalContextDecision {
    CopularNominal {
        nominal: Range<usize>,
        copula: Range<usize>,
    },
    RepeatedAdverb,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LexicalContextAnalysis {
    current: AnalysisWindow,
    decision: LexicalContextDecision,
}

impl LexicalContextAnalysis {
    pub(super) fn extract(
        evaluator: &LocalComponentEvaluator,
        haystack: &[u8],
        candidate: Range<usize>,
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
        let context_text = std::str::from_utf8(haystack.get(context_span)?).ok()?;
        if context_text.nfc().count() > DEFAULT_ANALYSIS_WINDOW_LIMITS.max_normalized_scalars {
            return None;
        }
        let previous = previous_span.and_then(|span| normalized_token(haystack, span));
        let next = next_span.and_then(|span| normalized_token(haystack, span));
        let resource = evaluator.resource();
        let copular = copular_nominal_decision(
            resource,
            previous.as_deref(),
            current.normalized(),
            next.as_deref(),
        );
        let repeated = repeated_adverb_decision(
            resource,
            previous.as_deref(),
            current.normalized(),
            next.as_deref(),
        );
        let decision = match (copular, repeated) {
            (Some(decision), false) => decision,
            (None, true) => LexicalContextDecision::RepeatedAdverb,
            _ => return None,
        };
        Some(Self { current, decision })
    }

    pub(super) fn accepts(&self, candidate: Range<usize>, fine_pos: FinePos) -> bool {
        let Some(normalized) = self.current.normalized_span(candidate) else {
            return false;
        };
        match &self.decision {
            LexicalContextDecision::CopularNominal { nominal, copula } => {
                (normalized == *nominal && fine_pos.coarse() == kfind_morph::CoarsePos::Noun)
                    || (normalized == *copula && fine_pos == FinePos::Copula)
            }
            LexicalContextDecision::RepeatedAdverb => {
                normalized == (0..self.current.normalized().len())
                    && fine_pos == FinePos::GeneralAdverb
            }
        }
    }
}

fn copular_nominal_decision(
    resource: &ComponentResource,
    previous: Option<&str>,
    current: &str,
    next: Option<&str>,
) -> Option<LexicalContextDecision> {
    if !previous.is_some_and(|token| complete_pos_path(resource, token, &["VCN", "EC"]))
        || !next.is_some_and(|token| starts_with_pos(resource, token, is_dependent_noun))
    {
        return None;
    }

    let mut splits = current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&offset| {
            exact_single_pos(resource, &current[..offset], DataFinePos::is_nominal)
                && exact_pos_sequence(resource, &current[offset..], &["VCP", "ETM"])
        });
    let split = splits.next()?;
    if splits.next().is_some() {
        return None;
    }
    Some(LexicalContextDecision::CopularNominal {
        nominal: 0..split,
        copula: split..current.len(),
    })
}

fn repeated_adverb_decision(
    resource: &ComponentResource,
    previous: Option<&str>,
    current: &str,
    next: Option<&str>,
) -> bool {
    (previous == Some(current) || next == Some(current))
        && exact_single_pos(resource, current, |pos| pos == DataFinePos::Mag)
}

fn exact_single_pos(
    resource: &ComponentResource,
    token: &str,
    accepts: impl Fn(DataFinePos) -> bool,
) -> bool {
    exact_analysis(resource, token, |pos| {
        DataFinePos::parse(pos).is_some_and(&accepts)
    })
}

fn exact_pos_sequence(resource: &ComponentResource, token: &str, expected: &[&str]) -> bool {
    exact_analysis(resource, token, |pos| {
        pos.split('+').eq(expected.iter().copied())
    })
}

fn complete_pos_path(resource: &ComponentResource, token: &str, expected: &[&str]) -> bool {
    complete_pos_path_from(resource, token.as_bytes(), expected)
}

fn complete_pos_path_from(resource: &ComponentResource, input: &[u8], expected: &[&str]) -> bool {
    if expected.is_empty() {
        return input.is_empty();
    }
    let mut candidates = Vec::new();
    resource.common_prefixes(input, |length, analyses| {
        for analysis in analyses {
            let actual = analysis.pos.split('+').collect::<Vec<_>>();
            if expected.starts_with(&actual) {
                candidates.push((length, actual.len()));
            }
        }
    });
    candidates.into_iter().any(|(length, consumed)| {
        length > 0 && complete_pos_path_from(resource, &input[length..], &expected[consumed..])
    })
}

fn starts_with_pos(
    resource: &ComponentResource,
    token: &str,
    accepts: impl Fn(&str) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefixes(token.as_bytes(), |_, analyses| {
        matched |= analyses.iter().any(|analysis| accepts(analysis.pos));
    });
    matched
}

fn exact_analysis(
    resource: &ComponentResource,
    token: &str,
    accepts: impl Fn(&str) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefixes(token.as_bytes(), |length, analyses| {
        if length == token.len() {
            matched |= analyses.iter().any(|analysis| accepts(analysis.pos));
        }
    });
    matched
}

fn is_dependent_noun(pos: &str) -> bool {
    pos.split('+')
        .next()
        .is_some_and(|first| matches!(first, "NNB" | "NNBC"))
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
