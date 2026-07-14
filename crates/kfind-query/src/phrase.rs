use std::collections::BTreeSet;
use std::ops::Range;

use crate::{PhraseJoinError, PhrasePolicy, VerifiedSpan};

const MAX_PHRASE_JOIN_PARTIALS: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseMatch {
    pub span: Range<usize>,
    pub atoms: Vec<VerifiedSpan>,
}

pub fn join_phrase_spans(
    text: &str,
    atom_spans: &[Vec<VerifiedSpan>],
    policy: PhrasePolicy,
) -> Result<Vec<PhraseMatch>, PhraseJoinError> {
    if atom_spans.is_empty() {
        return Err(PhraseJoinError::NoAtoms);
    }
    let mut sorted = Vec::with_capacity(atom_spans.len());
    for (atom_index, spans) in atom_spans.iter().enumerate() {
        let mut spans = spans.clone();
        for span in &spans {
            validate_span(text, atom_index, span)?;
        }
        spans.sort_by_key(|span| {
            (
                span.token.start,
                span.token.end,
                span.core.start,
                span.core.end,
            )
        });
        sorted.push(spans);
    }
    if sorted.iter().any(Vec::is_empty) {
        return Ok(Vec::new());
    }

    let mut partials = sorted[0]
        .iter()
        .cloned()
        .map(|span| PhraseMatch {
            span: span.token.clone(),
            atoms: vec![span],
        })
        .collect::<Vec<_>>();
    ensure_partial_limit(partials.len())?;
    for next_spans in &sorted[1..] {
        let mut joined = Vec::new();
        for partial in &partials {
            let previous_end = partial
                .atoms
                .last()
                .expect("partial phrase has an atom")
                .token
                .end;
            for next in next_spans {
                if next.token.start < previous_end {
                    continue;
                }
                let gap = &text[previous_end..next.token.start];
                if gap.contains(['\n', '\r']) || gap.chars().count() > policy.max_gap {
                    break;
                }
                let mut atoms = partial.atoms.clone();
                atoms.push(next.clone());
                ensure_partial_limit(joined.len().saturating_add(1))?;
                joined.push(PhraseMatch {
                    span: partial.span.start..next.token.end,
                    atoms,
                });
            }
        }
        partials = deduplicate(joined);
        if partials.is_empty() {
            break;
        }
    }
    Ok(partials)
}

fn ensure_partial_limit(count: usize) -> Result<(), PhraseJoinError> {
    if count > MAX_PHRASE_JOIN_PARTIALS {
        Err(PhraseJoinError::CandidateLimitExceeded {
            limit: MAX_PHRASE_JOIN_PARTIALS,
        })
    } else {
        Ok(())
    }
}

fn validate_span(
    text: &str,
    atom_index: usize,
    span: &VerifiedSpan,
) -> Result<(), PhraseJoinError> {
    let valid = span.token.start <= span.core.start
        && span.core.start <= span.core.end
        && span.core.end <= span.token.end
        && span.token.end <= text.len()
        && text.is_char_boundary(span.token.start)
        && text.is_char_boundary(span.core.start)
        && text.is_char_boundary(span.core.end)
        && text.is_char_boundary(span.token.end);
    if valid {
        Ok(())
    } else {
        Err(PhraseJoinError::InvalidSpan {
            atom_index,
            start: span.token.start,
            end: span.token.end,
            text_len: text.len(),
        })
    }
}

fn deduplicate(matches: Vec<PhraseMatch>) -> Vec<PhraseMatch> {
    let mut seen = BTreeSet::new();
    matches
        .into_iter()
        .filter(|matched| {
            seen.insert(
                matched
                    .atoms
                    .iter()
                    .map(|span| (span.token.start, span.token.end))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: usize, end: usize) -> VerifiedSpan {
        VerifiedSpan {
            core: start..end,
            token: start..end,
            origins: Vec::new(),
        }
    }

    #[test]
    fn joins_in_order_using_unicode_scalar_gap() {
        let text = "권한을 먼저 검증했다";
        let atoms = vec![
            vec![span(0, "권한을".len())],
            vec![span("권한을 먼저 ".len(), "권한을 먼저 검증했다".len())],
        ];

        assert!(
            join_phrase_spans(text, &atoms, PhrasePolicy { max_gap: 3 })
                .unwrap()
                .is_empty()
        );
        let matched = join_phrase_spans(text, &atoms, PhrasePolicy { max_gap: 4 }).unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].atoms.len(), 2);
    }

    #[test]
    fn rejects_reversed_overlapping_and_cross_line_spans() {
        let text = "권한\n검증";
        let cross_line = vec![
            vec![span(0, "권한".len())],
            vec![span("권한\n".len(), text.len())],
        ];
        assert!(
            join_phrase_spans(text, &cross_line, PhrasePolicy { max_gap: 24 })
                .unwrap()
                .is_empty()
        );

        let overlap = vec![vec![span(0, 6)], vec![span(3, 9)]];
        assert!(
            join_phrase_spans("가나다", &overlap, PhrasePolicy { max_gap: 24 })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_phrase_partial_growth_beyond_the_fixed_limit() {
        let text = "x".repeat(400);
        let repeated: Vec<_> = (0..400).map(|start| span(start, start + 1)).collect();
        let error = join_phrase_spans(
            &text,
            &[repeated.clone(), repeated],
            PhrasePolicy { max_gap: 400 },
        )
        .expect_err("pathological phrase combinations must be bounded");

        assert!(matches!(
            error,
            PhraseJoinError::CandidateLimitExceeded { limit: 65_536 }
        ));
    }
}
