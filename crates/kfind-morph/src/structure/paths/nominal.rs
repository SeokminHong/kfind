use crate::structure::graph::EdgeGraph;
use kfind_data::ComponentPos as StructuralPos;
use std::ops::Range;

pub(in crate::structure) fn runtime_nominal_derivation_spans(
    graph: &EdgeGraph<'_>,
    complete: &[bool],
) -> Box<[Range<usize>]> {
    let edges = graph.edges();
    let mut spans = graph
        .starting_range(0)
        .filter(|&index| {
            let edge = &edges[index];
            let positions = graph.positions(edge);
            complete[index]
                && positions.last().is_some_and(|pos| pos.is_nominal())
                && graph.starting_range(edge.span.end).any(|next_index| {
                    let next = &edges[next_index];
                    complete[next_index]
                        && graph.positions(next).first().is_some_and(|pos| {
                            pos.is_predicate_tag()
                                || matches!(*pos, StructuralPos::XSV | StructuralPos::XSA)
                        })
                })
        })
        .map(|index| edges[index].span.clone())
        .collect::<Vec<_>>();
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum NominalDerivationState {
    Start,
    Nominal,
    DerivedNominal,
}

const NOMINAL_DERIVATION_STATE_COUNT: usize = 3;

fn nominal_derivation_state(index: usize) -> NominalDerivationState {
    match index {
        0 => NominalDerivationState::Start,
        1 => NominalDerivationState::Nominal,
        2 => NominalDerivationState::DerivedNominal,
        _ => unreachable!("invalid nominal derivation state"),
    }
}

fn advance_nominal_derivation(
    state: NominalDerivationState,
    pos: StructuralPos,
) -> Option<NominalDerivationState> {
    match (state, pos) {
        (NominalDerivationState::Start, pos) if pos.is_nominal() => {
            Some(NominalDerivationState::Nominal)
        }
        (NominalDerivationState::Nominal, pos) if pos.is_nominal() => {
            Some(NominalDerivationState::Nominal)
        }
        (
            NominalDerivationState::Nominal | NominalDerivationState::DerivedNominal,
            StructuralPos::XSN,
        ) => Some(NominalDerivationState::DerivedNominal),
        _ => None,
    }
}

pub(in crate::structure) fn nominal_derivation_predicate_prefixes(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    ending_suffix: &[bool],
) -> Box<[Range<usize>]> {
    let mut nominal_prefix = vec![[false; NOMINAL_DERIVATION_STATE_COUNT]; text_len + 1];
    nominal_prefix[0][NominalDerivationState::Start as usize] = true;
    for start in 0..text_len {
        for edge in graph.starting_at(start) {
            for state_index in 0..NOMINAL_DERIVATION_STATE_COUNT {
                if !nominal_prefix[start][state_index] {
                    continue;
                }
                let Some(next) = graph.positions(edge).iter().copied().try_fold(
                    nominal_derivation_state(state_index),
                    advance_nominal_derivation,
                ) else {
                    continue;
                };
                nominal_prefix[edge.span.end][next as usize] = true;
            }
        }
    }

    let mut prefixes = (0..text_len)
        .filter(|&end| {
            nominal_prefix[end][NominalDerivationState::DerivedNominal as usize]
                && graph.starting_at(end).iter().any(|edge| {
                    let mut positions = graph.positions(edge).iter().copied();
                    matches!(
                        positions.next(),
                        Some(StructuralPos::XSV | StructuralPos::XSA)
                    ) && positions.clone().all(StructuralPos::is_ending)
                        && ending_suffix[edge.span.end]
                        && (positions.next().is_some() || edge.span.end < text_len)
                })
        })
        .map(|end| 0..end)
        .collect::<Vec<_>>();
    prefixes.sort_unstable_by_key(|span| (span.start, span.end));
    prefixes.dedup();
    prefixes.into_boxed_slice()
}

pub(in crate::structure) fn derivational_suffix_starts(
    graph: &EdgeGraph<'_>,
    complete: &[bool],
) -> Box<[usize]> {
    let mut starts = graph
        .edges()
        .iter()
        .enumerate()
        .filter(|(index, edge)| {
            complete[*index]
                && graph
                    .positions(edge)
                    .first()
                    .is_some_and(|pos| matches!(*pos, StructuralPos::XSV | StructuralPos::XSA))
        })
        .map(|(_, edge)| edge.span.start)
        .collect::<Vec<_>>();
    starts.sort_unstable();
    starts.dedup();
    starts.into_boxed_slice()
}

pub(in crate::structure) fn adnominal_derivation_suffix_starts(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    common_nominal_prefix: &[[bool; 2]],
) -> Box<[usize]> {
    let edges = graph.edges();

    let mut adnominal_ending_suffix = vec![false; text_len + 1];
    for start in (0..text_len).rev() {
        adnominal_ending_suffix[start] = graph.starting_at(start).iter().any(|edge| {
            ending_path_reaches_adnominal_boundary(
                graph.positions(edge),
                edge.span.end,
                text_len,
                &adnominal_ending_suffix,
            )
        });
    }

    let mut starts = edges
        .iter()
        .filter_map(|edge| {
            if !common_nominal_prefix[edge.span.start]
                .iter()
                .any(|&reachable| reachable)
            {
                return None;
            }
            let (first, endings) = graph.positions(edge).split_first()?;
            if !matches!(*first, StructuralPos::XSV | StructuralPos::XSA) {
                return None;
            }
            let complete = ending_path_reaches_adnominal_boundary(
                endings,
                edge.span.end,
                text_len,
                &adnominal_ending_suffix,
            );
            complete.then_some(edge.span.start)
        })
        .collect::<Vec<_>>();
    starts.sort_unstable();
    starts.dedup();
    starts.into_boxed_slice()
}

fn ending_path_reaches_adnominal_boundary(
    positions: &[StructuralPos],
    end: usize,
    text_len: usize,
    suffix: &[bool],
) -> bool {
    let mut saw_adnominal = false;
    let mut last = None;
    for &position in positions {
        if !position.is_ending() {
            return false;
        }
        saw_adnominal |= position == StructuralPos::ETM;
        last = Some(position);
    }
    if saw_adnominal {
        last == Some(StructuralPos::ETM) && end == text_len
    } else {
        suffix[end]
    }
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum CopulaSuffixState {
    Start,
    Copula,
    Ending,
    Particle,
}

const COPULA_SUFFIX_STATE_COUNT: usize = 4;

pub(in crate::structure) fn nominal_copula_hosts(
    text: &str,
    graph: &EdgeGraph<'_>,
    common_nominal_prefix: &[[bool; 2]],
) -> Box<[Range<usize>]> {
    if !text
        .char_indices()
        .skip(1)
        .any(|(_, character)| matches!(character, '이' | '입'))
    {
        return Box::default();
    }
    let text_len = text.len();
    let mut suffix = vec![[false; COPULA_SUFFIX_STATE_COUNT]; text_len + 1];
    suffix[text_len][CopulaSuffixState::Ending as usize] = true;
    suffix[text_len][CopulaSuffixState::Particle as usize] = true;
    for start in (0..text_len).rev() {
        for edge in graph.starting_at(start) {
            for state in copula_suffix_states() {
                let Some(next) = advance_copula_suffix(state, graph.positions(edge)) else {
                    continue;
                };
                suffix[start][state as usize] |= suffix[edge.span.end][next as usize];
            }
        }
    }

    common_nominal_prefix
        .iter()
        .enumerate()
        .skip(1)
        .take(text_len.saturating_sub(1))
        .filter_map(|(end, states)| {
            (states.iter().any(|&reachable| reachable)
                && copula_surface_begins_at(text, end)
                && suffix[end][CopulaSuffixState::Start as usize])
                .then_some(0..end)
        })
        .collect()
}

pub(in crate::structure) fn copula_surface_begins_at(text: &str, start: usize) -> bool {
    text.get(start..)
        .is_some_and(|suffix| suffix.starts_with('이') || suffix.starts_with('입'))
}

const fn copula_suffix_states() -> [CopulaSuffixState; COPULA_SUFFIX_STATE_COUNT] {
    [
        CopulaSuffixState::Start,
        CopulaSuffixState::Copula,
        CopulaSuffixState::Ending,
        CopulaSuffixState::Particle,
    ]
}

fn advance_copula_suffix(
    mut state: CopulaSuffixState,
    positions: &[StructuralPos],
) -> Option<CopulaSuffixState> {
    for &pos in positions {
        state = match (state, pos) {
            (CopulaSuffixState::Start, StructuralPos::VCP) => CopulaSuffixState::Copula,
            (CopulaSuffixState::Copula | CopulaSuffixState::Ending, pos) if pos.is_ending() => {
                CopulaSuffixState::Ending
            }
            (CopulaSuffixState::Ending | CopulaSuffixState::Particle, pos) if pos.is_particle() => {
                CopulaSuffixState::Particle
            }
            _ => return None,
        };
    }
    Some(state)
}
