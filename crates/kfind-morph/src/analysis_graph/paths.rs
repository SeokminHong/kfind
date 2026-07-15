use std::cmp::Ordering;
use std::ops::Range;

use kfind_data::{MorphologyGraphAnalysis, MorphologyGraphExpressionKind, MorphologyGraphResource};

use crate::lattice::unknown::{UnknownAnalysis, UnknownDictionary};

use super::{
    ConstraintComponentProof, ConstraintEvidenceKind, ConstraintNodeProof, ConstraintNodeSource,
    ConstraintPathProof,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Component {
    pub surface: String,
    pub pos: String,
    pub span: Option<Range<usize>>,
}

impl Ord for Component {
    fn cmp(&self, other: &Self) -> Ordering {
        self.surface
            .cmp(&other.surface)
            .then_with(|| self.pos.cmp(&other.pos))
            .then_with(|| span_key(self.span.as_ref()).cmp(&span_key(other.span.as_ref())))
    }
}

impl PartialOrd for Component {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub(super) struct Node {
    pub surface: String,
    pub span: Range<usize>,
    pub pos: String,
    pub start_pos: String,
    pub end_pos: String,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
    pub source: ConstraintNodeSource,
    pub analysis_type: Option<String>,
    pub expression_kind: Option<MorphologyGraphExpressionKind>,
    pub components: Vec<Component>,
}

impl Node {
    fn source(surface: &str, span: Range<usize>, analysis: &MorphologyGraphAnalysis<'_>) -> Self {
        Self {
            surface: surface.to_owned(),
            components: analysis
                .components
                .iter()
                .map(|component| Component {
                    surface: component.surface.to_owned(),
                    pos: component.pos.to_owned(),
                    span: component.span.as_ref().map(|component_span| {
                        span.start + component_span.start..span.start + component_span.end
                    }),
                })
                .collect(),
            span,
            pos: analysis.pos.to_owned(),
            start_pos: effective_start_pos(analysis),
            end_pos: effective_end_pos(analysis),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            source: ConstraintNodeSource::Source,
            analysis_type: Some(analysis.analysis_type.to_owned()),
            expression_kind: Some(analysis.expression_kind),
        }
    }

    fn unknown(surface: &str, span: Range<usize>, analysis: &UnknownAnalysis) -> Self {
        Self {
            surface: surface.to_owned(),
            span,
            pos: analysis.pos.clone(),
            start_pos: analysis.pos.clone(),
            end_pos: analysis.pos.clone(),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            source: ConstraintNodeSource::Unknown,
            analysis_type: None,
            expression_kind: None,
            components: Vec::new(),
        }
    }

    fn proof(&self) -> ConstraintNodeProof {
        ConstraintNodeProof {
            surface: self.surface.clone(),
            span: self.span.clone(),
            pos: self.pos.clone(),
            start_pos: self.start_pos.clone(),
            end_pos: self.end_pos.clone(),
            left_id: self.left_id,
            right_id: self.right_id,
            word_cost: self.word_cost,
            source: self.source,
            analysis_type: self.analysis_type.clone(),
            expression_kind: self.expression_kind,
            components: self
                .components
                .iter()
                .map(|component| ConstraintComponentProof {
                    surface: component.surface.clone(),
                    pos: component.pos.clone(),
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
    PathLimit { actual: usize },
}

#[derive(Debug)]
pub(super) struct TokenGraph {
    nodes: Vec<Node>,
    complete: Vec<Vec<usize>>,
}

impl TokenGraph {
    pub fn known(
        resource: &MorphologyGraphResource,
        text: &str,
        node_limit: usize,
        path_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        Self::build(resource, text, None, node_limit, path_limit)
    }

    pub fn with_unknown(
        resource: &MorphologyGraphResource,
        text: &str,
        unknown: &UnknownDictionary,
        node_limit: usize,
        path_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        Self::build(resource, text, Some(unknown), node_limit, path_limit)
    }

    fn build(
        resource: &MorphologyGraphResource,
        text: &str,
        unknown: Option<&UnknownDictionary>,
        node_limit: usize,
        path_limit: usize,
    ) -> Result<Self, TokenGraphError> {
        let mut nodes = Vec::new();
        for (start, _) in text.char_indices() {
            let before_dictionary = nodes.len();
            resource.common_prefixes(&text.as_bytes()[start..], |length, surface, analyses| {
                let end = start + length;
                if end <= text.len() && text.is_char_boundary(end) {
                    nodes.extend(
                        analyses
                            .iter()
                            .map(|analysis| Node::source(surface, start..end, analysis)),
                    );
                }
            });
            let has_dictionary = nodes.len() > before_dictionary;
            if let Some(unknown) = unknown {
                nodes.extend(
                    unknown
                        .nodes_at(text, start, has_dictionary)
                        .into_iter()
                        .filter_map(|(end, analysis)| {
                            text.get(start..end)
                                .map(|surface| Node::unknown(surface, start..end, analysis))
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
                .then_with(|| left.surface.cmp(&right.surface))
                .then_with(|| left.pos.cmp(&right.pos))
                .then_with(|| left.start_pos.cmp(&right.start_pos))
                .then_with(|| left.end_pos.cmp(&right.end_pos))
                .then_with(|| left.left_id.cmp(&right.left_id))
                .then_with(|| left.right_id.cmp(&right.right_id))
                .then_with(|| left.word_cost.cmp(&right.word_cost))
                .then_with(|| left.analysis_type.cmp(&right.analysis_type))
                .then_with(|| left.expression_kind.cmp(&right.expression_kind))
                .then_with(|| left.components.cmp(&right.components))
        });
        let complete = complete_paths(resource, text.len(), &nodes, path_limit)
            .map_err(|actual| TokenGraphError::PathLimit { actual })?;
        Ok(Self { nodes, complete })
    }

    pub fn has_complete_paths(&self) -> bool {
        !self.complete.is_empty()
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
        self.complete
            .iter()
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
                    nodes: indices
                        .iter()
                        .map(|&index| self.nodes[index].proof())
                        .collect(),
                }
            })
            .collect()
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn complete_paths(&self) -> &[Vec<usize>] {
        &self.complete
    }
}

fn span_key(span: Option<&Range<usize>>) -> Option<(usize, usize)> {
    span.map(|span| (span.start, span.end))
}

fn complete_paths(
    resource: &MorphologyGraphResource,
    text_len: usize,
    nodes: &[Node],
    path_limit: usize,
) -> Result<Vec<Vec<usize>>, usize> {
    let mut starting_at = vec![Vec::<usize>::new(); text_len + 1];
    for (index, node) in nodes.iter().enumerate() {
        starting_at[node.span.start].push(index);
    }
    let successors = nodes
        .iter()
        .map(|node| {
            starting_at[node.span.end]
                .iter()
                .copied()
                .filter(|&next| resource.allows_transition(&node.end_pos, &nodes[next].start_pos))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut path_counts = vec![0_usize; nodes.len()];
    for index in (0..nodes.len()).rev() {
        path_counts[index] = if nodes[index].span.end == text_len {
            1
        } else {
            successors[index].iter().fold(0_usize, |count, &next| {
                count.saturating_add(path_counts[next])
            })
        };
    }
    let total = starting_at[0].iter().fold(0_usize, |count, &start| {
        count.saturating_add(path_counts[start])
    });
    if total > path_limit {
        return Err(total);
    }
    let mut complete = Vec::with_capacity(total);
    let mut path = Vec::new();
    for &start in &starting_at[0] {
        collect_complete_paths(
            start,
            text_len,
            nodes,
            &successors,
            &mut path,
            &mut complete,
        );
    }
    Ok(complete)
}

fn collect_complete_paths(
    index: usize,
    text_len: usize,
    nodes: &[Node],
    successors: &[Vec<usize>],
    path: &mut Vec<usize>,
    complete: &mut Vec<Vec<usize>>,
) {
    path.push(index);
    if nodes[index].span.end == text_len {
        complete.push(path.clone());
    } else {
        for &next in &successors[index] {
            collect_complete_paths(next, text_len, nodes, successors, path, complete);
        }
    }
    path.pop();
}

fn effective_start_pos(analysis: &MorphologyGraphAnalysis<'_>) -> String {
    if analysis.start_pos == "*" {
        analysis.pos.split('+').next().unwrap_or("*").to_owned()
    } else {
        analysis.start_pos.to_owned()
    }
}

fn effective_end_pos(analysis: &MorphologyGraphAnalysis<'_>) -> String {
    if analysis.end_pos == "*" {
        analysis
            .pos
            .split('+')
            .next_back()
            .unwrap_or("*")
            .to_owned()
    } else {
        analysis.end_pos.to_owned()
    }
}
