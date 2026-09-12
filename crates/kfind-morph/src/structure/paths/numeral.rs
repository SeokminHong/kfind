use crate::structure::graph::EdgeGraph;
use kfind_data::ComponentPos as StructuralPos;
use std::ops::Range;

#[repr(usize)]
#[derive(Clone, Copy)]
enum NumeralPathState {
    Start,
    OneNumeral,
    ManyNumerals,
    Unit,
    ManyNumeralParticles,
    UnitParticles,
}

const NUMERAL_PATH_STATE_COUNT: usize = 6;

fn numeral_path_transition(
    state: NumeralPathState,
    positions: &[StructuralPos],
) -> Option<NumeralPathState> {
    let particle = positions.iter().all(|pos| pos.is_particle());
    match (state, positions) {
        (NumeralPathState::Start, [StructuralPos::NR]) => Some(NumeralPathState::OneNumeral),
        (NumeralPathState::OneNumeral | NumeralPathState::ManyNumerals, [StructuralPos::NR]) => {
            Some(NumeralPathState::ManyNumerals)
        }
        (
            NumeralPathState::OneNumeral | NumeralPathState::ManyNumerals,
            [StructuralPos::NNB | StructuralPos::NNBC],
        ) => Some(NumeralPathState::Unit),
        (NumeralPathState::ManyNumerals, _) if particle => {
            Some(NumeralPathState::ManyNumeralParticles)
        }
        (NumeralPathState::ManyNumeralParticles, _) if particle => {
            Some(NumeralPathState::ManyNumeralParticles)
        }
        (NumeralPathState::Unit, _) | (NumeralPathState::UnitParticles, _) if particle => {
            Some(NumeralPathState::UnitParticles)
        }
        _ => None,
    }
}

fn numeral_path_state(index: usize) -> NumeralPathState {
    const STATES: [NumeralPathState; NUMERAL_PATH_STATE_COUNT] = [
        NumeralPathState::Start,
        NumeralPathState::OneNumeral,
        NumeralPathState::ManyNumerals,
        NumeralPathState::Unit,
        NumeralPathState::ManyNumeralParticles,
        NumeralPathState::UnitParticles,
    ];
    STATES[index]
}

fn complete_numeral_path_state(state: NumeralPathState, require_unit: bool) -> bool {
    if require_unit {
        matches!(
            state,
            NumeralPathState::Unit | NumeralPathState::UnitParticles
        )
    } else {
        matches!(
            state,
            NumeralPathState::ManyNumerals
                | NumeralPathState::Unit
                | NumeralPathState::ManyNumeralParticles
                | NumeralPathState::UnitParticles
        )
    }
}

pub(in crate::structure) fn hangul_numeral_spans(
    text_len: usize,
    graph: &EdgeGraph<'_>,
) -> Vec<Range<usize>> {
    numeral_sequence_spans(text_len, 0, graph, false)
}

pub(in crate::structure) fn numeral_sequence_spans(
    text_len: usize,
    sequence_start: usize,
    graph: &EdgeGraph<'_>,
    require_unit: bool,
) -> Vec<Range<usize>> {
    let mut forward = vec![[false; NUMERAL_PATH_STATE_COUNT]; text_len + 1];
    forward[sequence_start][NumeralPathState::Start as usize] = true;
    for start in sequence_start..text_len {
        for edge in graph.starting_at(start) {
            for state_index in 0..NUMERAL_PATH_STATE_COUNT {
                if !forward[start][state_index] {
                    continue;
                }
                let state = numeral_path_state(state_index);
                if let Some(next) = numeral_path_transition(state, graph.positions(edge)) {
                    forward[edge.span.end][next as usize] = true;
                }
            }
        }
    }

    let mut backward = vec![[false; NUMERAL_PATH_STATE_COUNT]; text_len + 1];
    for (state_index, complete) in backward[text_len].iter_mut().enumerate() {
        *complete = complete_numeral_path_state(numeral_path_state(state_index), require_unit);
    }
    for start in (0..text_len).rev() {
        for edge in graph.starting_at(start) {
            for state_index in 0..NUMERAL_PATH_STATE_COUNT {
                let state = numeral_path_state(state_index);
                let Some(next) = numeral_path_transition(state, graph.positions(edge)) else {
                    continue;
                };
                backward[start][state_index] |= backward[edge.span.end][next as usize];
            }
        }
    }

    let mut spans = Vec::new();
    for edge in graph
        .edges()
        .iter()
        .filter(|edge| graph.positions(edge) == [StructuralPos::NR])
    {
        let belongs_to_complete_path = (0..NUMERAL_PATH_STATE_COUNT).any(|state_index| {
            if !forward[edge.span.start][state_index] {
                return false;
            }
            let state = numeral_path_state(state_index);
            numeral_path_transition(state, graph.positions(edge))
                .is_some_and(|next| backward[edge.span.end][next as usize])
        });
        if belongs_to_complete_path {
            spans.push(edge.span.clone());
        }
    }
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans
}
