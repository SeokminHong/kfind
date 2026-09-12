use crate::structure::graph::EdgeGraph;
use kfind_data::{ComponentPos as StructuralPos, DataFinePos};
use std::ops::Range;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::structure) struct PredicateComponent {
    pub(in crate::structure) start: usize,
    pub(in crate::structure) end: usize,
    pub(in crate::structure) pos: DataFinePos,
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum CompoundTailState {
    Predicate,
    Ending,
    Particle,
}

const COMPOUND_TAIL_STATE_COUNT: usize = 3;

pub(in crate::structure) fn compound_predicate_components(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    connective_boundary: &[bool],
) -> Box<[PredicateComponent]> {
    let edges = graph.edges();

    let mut suffix = vec![[false; COMPOUND_TAIL_STATE_COUNT]; text_len + 1];
    for state in compound_tail_states() {
        suffix[text_len][state as usize] = true;
    }
    for start in (0..text_len).rev() {
        for edge in graph.starting_at(start) {
            for state in compound_tail_states() {
                let Some(next) = advance_compound_tail_suffix(state, graph.positions(edge)) else {
                    continue;
                };
                suffix[start][state as usize] |= suffix[edge.span.end][next as usize];
            }
        }
    }

    let mut components = Vec::new();
    for edge in graph.starting_at(0) {
        let Some((pos, state)) = compound_tail_inside_analysis(graph.positions(edge)) else {
            continue;
        };
        if !suffix[edge.span.end][state as usize] {
            continue;
        }
        components.extend(
            graph
                .components(edge)
                .filter(|component| DataFinePos::parse(component.pos) == Some(pos))
                .map(|component| PredicateComponent {
                    start: edge.span.start + component.span.start,
                    end: edge.span.start + component.span.end,
                    pos,
                }),
        );
    }
    for edge in edges
        .iter()
        .filter(|edge| connective_boundary[edge.span.start])
    {
        let Some((&first, suffix_positions)) = graph.positions(edge).split_first() else {
            continue;
        };
        let Some(pos) = first.fine_pos().filter(|pos| pos.is_predicate()) else {
            continue;
        };
        let Some(state) =
            advance_compound_tail_suffix(CompoundTailState::Predicate, suffix_positions)
        else {
            continue;
        };
        if !suffix[edge.span.end][state as usize] {
            continue;
        }
        components.push(PredicateComponent {
            start: edge.span.start,
            end: edge.span.end,
            pos,
        });
        components.extend(
            graph
                .components(edge)
                .filter(|component| DataFinePos::parse(component.pos) == Some(pos))
                .map(|component| PredicateComponent {
                    start: edge.span.start + component.span.start,
                    end: edge.span.start + component.span.end,
                    pos,
                }),
        );
    }
    components.sort_unstable();
    components.dedup();
    components.into_boxed_slice()
}

fn compound_tail_inside_analysis(
    positions: &[StructuralPos],
) -> Option<(DataFinePos, CompoundTailState)> {
    let (first, positions) = positions.split_first()?;
    first.fine_pos().filter(|pos| pos.is_predicate())?;
    let mut connective = false;
    for (index, &position) in positions.iter().enumerate() {
        if position.is_ending() {
            connective = position == StructuralPos::EC;
            continue;
        }
        let pos = position.fine_pos().filter(|pos| pos.is_predicate())?;
        if !connective {
            return None;
        }
        let mut state = CompoundTailState::Predicate;
        for &suffix in &positions[index + 1..] {
            state = advance_compound_tail_position(state, suffix)?;
        }
        return Some((pos, state));
    }
    None
}

const fn compound_tail_states() -> [CompoundTailState; COMPOUND_TAIL_STATE_COUNT] {
    [
        CompoundTailState::Predicate,
        CompoundTailState::Ending,
        CompoundTailState::Particle,
    ]
}

fn advance_compound_tail_suffix(
    mut state: CompoundTailState,
    positions: &[StructuralPos],
) -> Option<CompoundTailState> {
    for &pos in positions {
        state = advance_compound_tail_position(state, pos)?;
    }
    Some(state)
}

fn advance_compound_tail_position(
    state: CompoundTailState,
    pos: StructuralPos,
) -> Option<CompoundTailState> {
    match (state, pos) {
        (CompoundTailState::Predicate | CompoundTailState::Ending, pos) if pos.is_ending() => {
            Some(CompoundTailState::Ending)
        }
        (CompoundTailState::Ending | CompoundTailState::Particle, pos) if pos.is_particle() => {
            Some(CompoundTailState::Particle)
        }
        _ => None,
    }
}

pub(in crate::structure) fn attached_auxiliary_spans(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    connective_boundary: &[bool],
    ending_suffix: &[bool],
) -> Box<[Range<usize>]> {
    let edges = graph.edges();

    let mut spans = edges
        .iter()
        .filter(|edge| {
            let mut positions = graph.positions(edge).iter().copied();
            connective_boundary[edge.span.start]
                && positions.next() == Some(StructuralPos::VX)
                && positions.all(StructuralPos::is_ending)
                && ending_suffix[edge.span.end]
        })
        .map(|edge| edge.span.clone())
        .collect::<Vec<_>>();
    spans.extend(
        edges
            .iter()
            .filter(|edge| {
                edge.span == (0..text_len)
                    && is_attached_auxiliary_whole_path(graph.positions(edge))
            })
            .map(|edge| edge.span.clone()),
    );
    if has_complete_attached_auxiliary_path(text_len, graph) {
        spans.push(0..text_len);
    }
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum AttachedAuxiliaryState {
    Start,
    Root,
    Predicate,
    Connective,
    Auxiliary,
    AuxiliaryEnding,
}

const ATTACHED_AUXILIARY_STATES: [AttachedAuxiliaryState; 6] = [
    AttachedAuxiliaryState::Start,
    AttachedAuxiliaryState::Root,
    AttachedAuxiliaryState::Predicate,
    AttachedAuxiliaryState::Connective,
    AttachedAuxiliaryState::Auxiliary,
    AttachedAuxiliaryState::AuxiliaryEnding,
];
const ATTACHED_AUXILIARY_STATE_COUNT: usize = ATTACHED_AUXILIARY_STATES.len();

pub(in crate::structure) fn has_complete_attached_auxiliary_path(
    text_len: usize,
    graph: &EdgeGraph<'_>,
) -> bool {
    let mut reachable = vec![[false; ATTACHED_AUXILIARY_STATE_COUNT]; text_len + 1];
    reachable[0][AttachedAuxiliaryState::Start as usize] = true;
    for start in 0..text_len {
        for edge in graph.starting_at(start) {
            for state in ATTACHED_AUXILIARY_STATES {
                if !reachable[start][state as usize] {
                    continue;
                }
                if let Some(next) = advance_attached_auxiliary_path(state, graph.positions(edge)) {
                    reachable[edge.span.end][next as usize] = true;
                }
            }
        }
    }
    reachable[text_len][AttachedAuxiliaryState::AuxiliaryEnding as usize]
}

fn advance_attached_auxiliary_path(
    mut state: AttachedAuxiliaryState,
    positions: &[StructuralPos],
) -> Option<AttachedAuxiliaryState> {
    for &pos in positions {
        state = match (state, pos) {
            (AttachedAuxiliaryState::Start, StructuralPos::VV | StructuralPos::VA) => {
                AttachedAuxiliaryState::Predicate
            }
            (AttachedAuxiliaryState::Start, StructuralPos::XR) => AttachedAuxiliaryState::Root,
            (AttachedAuxiliaryState::Root, StructuralPos::XSV | StructuralPos::XSA) => {
                AttachedAuxiliaryState::Predicate
            }
            (AttachedAuxiliaryState::Predicate | AttachedAuxiliaryState::Connective, pos)
                if pos.is_ending() =>
            {
                if pos == StructuralPos::EC {
                    AttachedAuxiliaryState::Connective
                } else {
                    AttachedAuxiliaryState::Predicate
                }
            }
            (AttachedAuxiliaryState::Connective, StructuralPos::VX) => {
                AttachedAuxiliaryState::Auxiliary
            }
            (AttachedAuxiliaryState::Auxiliary, pos) if pos.is_ending() => {
                AttachedAuxiliaryState::AuxiliaryEnding
            }
            (AttachedAuxiliaryState::AuxiliaryEnding, pos) if pos.is_ending() => {
                AttachedAuxiliaryState::AuxiliaryEnding
            }
            _ => return None,
        };
    }
    Some(state)
}

pub(in crate::structure) fn is_attached_auxiliary_whole_path(positions: &[StructuralPos]) -> bool {
    let mut positions = positions.iter().copied();
    match positions.next() {
        Some(StructuralPos::VV | StructuralPos::VA) => {}
        Some(StructuralPos::XR)
            if matches!(
                positions.next(),
                Some(StructuralPos::XSV | StructuralPos::XSA)
            ) => {}
        _ => return false,
    }

    let mut connective_before_auxiliary = false;
    for position in &mut positions {
        if position == StructuralPos::VX {
            return connective_before_auxiliary
                && positions.next().is_some_and(StructuralPos::is_ending)
                && positions.all(StructuralPos::is_ending);
        }
        if !position.is_ending() {
            return false;
        }
        connective_before_auxiliary = position == StructuralPos::EC;
    }
    false
}

pub(in crate::structure) fn leading_predicate_spans(
    graph: &EdgeGraph<'_>,
    ending_suffix: &[bool],
) -> Box<[Range<usize>]> {
    let mut spans = graph
        .edges()
        .iter()
        .filter_map(|edge| {
            let mut positions = graph.positions(edge).iter().copied();
            (edge.span.start == 0
                && positions.next().is_some_and(StructuralPos::is_predicate)
                && positions.all(StructuralPos::is_ending)
                && ending_suffix[edge.span.end])
                .then(|| edge.span.clone())
        })
        .collect::<Vec<_>>();
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans.into_boxed_slice()
}
