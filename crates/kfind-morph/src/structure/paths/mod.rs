use crate::hangul::{JONG_NONE, replace_final};
use crate::structure::graph::EdgeGraph;
use kfind_data::ComponentPos as StructuralPos;
use std::ops::Range;

pub(super) mod nominal;
pub(super) mod numeral;
pub(super) mod predicate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::structure) enum PredicateBoundary {
    None,
    Connective,
    Purpose,
    NominalizedTopic,
}

const PREDICATE: u8 = 1;
const DERIVATION_ROOT: u8 = 2;
const DERIVED_PREDICATE: u8 = 4;
const AFTER_CONNECTIVE: u8 = 8;
const LINKED_PREDICATE: u8 = 16;
const GI_NOMINALIZER: u8 = 32;
const COPULAR_PREDICATE: u8 = 64;

pub(in crate::structure) struct NominalPathFacts {
    pub(in crate::structure) particle_hosts: Box<[Range<usize>]>,
    pub(in crate::structure) complete_particle_host: Option<Range<usize>>,
}

pub(in crate::structure) struct CommonPathFacts {
    pub(in crate::structure) nominal_prefix: Vec<[bool; 2]>,
    pub(in crate::structure) ending_suffix: Vec<bool>,
    pub(in crate::structure) particle_suffix: Vec<bool>,
    pub(in crate::structure) predicate_boundaries: Vec<PredicateBoundary>,
    pub(in crate::structure) exact_nominal_end: Vec<bool>,
    pub(in crate::structure) has_bound_go: bool,
    pub(in crate::structure) terminal_predicate_edge_start: Option<usize>,
}

impl CommonPathFacts {
    pub(in crate::structure) fn collect(text: &str, graph: &EdgeGraph<'_>) -> Self {
        let mut nominal_prefix = vec![[false; 2]; text.len() + 1];
        nominal_prefix[0][0] = true;
        let mut predicate_path = vec![0; text.len() + 1];
        let mut predicate_boundaries = vec![PredicateBoundary::None; text.len() + 1];
        let mut exact_nominal_end = vec![false; text.len() + 1];
        let terminal_start = text.strip_suffix('고').map(str::len);
        let mut has_final_before_terminal = false;
        let mut terminal_predicate_edge_start = None;
        for start in 0..text.len() {
            for edge in graph.starting_at(start) {
                let positions = graph.positions(edge);
                for has_nominal in [false, true] {
                    if !nominal_prefix[start][usize::from(has_nominal)] {
                        continue;
                    }
                    let mut next_has_nominal = has_nominal;
                    let valid = positions.iter().all(|pos| {
                        if pos.is_nominal() {
                            next_has_nominal = true;
                            true
                        } else {
                            matches!(
                                *pos,
                                StructuralPos::XPN | StructuralPos::XSN | StructuralPos::XR
                            )
                        }
                    });
                    if valid {
                        nominal_prefix[edge.span.end][usize::from(next_has_nominal)] = true;
                    }
                }

                if start == 0 {
                    exact_nominal_end[edge.span.end] |=
                        matches!(positions, [pos] if pos.is_nominal());
                }
                let state = predicate_path[start];
                predicate_path[edge.span.end] |= linked_prefix_flags(start, state, positions);
                if (nominal_prefix[start][1]
                    && matches!(positions, [StructuralPos::VCP, prefinals @ ..]
                            if prefinals.iter().all(|&pos| pos == StructuralPos::EP)))
                    || (state & COPULAR_PREDICATE != 0
                        && positions.iter().all(|&pos| pos == StructuralPos::EP))
                {
                    predicate_path[edge.span.end] |= COPULAR_PREDICATE;
                }
                if edge.span.end == text.len() || terminal_start == Some(edge.span.end) {
                    let predicate = state
                        & (PREDICATE | DERIVED_PREDICATE | LINKED_PREDICATE | COPULAR_PREDICATE);
                    let endings = match positions {
                        [head, endings @ ..]
                            if (start == 0 || state & AFTER_CONNECTIVE != 0)
                                && head.is_predicate() =>
                        {
                            endings
                        }
                        [StructuralPos::XSV | StructuralPos::XSA, endings @ ..]
                            if state & DERIVATION_ROOT != 0 =>
                        {
                            endings
                        }
                        [StructuralPos::VCP, endings @ ..] if nominal_prefix[start][1] => endings,
                        [host, StructuralPos::VCP, endings @ ..]
                            if start == 0 && edge.span.end == text.len() && host.is_nominal() =>
                        {
                            endings
                        }
                        _ if predicate != 0 => positions,
                        _ => &[],
                    };
                    if let [
                        prefinals @ ..,
                        last @ (StructuralPos::EC | StructuralPos::EF),
                    ] = endings
                        && prefinals.iter().all(|&pos| pos == StructuralPos::EP)
                    {
                        if edge.span.end == text.len() {
                            terminal_predicate_edge_start.get_or_insert(start);
                        } else if *last == StructuralPos::EF {
                            has_final_before_terminal = true;
                        }
                    }
                }
                if state & (DERIVED_PREDICATE | LINKED_PREDICATE) != 0
                    && positions == [StructuralPos::EC]
                    && &text[edge.span.clone()] == "게"
                    && text[edge.span.end..].chars().next().is_some_and(|initial| {
                        matches!(
                            replace_final(initial, JONG_NONE),
                            Some('되' | '돼' | '하' | '해')
                        )
                    })
                {
                    predicate_boundaries[edge.span.end] = PredicateBoundary::Connective;
                }
                let surface = &text[edge.span.clone()];
                if state & (PREDICATE | DERIVED_PREDICATE | LINKED_PREDICATE) != 0 {
                    match (surface, positions) {
                        ("기", [StructuralPos::ETN]) => {
                            predicate_path[edge.span.end] |= GI_NOMINALIZER;
                            if predicate_boundaries[edge.span.end] == PredicateBoundary::None
                                && text[edge.span.end..]
                                    .strip_prefix('위')
                                    .is_some_and(|tail| {
                                        tail.chars().next().is_some_and(|initial| {
                                            matches!(
                                                replace_final(initial, JONG_NONE),
                                                Some('하' | '해')
                                            )
                                        })
                                    })
                            {
                                predicate_boundaries[edge.span.end] = PredicateBoundary::Purpose;
                            }
                        }
                        ("긴" | "기는", [StructuralPos::ETN, StructuralPos::JX])
                            if predicate_boundaries[edge.span.end] == PredicateBoundary::None =>
                        {
                            predicate_boundaries[edge.span.end] =
                                PredicateBoundary::NominalizedTopic;
                        }
                        _ => {}
                    }
                }
                if state & GI_NOMINALIZER != 0
                    && predicate_boundaries[edge.span.end] == PredicateBoundary::None
                    && surface == "는"
                    && positions == [StructuralPos::JX]
                {
                    predicate_boundaries[edge.span.end] = PredicateBoundary::NominalizedTopic;
                }
                let ends_in_connective = if start == 0 {
                    predicate_path_ends_in_connective(positions)
                } else if state & PREDICATE != 0 {
                    ending_path_ends_in_connective(positions)
                } else {
                    None
                };
                if let Some(ends_in_connective) = ends_in_connective {
                    if ends_in_connective {
                        predicate_boundaries[edge.span.end] = PredicateBoundary::Connective;
                        predicate_path[edge.span.end] |= AFTER_CONNECTIVE;
                    } else {
                        predicate_path[edge.span.end] |= PREDICATE;
                    }
                }
            }
        }

        let mut ending_suffix = vec![false; text.len() + 1];
        let mut particle_suffix = vec![false; text.len() + 1];
        ending_suffix[text.len()] = true;
        particle_suffix[text.len()] = true;
        for start in (0..text.len()).rev() {
            for edge in graph.starting_at(start) {
                if ending_suffix[edge.span.end]
                    && graph.positions(edge).iter().all(|pos| pos.is_ending())
                {
                    ending_suffix[start] = true;
                }
                if particle_suffix[edge.span.end]
                    && graph.positions(edge).iter().all(|pos| pos.is_particle())
                {
                    particle_suffix[start] = true;
                }
            }
        }

        let has_bound_go = terminal_start.is_some_and(|start| {
            let continues = predicate_path[start]
                & (PREDICATE | DERIVED_PREDICATE | LINKED_PREDICATE | COPULAR_PREDICATE)
                != 0;
            graph.starting_at(start).iter().any(|edge| {
                edge.span.end == text.len()
                    && ((continues && graph.positions(edge) == [StructuralPos::EC])
                        || (has_final_before_terminal
                            && graph.positions(edge) == [StructuralPos::JKQ]))
            })
        });

        Self {
            nominal_prefix,
            ending_suffix,
            particle_suffix,
            predicate_boundaries,
            exact_nominal_end,
            has_bound_go,
            terminal_predicate_edge_start,
        }
    }

    pub(in crate::structure) fn nominal_paths(&self, text: &str) -> NominalPathFacts {
        let mut particle_hosts = Vec::new();
        let mut complete_particle_host = None;
        for split in text.char_indices().map(|(offset, _)| offset).skip(1) {
            if self.exact_nominal_end[split] && self.particle_suffix[split] {
                particle_hosts.push(0..split);
            }
            if self.nominal_prefix[split][1] && self.particle_suffix[split] {
                complete_particle_host = Some(0..split);
            }
        }
        NominalPathFacts {
            particle_hosts: particle_hosts.into_boxed_slice(),
            complete_particle_host,
        }
    }
}

fn linked_prefix_flags(start: usize, state: u8, positions: &[StructuralPos]) -> u8 {
    if (start == 0 || state & AFTER_CONNECTIVE != 0)
        && matches!(positions, [StructuralPos::NNG | StructuralPos::XR])
    {
        return DERIVATION_ROOT;
    }
    let active = DERIVED_PREDICATE | LINKED_PREDICATE;
    if state & (DERIVATION_ROOT | AFTER_CONNECTIVE | active) == 0 {
        return 0;
    }
    let (end, next) = if state & DERIVATION_ROOT != 0
        && let [StructuralPos::XSV | StructuralPos::XSA, endings @ ..] = positions
    {
        (ending_path_ends_in_connective(endings), DERIVED_PREDICATE)
    } else if state & AFTER_CONNECTIVE != 0
        && matches!(positions, [StructuralPos::VV | StructuralPos::VA, ..])
    {
        (
            predicate_path_ends_in_connective(positions),
            LINKED_PREDICATE,
        )
    } else {
        (ending_path_ends_in_connective(positions), state & active)
    };
    match end {
        Some(true) if next != 0 => AFTER_CONNECTIVE,
        Some(false) => next,
        _ => 0,
    }
}

fn predicate_path_ends_in_connective(positions: &[StructuralPos]) -> Option<bool> {
    let (first, positions) = positions.split_first()?;
    if !matches!(*first, StructuralPos::VV | StructuralPos::VA) {
        return None;
    }
    ending_positions_end_in_connective(positions)
}

fn ending_path_ends_in_connective(positions: &[StructuralPos]) -> Option<bool> {
    ending_positions_end_in_connective(positions)
}

fn ending_positions_end_in_connective(positions: &[StructuralPos]) -> Option<bool> {
    let mut connective = false;
    for &position in positions {
        match position {
            StructuralPos::EP if !connective => {}
            StructuralPos::EC if !connective => connective = true,
            _ => return None,
        }
    }
    Some(connective)
}

pub(in crate::structure) fn forward_positions(text_len: usize, graph: &EdgeGraph<'_>) -> Vec<bool> {
    let mut forward = vec![false; text_len + 1];
    forward[0] = true;
    for start in 0..text_len {
        if !forward[start] {
            continue;
        }
        for edge in graph.starting_at(start) {
            forward[edge.span.end] = true;
        }
    }
    forward
}

pub(in crate::structure) fn forward_positions_with_prefix(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    prefix_end: usize,
) -> Vec<bool> {
    let mut forward = vec![false; text_len + 1];
    forward[0] = true;
    forward[prefix_end] = true;
    for start in 0..text_len {
        if !forward[start] {
            continue;
        }
        for edge in graph.starting_at(start) {
            forward[edge.span.end] = true;
        }
    }
    forward
}

pub(in crate::structure) fn complete_edges(
    text_len: usize,
    graph: &EdgeGraph<'_>,
    forward: &[bool],
) -> Vec<bool> {
    let mut backward = vec![false; text_len + 1];
    backward[text_len] = true;
    for start in (0..text_len).rev() {
        backward[start] = graph
            .starting_at(start)
            .iter()
            .any(|edge| backward[edge.span.end]);
    }
    graph
        .edges()
        .iter()
        .map(|edge| forward[edge.span.start] && backward[edge.span.end])
        .collect()
}
