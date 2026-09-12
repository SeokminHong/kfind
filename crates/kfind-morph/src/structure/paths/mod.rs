use crate::structure::graph::EdgeGraph;
use kfind_data::ComponentPos as StructuralPos;
use std::ops::Range;

pub(super) mod nominal;
pub(super) mod numeral;
pub(super) mod predicate;

pub(in crate::structure) struct NominalPathFacts {
    pub(in crate::structure) particle_hosts: Box<[Range<usize>]>,
    pub(in crate::structure) complete_particle_host: Option<Range<usize>>,
}

pub(in crate::structure) struct CommonPathFacts {
    pub(in crate::structure) nominal_prefix: Vec<[bool; 2]>,
    pub(in crate::structure) ending_suffix: Vec<bool>,
    pub(in crate::structure) particle_suffix: Vec<bool>,
    pub(in crate::structure) predicate_connective_boundaries: Vec<bool>,
    pub(in crate::structure) exact_nominal_end: Vec<bool>,
}

impl CommonPathFacts {
    pub(in crate::structure) fn collect(text: &str, graph: &EdgeGraph<'_>) -> Self {
        let mut nominal_prefix = vec![[false; 2]; text.len() + 1];
        nominal_prefix[0][0] = true;
        let mut predicate_path = vec![false; text.len() + 1];
        let mut predicate_connective_boundaries = vec![false; text.len() + 1];
        let mut exact_nominal_end = vec![false; text.len() + 1];
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
                let ends_in_connective = if start == 0 {
                    predicate_path_ends_in_connective(positions)
                } else if predicate_path[start] {
                    ending_path_ends_in_connective(positions)
                } else {
                    None
                };
                if let Some(ends_in_connective) = ends_in_connective {
                    if ends_in_connective {
                        predicate_connective_boundaries[edge.span.end] = true;
                    } else {
                        predicate_path[edge.span.end] = true;
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

        Self {
            nominal_prefix,
            ending_suffix,
            particle_suffix,
            predicate_connective_boundaries,
            exact_nominal_end,
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
