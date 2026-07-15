use std::ops::Range;

use kfind_data::DataFinePos;

use crate::ContinuationState;

use super::super::paths::TokenGraph;
use super::super::{CandidateSpans, ConstraintNodeSource, MorphContinuation, QueryMorphPattern};
use super::{
    SupportCandidate, Unit, append_node_units, continuation_prefix_possible,
    continuation_prefix_units, enclosing_coverage, lexical_competition, predicate_continuation,
    predicate_prefix, source_pos, source_pos_matches, valid_particle_sequence,
};

pub(super) fn maximal_attached_successor_end(
    graph: &TokenGraph<'_>,
    successors: &[usize],
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    units: &[Unit<'_>],
    support: &SupportCandidate,
    allowed: bool,
) -> Option<usize> {
    let selected = continuation_prefix_units(pattern, spans, units, support, allowed)?;
    if !attached_nominal_frame_prefix(&selected) {
        return None;
    }
    let node_position = units
        .last()
        .map_or(0, |unit| unit.node_position.saturating_add(1));
    successors
        .iter()
        .filter_map(|&successor| {
            let next = &graph.nodes()[successor];
            if next.span.end > spans.consumed.end || !graph.is_on_complete_path(successor) {
                return None;
            }
            let mut extended = units.to_vec();
            append_node_units(&mut extended, node_position, successor, graph.nodes());
            continuation_prefix_possible(pattern, spans, &extended, support, allowed)
                .then_some(next.span.end)
        })
        .max()
}

pub(super) fn can_attach_nominal_frame(
    graph: &TokenGraph<'_>,
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
) -> bool {
    may_apply(pattern, spans) && !lexical_competition_at_prefix(graph, spans, pattern)
}

pub(super) fn attached_nominal_frame_prefix(units: &[&Unit<'_>]) -> bool {
    (1..=units.len()).any(|split| {
        let predicate = &units[..split];
        predicate.last().is_some_and(|unit| unit.pos == "ETM")
            && predicate_prefix(predicate.iter().map(|unit| unit.pos))
            && nominal_prefix(units[split..].iter().map(|unit| unit.pos))
    })
}

pub(super) fn match_attached_nominal_frame<'units, 'data>(
    state: ContinuationState,
    nominal_particles: bool,
    spans: &CandidateSpans,
    units: &[&'units Unit<'data>],
) -> Option<(Vec<&'units Unit<'data>>, Range<usize>)> {
    (1..units.len()).find_map(|split| {
        let predicate = &units[..split];
        let frame = &units[split..];
        let frame_start = frame.first()?.coverage.start;
        let frame_span = frame_start..spans.token.end;
        (predicate.last().is_some_and(|unit| {
            unit.pos == "ETM"
                && unit.coverage.end == frame_start
                && frame
                    .first()
                    .is_some_and(|frame| frame.source_node_index != unit.source_node_index)
        }) && predicate_continuation(state, nominal_particles, spans, predicate)
            && frame_start >= spans.core.end
            && nominal_morphology(frame)
            && enclosing_coverage(frame, frame_span.clone())
            && valid_particle_sequence(frame, ""))
        .then(|| (predicate.to_vec(), frame_span))
    })
}

fn may_apply(pattern: &QueryMorphPattern, spans: &CandidateSpans) -> bool {
    matches!(
        pattern.continuation,
        MorphContinuation::Predicate {
            nominal_particles: false,
            ..
        }
    ) && spans.core.start == spans.token.start
        && spans.core.end < spans.token.end
        && spans.consumed == spans.token
}

fn lexical_competition_at_prefix(
    graph: &TokenGraph<'_>,
    spans: &CandidateSpans,
    pattern: &QueryMorphPattern,
) -> bool {
    lexical_competition(graph, spans, std::slice::from_ref(pattern))
        || graph.nodes().iter().enumerate().any(|(index, node)| {
            graph.is_on_complete_path(index)
                && node.source == ConstraintNodeSource::Source
                && node.span.start == spans.token.start
                && node.span.end > spans.core.end
                && node.span.end <= spans.token.end
                && source_pos(node.start_pos).is_some_and(|pos| {
                    pos.is_predicate()
                        || (pos.is_nominal()
                            && !node.pos.contains('+')
                            && node.components.is_empty())
                })
                && !source_span_contains_pattern_lexeme(graph, &node.span, pattern)
        })
}

fn source_span_contains_pattern_lexeme(
    graph: &TokenGraph<'_>,
    span: &Range<usize>,
    pattern: &QueryMorphPattern,
) -> bool {
    graph.nodes().iter().enumerate().any(|(index, node)| {
        graph.is_on_complete_path(index)
            && node.source == ConstraintNodeSource::Source
            && node.span == *span
            && node.components.iter().any(|component| {
                component.surface == pattern.lexical_form.as_ref()
                    && source_pos_matches(component.pos, pattern.fine_pos)
            })
    })
}

fn nominal_prefix<'a>(positions: impl IntoIterator<Item = &'a str>) -> bool {
    let mut has_nominal = false;
    let mut particles = false;
    positions.into_iter().all(|pos| {
        if is_nominal_source_pos(pos) && !particles {
            has_nominal = true;
            true
        } else if pos == "XSN" && has_nominal && !particles {
            true
        } else if pos.starts_with('J') && has_nominal {
            particles = true;
            true
        } else {
            false
        }
    })
}

fn nominal_morphology(units: &[&Unit<'_>]) -> bool {
    let mut has_nominal = false;
    let mut particles = false;
    units.iter().all(|unit| {
        if is_nominal_source_pos(unit.pos) && !particles {
            has_nominal = true;
            true
        } else if unit.pos == "XSN" && has_nominal && !particles {
            true
        } else if unit.pos.starts_with('J') && has_nominal {
            particles = true;
            true
        } else {
            false
        }
    }) && has_nominal
}

fn is_nominal_source_pos(pos: &str) -> bool {
    pos == "NNBC" || source_pos(pos).is_some_and(DataFinePos::is_nominal)
}
