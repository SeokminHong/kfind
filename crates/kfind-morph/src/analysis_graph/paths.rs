use std::cmp::Ordering;
use std::ops::Range;
use std::sync::OnceLock;

use kfind_data::{
    MorphologyGraphAnalysis, MorphologyGraphExpressionKind, MorphologyGraphPosClass,
    MorphologyGraphResource, MorphologyGraphStringId,
};

use crate::lattice::unknown::{UnknownAnalysis, UnknownDictionary};

use super::{
    ConstraintComponentProof, ConstraintEvidenceKind, ConstraintNodeProof, ConstraintNodeSource,
    ConstraintPathProof,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Component<'a> {
    pub surface: &'a str,
    pub surface_id: MorphologyGraphStringId,
    pub pos: &'a str,
    pub pos_id: MorphologyGraphStringId,
    pub span: Option<Range<usize>>,
}

impl Ord for Component<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.surface
            .cmp(other.surface)
            .then_with(|| self.pos.cmp(other.pos))
            .then_with(|| span_key(self.span.as_ref()).cmp(&span_key(other.span.as_ref())))
    }
}

impl PartialOrd for Component<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub(super) struct Node<'a> {
    pub surface: &'a str,
    pub surface_id: Option<MorphologyGraphStringId>,
    pub span: Range<usize>,
    pub pos: &'a str,
    pub pos_id: Option<MorphologyGraphStringId>,
    pub start_pos: &'a str,
    pub end_pos: &'a str,
    start_class: Option<MorphologyGraphPosClass>,
    end_class: Option<MorphologyGraphPosClass>,
    pub source: ConstraintNodeSource,
    pub expression_kind: Option<MorphologyGraphExpressionKind>,
    pub components: Vec<Component<'a>>,
}

impl<'a> Node<'a> {
    fn source(
        resource: &MorphologyGraphResource,
        surface: &'a str,
        span: Range<usize>,
        analysis: &MorphologyGraphAnalysis<'a>,
    ) -> Self {
        let start_pos = effective_start_pos(analysis);
        let end_pos = effective_end_pos(analysis);
        Self {
            surface,
            surface_id: Some(analysis.surface_id),
            components: analysis
                .components
                .iter()
                .map(|component| Component {
                    surface: component.surface,
                    surface_id: component.surface_id,
                    pos: component.pos,
                    pos_id: component.pos_id,
                    span: component.span.as_ref().map(|component_span| {
                        span.start + component_span.start..span.start + component_span.end
                    }),
                })
                .collect(),
            span,
            pos: analysis.pos,
            pos_id: (!analysis.pos.contains('+')).then_some(analysis.pos_id),
            start_pos,
            end_pos,
            start_class: resource.transition_class(start_pos),
            end_class: resource.transition_class(end_pos),
            source: ConstraintNodeSource::Source,
            expression_kind: Some(analysis.expression_kind),
        }
    }

    fn unknown(
        resource: &MorphologyGraphResource,
        surface: &'a str,
        span: Range<usize>,
        analysis: &'a UnknownAnalysis,
    ) -> Self {
        Self {
            surface,
            surface_id: None,
            span,
            pos: &analysis.pos,
            pos_id: None,
            start_pos: &analysis.pos,
            end_pos: &analysis.pos,
            start_class: resource.transition_class(&analysis.pos),
            end_class: resource.transition_class(&analysis.pos),
            source: ConstraintNodeSource::Unknown,
            expression_kind: None,
            components: Vec::new(),
        }
    }

    fn proof(&self) -> ConstraintNodeProof {
        ConstraintNodeProof {
            surface: self.surface.to_owned(),
            span: self.span.clone(),
            pos: self.pos.to_owned(),
            start_pos: self.start_pos.to_owned(),
            end_pos: self.end_pos.to_owned(),
            source: self.source,
            expression_kind: self.expression_kind,
            components: self
                .components
                .iter()
                .map(|component| ConstraintComponentProof {
                    surface: component.surface.to_owned(),
                    pos: component.pos.to_owned(),
                    span: component.span.clone(),
                })
                .collect(),
            matches_query_node: false,
            matches_source_component: false,
            has_opaque_expression: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TokenGraphError {
    NodeLimit { actual: usize },
}

#[derive(Debug)]
pub(super) struct TokenGraph<'a> {
    nodes: Vec<Node<'a>>,
    successors: Vec<Vec<usize>>,
    predecessors: OnceLock<Vec<Vec<usize>>>,
    reachable_from_start: Vec<bool>,
    reaches_end: Vec<bool>,
}

impl<'a> TokenGraph<'a> {
    pub fn known(
        resource: &'a MorphologyGraphResource,
        text: &'a str,
        node_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        Self::build(resource, text, None, None, node_limit)
    }

    pub fn with_unknown(
        resource: &'a MorphologyGraphResource,
        text: &'a str,
        unknown: &'a UnknownDictionary,
        node_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        Self::build(resource, text, Some(unknown), None, node_limit)
    }

    pub fn with_unknown_prefix(
        resource: &'a MorphologyGraphResource,
        text: &'a str,
        unknown: &'a UnknownDictionary,
        prefix_end: usize,
        node_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        Self::build(resource, text, Some(unknown), Some(prefix_end), node_limit)
    }

    fn build(
        resource: &'a MorphologyGraphResource,
        text: &'a str,
        unknown: Option<&'a UnknownDictionary>,
        unknown_prefix_end: Option<usize>,
        node_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        let mut nodes = Vec::new();
        for (start, _) in text.char_indices() {
            let before_dictionary = nodes.len();
            resource.common_prefixes(&text.as_bytes()[start..], |length, _, analyses| {
                let end = start + length;
                if let Some(surface) = text.get(start..end) {
                    nodes.extend(
                        analyses
                            .iter()
                            .map(|analysis| Node::source(resource, surface, start..end, analysis)),
                    );
                }
            });
            let has_dictionary = nodes.len() > before_dictionary;
            if let Some(unknown) = unknown
                && (unknown_prefix_end.is_none() || !has_dictionary)
            {
                nodes.extend(
                    unknown
                        .nodes_at(text, start, has_dictionary)
                        .into_iter()
                        .filter(|(end, _)| unknown_prefix_end.is_none_or(|limit| *end <= limit))
                        .filter_map(|(end, analysis)| {
                            text.get(start..end).map(|surface| {
                                Node::unknown(resource, surface, start..end, analysis)
                            })
                        }),
                );
            }
            if nodes.len() > node_limit {
                return Err(TokenGraphError::NodeLimit {
                    actual: nodes.len(),
                });
            }
        }
        nodes.sort_by(|left, right| {
            left.span
                .start
                .cmp(&right.span.start)
                .then_with(|| left.span.end.cmp(&right.span.end))
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.surface.cmp(right.surface))
                .then_with(|| left.pos.cmp(right.pos))
                .then_with(|| left.start_pos.cmp(right.start_pos))
                .then_with(|| left.end_pos.cmp(right.end_pos))
                .then_with(|| left.expression_kind.cmp(&right.expression_kind))
                .then_with(|| left.components.cmp(&right.components))
        });
        let successors = graph_edges(resource, text.len(), &nodes, unknown_prefix_end);
        let reachable_from_start = reachable_from_start(&nodes, &successors);
        let reaches_end = reaches_end(text.len(), &nodes, &successors);
        Ok(Self {
            nodes,
            successors,
            predecessors: OnceLock::new(),
            reachable_from_start,
            reaches_end,
        })
    }

    pub fn has_complete_paths(&self) -> bool {
        self.nodes.iter().enumerate().any(|(index, node)| {
            node.span.start == 0 && self.reachable_from_start[index] && self.reaches_end[index]
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn unknown_node_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.source == ConstraintNodeSource::Unknown)
            .count()
    }

    pub fn proof_paths(&self) -> Vec<ConstraintPathProof> {
        self.witness_paths()
            .into_iter()
            .map(|indices| {
                let evidence = if indices
                    .iter()
                    .any(|&index| self.nodes[index].source == ConstraintNodeSource::Unknown)
                {
                    ConstraintEvidenceKind::Unknown
                } else {
                    ConstraintEvidenceKind::Contradiction
                };
                ConstraintPathProof {
                    evidence,
                    node_indices: indices,
                }
            })
            .collect()
    }

    pub fn proof_nodes(&self) -> Vec<ConstraintNodeProof> {
        self.nodes.iter().map(Node::proof).collect()
    }

    pub fn witness_paths(&self) -> Vec<Vec<usize>> {
        let mut witnesses = Vec::new();
        for index in 0..self.nodes.len() {
            if !self.is_on_complete_path(index) {
                continue;
            }
            let Some(indices) = self.witness_path_through(&[index]) else {
                continue;
            };
            if !witnesses.contains(&indices) {
                witnesses.push(indices);
            }
            for &successor in &self.successors[index] {
                let Some(indices) = self.witness_path_through(&[index, successor]) else {
                    continue;
                };
                if !witnesses.contains(&indices) {
                    witnesses.push(indices);
                }
            }
        }
        witnesses
    }

    pub fn nodes(&self) -> &[Node<'a>] {
        &self.nodes
    }

    pub fn successors(&self, index: usize) -> &[usize] {
        &self.successors[index]
    }

    pub fn is_on_complete_path(&self, index: usize) -> bool {
        self.reachable_from_start[index] && self.reaches_end[index]
    }

    pub fn witness_path_through(&self, required: &[usize]) -> Option<Vec<usize>> {
        let (&first, &last) = required.first().zip(required.last())?;
        if !required
            .windows(2)
            .all(|pair| self.successors[pair[0]].contains(&pair[1]))
            || !self.is_on_complete_path(first)
            || !self.is_on_complete_path(last)
        {
            return None;
        }
        let mut prefix = vec![first];
        let mut cursor = first;
        let predecessors = self.predecessors();
        while self.nodes[cursor].span.start != 0 {
            cursor = *predecessors[cursor]
                .iter()
                .find(|&&previous| self.reachable_from_start[previous])?;
            prefix.push(cursor);
        }
        prefix.reverse();
        prefix.extend_from_slice(&required[1..]);
        cursor = last;
        while !self.successors[cursor].is_empty() {
            cursor = *self.successors[cursor]
                .iter()
                .find(|&&next| self.reaches_end[next])?;
            prefix.push(cursor);
        }
        Some(prefix)
    }

    fn predecessors(&self) -> &[Vec<usize>] {
        self.predecessors.get_or_init(|| {
            let mut predecessors = vec![Vec::new(); self.nodes.len()];
            for (index, next) in self.successors.iter().enumerate() {
                for &successor in next {
                    predecessors[successor].push(index);
                }
            }
            predecessors
        })
    }
}

fn span_key(span: Option<&Range<usize>>) -> Option<(usize, usize)> {
    span.map(|span| (span.start, span.end))
}

fn graph_edges(
    resource: &MorphologyGraphResource,
    text_len: usize,
    nodes: &[Node<'_>],
    unknown_prefix_end: Option<usize>,
) -> Vec<Vec<usize>> {
    let mut starting_at = vec![Vec::<usize>::new(); text_len + 1];
    for (index, node) in nodes.iter().enumerate() {
        starting_at[node.span.start].push(index);
    }
    nodes
        .iter()
        .map(|node| {
            starting_at[node.span.end]
                .iter()
                .copied()
                .filter(|&next| {
                    let prefix_bridge = unknown_prefix_end.is_some_and(|limit| {
                        node.span.end <= limit
                            && (node.source == ConstraintNodeSource::Unknown
                                || nodes[next].source == ConstraintNodeSource::Unknown)
                    });
                    prefix_bridge
                        || node.end_class.zip(nodes[next].start_class).is_some_and(
                            |(end, start)| resource.allows_transition_classes(end, start),
                        )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn reachable_from_start(nodes: &[Node<'_>], successors: &[Vec<usize>]) -> Vec<bool> {
    let mut reachable = vec![false; nodes.len()];
    for index in 0..nodes.len() {
        reachable[index] |= nodes[index].span.start == 0;
        if reachable[index] {
            for &successor in &successors[index] {
                reachable[successor] = true;
            }
        }
    }
    reachable
}

fn reaches_end(text_len: usize, nodes: &[Node<'_>], successors: &[Vec<usize>]) -> Vec<bool> {
    let mut reaches = vec![false; nodes.len()];
    for index in (0..nodes.len()).rev() {
        reaches[index] = nodes[index].span.end == text_len
            || successors[index].iter().any(|&next| reaches[next]);
    }
    reaches
}

fn effective_start_pos<'a>(analysis: &MorphologyGraphAnalysis<'a>) -> &'a str {
    if analysis.start_pos == "*" {
        analysis.pos.split('+').next().unwrap_or("*")
    } else {
        analysis.start_pos
    }
}

fn effective_end_pos<'a>(analysis: &MorphologyGraphAnalysis<'a>) -> &'a str {
    if analysis.end_pos == "*" {
        analysis.pos.split('+').next_back().unwrap_or("*")
    } else {
        analysis.end_pos
    }
}
