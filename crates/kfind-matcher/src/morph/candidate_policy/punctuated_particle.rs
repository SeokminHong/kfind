use std::ops::Range;

use kfind_morph::{CoarsePos, FinePos, ParticleVerifier};
use kfind_query::{AtomPlan, CandidateConsumption, CandidateDecision, CandidateProgram};
use unicode_general_category::{GeneralCategory, get_general_category};

use crate::boundary::{is_boundary_after, is_token_character, previous_character};

use super::super::{context, valid_utf8_suffix};

pub(super) fn rejects(
    haystack: &[u8],
    span: &Range<usize>,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    particles: &ParticleVerifier,
) -> bool {
    let Some(close) = previous_character(haystack, span.start) else {
        return false;
    };
    let enclosure = match close {
        ')' => Some(('(', true)),
        ']' => Some(('[', true)),
        '〉' => Some(('〈', false)),
        '》' => Some(('《', false)),
        '」' => Some(('「', false)),
        '』' => Some(('『', false)),
        '”' => Some(('“', false)),
        '’' => Some(('‘', false)),
        '\'' | '"' | '`' => Some((close, false)),
        '%' | '％' => None,
        _ => return false,
    };
    if !is_boundary_after(haystack, span.end)
        || (matches!(branch.consumption, CandidateConsumption::Anchor)
            && matches!(branch.decision, CandidateDecision::Boundary(_)))
        || !branch.origins.iter().all(|origin| {
            let analysis = &atom.analyses[usize::from(origin.analysis_index)];
            analysis.fine_pos != FinePos::Copula
                && matches!(
                    analysis.coarse_pos,
                    CoarsePos::Noun
                        | CoarsePos::Pronoun
                        | CoarsePos::Numeral
                        | CoarsePos::Determiner
                        | CoarsePos::Adverb
                        | CoarsePos::Verb
                        | CoarsePos::Adjective
                )
        })
    {
        return false;
    }
    let Some(suffix) = context::normalized_token(haystack, span.clone()) else {
        return false;
    };
    let Some((open, supplement)) = enclosure else {
        return previous_character(haystack, span.start - close.len_utf8()).is_some_and(
            |character| get_general_category(character) == GeneralCategory::DecimalNumber,
        ) && particles.verify_exact("퍼센트", &suffix).is_some();
    };
    let backtick = close == '`';
    let quotative = !supplement && !backtick && suffix == "고";
    if !quotative
        && !particles
            .model()
            .allomorphs
            .iter()
            .any(|form| suffix.starts_with(form.surface.as_ref()))
    {
        return false;
    }
    let end = if backtick {
        span.start
    } else {
        span.start - close.len_utf8()
    };
    let left = valid_utf8_suffix(&haystack[..end]);
    let line = left.rsplit(['\r', '\n']).next().unwrap_or_default();
    let line_start_visible = line.len() < left.len() || left.len() == end;
    let enclosure = if backtick {
        line_start_visible
            .then(|| paired_backtick_contents(line))
            .flatten()
            .map(|enclosed| ("", enclosed))
    } else {
        let opener = if open == '"' && line_start_visible {
            let last = memchr::memrchr(b'"', line.as_bytes());
            last.filter(|&index| {
                memchr::memchr_iter(b'"', &line.as_bytes()[..index])
                    .count()
                    .is_multiple_of(2)
            })
        } else {
            matching_opener(line, open, close)
        };
        opener.map(|opener| (&line[..opener], &line[opener + open.len_utf8()..]))
    };
    let Some((before, enclosed)) = enclosure else {
        return false;
    };
    if !enclosed.chars().any(is_token_character) {
        return false;
    }
    if quotative {
        return true;
    }
    let tail = enclosed.trim_end_matches([')', ']']);
    (supplement && verifies_after(before, &suffix, particles))
        || verifies_after(tail, &suffix, particles)
        || before_trailing_supplement(enclosed)
            .is_some_and(|host| verifies_after(host, &suffix, particles))
}

fn paired_backtick_contents(line: &str) -> Option<&str> {
    let mut cursor = 0;
    while let Some(mut opener) = next_backtick_run(line, cursor) {
        cursor = opener.end;
        let before = &line[..opener.start];
        if !(before.len() - before.trim_end_matches('\\').len()).is_multiple_of(2) {
            opener.start += 1;
        }
        if opener.is_empty() {
            continue;
        }
        let mut search = opener.end;
        while let Some(closer) = next_backtick_run(line, search) {
            search = closer.end;
            if closer.len() != opener.len() {
                continue;
            }
            if closer.end == line.len() {
                let enclosed = &line[opener.end..closer.start];
                return Some(
                    enclosed
                        .strip_prefix(' ')
                        .and_then(|text| text.strip_suffix(' '))
                        .unwrap_or(enclosed),
                );
            }
            cursor = closer.end;
            break;
        }
    }
    None
}

fn next_backtick_run(line: &str, from: usize) -> Option<Range<usize>> {
    let start = from + memchr::memchr(b'`', &line.as_bytes()[from..])?;
    let end = line.len() - line[start..].trim_start_matches('`').len();
    Some(start..end)
}

fn before_trailing_supplement(enclosed: &str) -> Option<&str> {
    let close = enclosed.chars().next_back()?;
    let open = match close {
        ')' => '(',
        ']' => '[',
        _ => return None,
    };
    let start = matching_opener(&enclosed[..enclosed.len() - close.len_utf8()], open, close)?;
    Some(&enclosed[..start])
}

fn matching_opener(line: &str, open: char, close: char) -> Option<usize> {
    if open == close {
        let index = line.rfind(open)?;
        return line[..index]
            .chars()
            .next_back()
            .is_none_or(|character| !is_token_character(character))
            .then_some(index);
    }
    let mut nested = 0;
    for (index, character) in line.char_indices().rev() {
        if character == close {
            nested += 1;
        } else if character == open {
            if nested == 0 {
                return Some(index);
            }
            nested -= 1;
        }
    }
    None
}

fn verifies_after(host: &str, suffix: &str, particles: &ParticleVerifier) -> bool {
    host.chars().next_back().is_some_and(is_token_character)
        && context::normalized_token(host.as_bytes(), 0..host.len())
            .is_some_and(|normalized| particles.verify_exact(&normalized, suffix).is_some())
}
