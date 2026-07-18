#![cfg_attr(not(test), allow(dead_code))]

use std::ops::Range;

use kfind_query::{PhraseMatch, PhrasePolicy, VerifiedSpan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PhraseMatchLimit {
    First,
    Bounded(usize),
    All,
}

#[derive(Clone, Debug)]
pub(super) struct PhraseSelection {
    pub matches: Vec<PhraseMatch>,
    pub limit_exceeded: bool,
}

pub(super) fn select_phrase_matches(
    text: &str,
    atom_spans: &[Vec<VerifiedSpan>],
    policy: PhrasePolicy,
    limit: PhraseMatchLimit,
) -> PhraseSelection {
    if atom_spans.is_empty() || atom_spans.iter().any(Vec::is_empty) {
        return PhraseSelection {
            matches: Vec::new(),
            limit_exceeded: false,
        };
    }

    let candidate_index = CandidateIndex::new(text, atom_spans);
    let suffixes = suffix_states(atom_spans, policy, &candidate_index);
    collect_matches(atom_spans, &suffixes, limit)
}

#[derive(Clone, Copy, Debug)]
struct SuffixState {
    final_end: usize,
    next_span_index: Option<usize>,
}

fn suffix_states(
    atom_spans: &[Vec<VerifiedSpan>],
    policy: PhrasePolicy,
    candidate_index: &CandidateIndex,
) -> Vec<Vec<Option<SuffixState>>> {
    let mut suffixes = vec![Vec::new(); atom_spans.len()];
    let last_atom_index = atom_spans.len() - 1;
    suffixes[last_atom_index] = atom_spans[last_atom_index]
        .iter()
        .map(|span| {
            Some(SuffixState {
                final_end: span.token.end,
                next_span_index: None,
            })
        })
        .collect();

    for atom_index in (0..last_atom_index).rev() {
        let next_spans = &atom_spans[atom_index + 1];
        let range_maximum = RangeMaximum::new(&suffixes[atom_index + 1]);
        suffixes[atom_index] = atom_spans[atom_index]
            .iter()
            .enumerate()
            .map(|(span_index, span)| {
                let range = compatible_successors(
                    span,
                    candidate_index.metrics(atom_index, span_index),
                    next_spans,
                    candidate_index.atom_metrics(atom_index + 1),
                    policy.max_gap,
                );
                range_maximum.query(range).map(|choice| SuffixState {
                    final_end: choice.final_end,
                    next_span_index: Some(choice.span_index),
                })
            })
            .collect();
    }

    suffixes
}

fn compatible_successors(
    current: &VerifiedSpan,
    current_metrics: SpanMetrics,
    next_spans: &[VerifiedSpan],
    next_metrics: &[SpanMetrics],
    max_gap: usize,
) -> Range<usize> {
    let start = next_spans.partition_point(|next| next.token.start < current.token.end);
    let compatible = next_metrics[start..]
        .partition_point(|next| current_metrics.end.gap_allowed(next.start, max_gap));
    start..start + compatible
}

fn collect_matches(
    atom_spans: &[Vec<VerifiedSpan>],
    suffixes: &[Vec<Option<SuffixState>>],
    limit: PhraseMatchLimit,
) -> PhraseSelection {
    let first_spans = &atom_spans[0];
    let mut matches = Vec::new();
    let mut cursor = 0;
    let mut first_index = 0;

    while first_index < first_spans.len() {
        while first_index < first_spans.len() && first_spans[first_index].token.start < cursor {
            first_index += 1;
        }
        if first_index == first_spans.len() {
            break;
        }

        let phrase_start = first_spans[first_index].token.start;
        let group_end = first_index
            + first_spans[first_index..].partition_point(|span| span.token.start == phrase_start);
        let best = (first_index..group_end)
            .filter_map(|span_index| {
                suffixes[0][span_index].map(|state| SuffixChoice {
                    final_end: state.final_end,
                    span_index,
                })
            })
            .reduce(preferred_choice);

        let Some(best) = best else {
            first_index = group_end;
            continue;
        };
        if matches!(limit, PhraseMatchLimit::Bounded(maximum) if matches.len() == maximum) {
            return PhraseSelection {
                matches,
                limit_exceeded: true,
            };
        }
        let matched = reconstruct_match(best.span_index, atom_spans, suffixes);
        cursor = matched.span.end;
        matches.push(matched);
        if limit == PhraseMatchLimit::First {
            break;
        }
        first_index = group_end;
    }

    PhraseSelection {
        matches,
        limit_exceeded: false,
    }
}

fn reconstruct_match(
    first_span_index: usize,
    atom_spans: &[Vec<VerifiedSpan>],
    suffixes: &[Vec<Option<SuffixState>>],
) -> PhraseMatch {
    let mut atoms = Vec::with_capacity(atom_spans.len());
    let mut span_index = first_span_index;
    for atom_index in 0..atom_spans.len() {
        atoms.push(atom_spans[atom_index][span_index].clone());
        if atom_index + 1 < atom_spans.len() {
            span_index = suffixes[atom_index][span_index]
                .expect("selected phrase span has a suffix")
                .next_span_index
                .expect("non-final phrase span has a successor");
        }
    }
    let start = atoms.first().expect("phrase has an atom").token.start;
    let end = atoms.last().expect("phrase has an atom").token.end;
    PhraseMatch {
        span: start..end,
        atoms,
    }
}

#[derive(Clone, Copy, Debug)]
struct SuffixChoice {
    final_end: usize,
    span_index: usize,
}

fn preferred_choice(left: SuffixChoice, right: SuffixChoice) -> SuffixChoice {
    if (right.final_end, std::cmp::Reverse(right.span_index))
        > (left.final_end, std::cmp::Reverse(left.span_index))
    {
        right
    } else {
        left
    }
}

struct RangeMaximum {
    leaf_count: usize,
    tree: Vec<Option<SuffixChoice>>,
}

impl RangeMaximum {
    fn new(states: &[Option<SuffixState>]) -> Self {
        let leaf_count = states.len().next_power_of_two().max(1);
        let mut tree = vec![None; leaf_count * 2];
        for (span_index, state) in states.iter().enumerate() {
            tree[leaf_count + span_index] = state.map(|state| SuffixChoice {
                final_end: state.final_end,
                span_index,
            });
        }
        for index in (1..leaf_count).rev() {
            tree[index] = preferred_optional(tree[index * 2], tree[index * 2 + 1]);
        }
        Self { leaf_count, tree }
    }

    fn query(&self, range: Range<usize>) -> Option<SuffixChoice> {
        let mut start = range.start + self.leaf_count;
        let mut end = range.end + self.leaf_count;
        let mut best = None;
        while start < end {
            if start % 2 == 1 {
                best = preferred_optional(best, self.tree[start]);
                start += 1;
            }
            if end % 2 == 1 {
                end -= 1;
                best = preferred_optional(best, self.tree[end]);
            }
            start /= 2;
            end /= 2;
        }
        best
    }
}

fn preferred_optional(
    left: Option<SuffixChoice>,
    right: Option<SuffixChoice>,
) -> Option<SuffixChoice> {
    match (left, right) {
        (Some(left), Some(right)) => Some(preferred_choice(left, right)),
        (Some(choice), None) | (None, Some(choice)) => Some(choice),
        (None, None) => None,
    }
}

struct CandidateIndex {
    atom_metrics: Vec<Vec<SpanMetrics>>,
}

impl CandidateIndex {
    fn new(text: &str, atom_spans: &[Vec<VerifiedSpan>]) -> Self {
        let mut positions = atom_spans
            .iter()
            .flatten()
            .flat_map(|span| [span.token.start, span.token.end])
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions.dedup();

        let mut characters = text.char_indices().peekable();
        let mut prefix = PrefixMetrics::default();
        let mut prefixes = Vec::with_capacity(positions.len());
        for position in &positions {
            while characters
                .peek()
                .is_some_and(|(character_start, _)| character_start < position)
            {
                let (_, character) = characters.next().expect("peeked character exists");
                prefix.scalars += 1;
                prefix.line_breaks += usize::from(matches!(character, '\n' | '\r'));
            }
            prefixes.push(prefix);
        }
        let atom_metrics = atom_spans
            .iter()
            .map(|spans| {
                spans
                    .iter()
                    .map(|span| SpanMetrics {
                        start: prefix_at(&positions, &prefixes, span.token.start),
                        end: prefix_at(&positions, &prefixes, span.token.end),
                    })
                    .collect()
            })
            .collect();
        Self { atom_metrics }
    }

    fn metrics(&self, atom_index: usize, span_index: usize) -> SpanMetrics {
        self.atom_metrics[atom_index][span_index]
    }

    fn atom_metrics(&self, atom_index: usize) -> &[SpanMetrics] {
        &self.atom_metrics[atom_index]
    }
}

fn prefix_at(positions: &[usize], prefixes: &[PrefixMetrics], position: usize) -> PrefixMetrics {
    prefixes[positions
        .binary_search(&position)
        .expect("verified span endpoint is indexed")]
}

#[derive(Clone, Copy, Debug)]
struct SpanMetrics {
    start: PrefixMetrics,
    end: PrefixMetrics,
}

#[derive(Clone, Copy, Debug, Default)]
struct PrefixMetrics {
    scalars: usize,
    line_breaks: usize,
}

impl PrefixMetrics {
    fn gap_allowed(self, to: Self, max_gap: usize) -> bool {
        to.line_breaks == self.line_breaks && to.scalars.saturating_sub(self.scalars) <= max_gap
    }
}

#[cfg(test)]
mod tests {
    use kfind_query::join_phrase_spans;
    use proptest::prelude::*;

    use super::*;

    fn span(start: usize, end: usize) -> VerifiedSpan {
        VerifiedSpan {
            core: start..end,
            token: start..end,
            origins: Vec::new(),
        }
    }

    #[test]
    fn repeated_spans_use_bounded_suffix_state() {
        let text = "x".repeat(128);
        let repeated = (0..128)
            .map(|start| span(start, start + 1))
            .collect::<Vec<_>>();
        let atom_spans = vec![repeated; 8];

        let matches = select_phrase_matches(
            &text,
            &atom_spans,
            PhrasePolicy { max_gap: 128 },
            PhraseMatchLimit::All,
        )
        .matches;

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].span, 0..128);
        assert_eq!(matches[0].atoms.len(), 8);
    }

    #[test]
    fn phrase_selection_does_not_cross_a_line_break() {
        let text = "a\nb";
        let atom_spans = vec![vec![span(0, 1)], vec![span(2, 3)]];

        assert!(
            select_phrase_matches(
                text,
                &atom_spans,
                PhrasePolicy { max_gap: 10 },
                PhraseMatchLimit::All,
            )
            .matches
            .is_empty()
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_024))]

        #[test]
        fn bounded_selection_matches_exhaustive_join(
            first in prop::collection::vec((0usize..20, 1usize..5), 0..8),
            second in prop::collection::vec((0usize..20, 1usize..5), 0..8),
            third in prop::collection::vec((0usize..20, 1usize..5), 0..8),
            max_gap in 0usize..20,
        ) {
            let text = "x".repeat(20);
            let atom_spans = [first, second, third]
                .into_iter()
                .map(|ranges| normalized_spans(ranges, text.len()))
                .collect::<Vec<_>>();
            let policy = PhrasePolicy { max_gap };

            let mut exhaustive = join_phrase_spans(&text, &atom_spans, policy).unwrap();
            exhaustive.sort_by(|left, right| {
                (left.span.start, std::cmp::Reverse(left.span.end))
                    .cmp(&(right.span.start, std::cmp::Reverse(right.span.end)))
            });
            let mut cursor = 0;
            exhaustive.retain(|matched| {
                if matched.span.start < cursor {
                    false
                } else {
                    cursor = matched.span.end;
                    true
                }
            });

            let selected = select_phrase_matches(
                &text,
                &atom_spans,
                policy,
                PhraseMatchLimit::All,
            ).matches;
            prop_assert_eq!(selected.clone(), exhaustive.clone());
            let streamed = super::super::streaming_phrase::select_verified_spans(
                &text,
                &atom_spans,
                policy,
                PhraseMatchLimit::All,
            ).matches;
            prop_assert_eq!(streamed, exhaustive.clone());

            let expected_first = exhaustive.iter().take(1).cloned().collect::<Vec<_>>();
            let bulk_first = select_phrase_matches(
                &text,
                &atom_spans,
                policy,
                PhraseMatchLimit::First,
            ).matches;
            prop_assert_eq!(bulk_first, expected_first.clone());
            let streamed_first = super::super::streaming_phrase::select_verified_spans(
                &text,
                &atom_spans,
                policy,
                PhraseMatchLimit::First,
            ).matches;
            prop_assert_eq!(streamed_first, expected_first);

            for maximum in 0..=3 {
                let expected = PhraseSelection {
                    matches: exhaustive.iter().take(maximum).cloned().collect(),
                    limit_exceeded: exhaustive.len() > maximum,
                };
                let bulk = select_phrase_matches(
                    &text,
                    &atom_spans,
                    policy,
                    PhraseMatchLimit::Bounded(maximum),
                );
                prop_assert_eq!(bulk.matches, expected.matches.clone());
                prop_assert_eq!(bulk.limit_exceeded, expected.limit_exceeded);
                let streamed = super::super::streaming_phrase::select_verified_spans(
                    &text,
                    &atom_spans,
                    policy,
                    PhraseMatchLimit::Bounded(maximum),
                );
                prop_assert_eq!(streamed.matches, expected.matches);
                prop_assert_eq!(streamed.limit_exceeded, expected.limit_exceeded);
            }
        }
    }

    fn normalized_spans(ranges: Vec<(usize, usize)>, text_len: usize) -> Vec<VerifiedSpan> {
        let mut spans = ranges
            .into_iter()
            .map(|(start, length)| span(start, start.saturating_add(length).min(text_len)))
            .filter(|span| span.token.start < span.token.end)
            .collect::<Vec<_>>();
        spans.sort_by_key(|span| (span.token.start, span.token.end));
        spans.dedup_by(|left, right| left.token == right.token);
        spans
    }
}
