use std::array;
use std::ops::Range;

use kfind_data::{
    DataFinePos, MorphologyGraphAnalysis, MorphologyGraphExpressionKind, MorphologyGraphResource,
};

use crate::lattice::unknown::{UnknownAnalysis, UnknownDictionary};

use super::{
    ConstraintEvidenceKind, ConstraintNodeProof, ConstraintNodeSource, ConstraintPathProof,
    QueryMorphPattern,
};

pub(super) const EVIDENCE_EXACT: u8 = 1;
pub(super) const EVIDENCE_COMPONENT: u8 = 1 << 1;
pub(super) const EVIDENCE_OPAQUE: u8 = 1 << 2;
const EVIDENCE_UNKNOWN: u8 = 1 << 3;
const EVIDENCE_STATES: usize = 16;
const BOS_EOS_CONTEXT_ID: u16 = 0;

type ReachabilityStates = Vec<[Option<Predecessor>; EVIDENCE_STATES]>;
type CompletePaths = Vec<(usize, u8)>;

#[derive(Clone, Copy, Debug)]
enum Predecessor {
    Start,
    Node { index: usize, mask: u8 },
}

#[derive(Clone, Debug)]
struct Node {
    span: Range<usize>,
    pos: String,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    source: ConstraintNodeSource,
    analysis_type: Option<String>,
    expression_kind: Option<MorphologyGraphExpressionKind>,
    evidence: u8,
}

impl Node {
    fn source(
        span: Range<usize>,
        target: &Range<usize>,
        analysis: &MorphologyGraphAnalysis<'_>,
        pattern: &QueryMorphPattern,
    ) -> Self {
        let matches_query_node = span == *target
            && analysis
                .pos
                .split('+')
                .any(|pos| source_pos_matches(pos, pattern.fine_pos));
        let matches_source_component = analysis.components.iter().any(|component| {
            source_pos_matches(component.pos, pattern.fine_pos)
                && component.span.as_ref().is_some_and(|component| {
                    (span.start + component.start..span.start + component.end) == *target
                })
        });
        let has_opaque_expression = matches!(
            analysis.expression_kind,
            MorphologyGraphExpressionKind::Fused | MorphologyGraphExpressionKind::Unaligned
        ) && span.start <= target.start
            && target.end <= span.end
            && analysis
                .components
                .iter()
                .any(|component| source_pos_matches(component.pos, pattern.fine_pos));
        let mut evidence = 0;
        if matches_query_node {
            evidence |= EVIDENCE_EXACT;
        }
        if matches_source_component {
            evidence |= EVIDENCE_COMPONENT;
        }
        if has_opaque_expression {
            evidence |= EVIDENCE_OPAQUE;
        }
        Self {
            span,
            pos: analysis.pos.to_owned(),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            source: ConstraintNodeSource::Source,
            analysis_type: Some(analysis.analysis_type.to_owned()),
            expression_kind: Some(analysis.expression_kind),
            evidence,
        }
    }

    fn unknown(span: Range<usize>, analysis: &UnknownAnalysis) -> Self {
        Self {
            span,
            pos: analysis.pos.clone(),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            source: ConstraintNodeSource::Unknown,
            analysis_type: None,
            expression_kind: None,
            evidence: EVIDENCE_UNKNOWN,
        }
    }

    fn proof(&self) -> ConstraintNodeProof {
        ConstraintNodeProof {
            span: self.span.clone(),
            pos: self.pos.clone(),
            left_id: self.left_id,
            right_id: self.right_id,
            word_cost: self.word_cost,
            source: self.source,
            analysis_type: self.analysis_type.clone(),
            expression_kind: self.expression_kind,
            matches_query_node: self.evidence & EVIDENCE_EXACT != 0,
            matches_source_component: self.evidence & EVIDENCE_COMPONENT != 0,
            has_opaque_expression: self.evidence & EVIDENCE_OPAQUE != 0,
        }
    }
}

#[derive(Debug)]
pub(super) struct TokenGraph {
    nodes: Vec<Node>,
    states: ReachabilityStates,
    complete: CompletePaths,
}

impl TokenGraph {
    pub fn known(
        resource: &MorphologyGraphResource,
        text: &str,
        target: &Range<usize>,
        pattern: &QueryMorphPattern,
        node_limit: usize,
    ) -> Result<Self, usize> {
        Self::build(resource, text, target, pattern, None, node_limit)
    }

    pub fn with_unknown(
        resource: &MorphologyGraphResource,
        text: &str,
        target: &Range<usize>,
        pattern: &QueryMorphPattern,
        unknown: &UnknownDictionary,
        node_limit: usize,
    ) -> Result<Self, usize> {
        Self::build(resource, text, target, pattern, Some(unknown), node_limit)
    }

    fn build(
        resource: &MorphologyGraphResource,
        text: &str,
        target: &Range<usize>,
        pattern: &QueryMorphPattern,
        unknown: Option<&UnknownDictionary>,
        node_limit: usize,
    ) -> Result<Self, usize> {
        let mut nodes = Vec::new();
        for (start, _) in text.char_indices() {
            let before_dictionary = nodes.len();
            resource.common_prefixes(&text.as_bytes()[start..], |length, _, analyses| {
                let end = start + length;
                if end <= text.len() && text.is_char_boundary(end) {
                    nodes.extend(
                        analyses
                            .iter()
                            .map(|analysis| Node::source(start..end, target, analysis, pattern)),
                    );
                }
            });
            let has_dictionary = nodes.len() > before_dictionary;
            if let Some(unknown) = unknown {
                nodes.extend(
                    unknown
                        .nodes_at(text, start, has_dictionary)
                        .into_iter()
                        .map(|(end, analysis)| Node::unknown(start..end, analysis)),
                );
            }
            if nodes.len() > node_limit {
                return Err(nodes.len());
            }
        }
        nodes.sort_by(|left, right| {
            (
                left.span.start,
                left.span.end,
                left.source,
                left.pos.as_str(),
                left.left_id,
                left.right_id,
                left.word_cost,
                left.evidence,
            )
                .cmp(&(
                    right.span.start,
                    right.span.end,
                    right.source,
                    right.pos.as_str(),
                    right.left_id,
                    right.right_id,
                    right.word_cost,
                    right.evidence,
                ))
        });
        let (states, complete) = reachable_paths(resource, text.len(), &nodes);
        Ok(Self {
            nodes,
            states,
            complete,
        })
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

    pub fn complete_masks(&self) -> Vec<u8> {
        self.complete.iter().map(|(_, mask)| *mask).collect()
    }

    pub fn proof_paths(&self, text_len: usize) -> Vec<ConstraintPathProof> {
        self.complete
            .iter()
            .map(|&(end, mask)| {
                let mut indices = Vec::new();
                let mut current = end;
                let mut current_mask = mask;
                loop {
                    indices.push(current);
                    match self.states[current][usize::from(current_mask)]
                        .expect("completed path state")
                    {
                        Predecessor::Start => break,
                        Predecessor::Node { index, mask } => {
                            current = index;
                            current_mask = mask;
                        }
                    }
                }
                indices.reverse();
                let evidence = evidence_kind(mask, &indices, &self.nodes, text_len);
                ConstraintPathProof {
                    evidence,
                    nodes: indices
                        .into_iter()
                        .map(|index| self.nodes[index].proof())
                        .collect(),
                }
            })
            .collect()
    }
}

fn reachable_paths(
    resource: &MorphologyGraphResource,
    text_len: usize,
    nodes: &[Node],
) -> (ReachabilityStates, CompletePaths) {
    let mut ending_at = vec![Vec::<usize>::new(); text_len + 1];
    for (index, node) in nodes.iter().enumerate() {
        ending_at[node.span.end].push(index);
    }
    let mut states = (0..nodes.len())
        .map(|_| array::from_fn(|_| None))
        .collect::<Vec<[Option<Predecessor>; EVIDENCE_STATES]>>();
    for (index, node) in nodes.iter().enumerate() {
        if node.span.start == 0
            && resource
                .connection_cost(BOS_EOS_CONTEXT_ID, node.left_id)
                .is_some()
        {
            states[index][usize::from(node.evidence)] = Some(Predecessor::Start);
        }
        for &predecessor in &ending_at[node.span.start] {
            if resource
                .connection_cost(nodes[predecessor].right_id, node.left_id)
                .is_none()
            {
                continue;
            }
            for previous_mask in 0_u8..EVIDENCE_STATES as u8 {
                if states[predecessor][usize::from(previous_mask)].is_none() {
                    continue;
                }
                let mask = previous_mask | node.evidence;
                states[index][usize::from(mask)].get_or_insert(Predecessor::Node {
                    index: predecessor,
                    mask: previous_mask,
                });
            }
        }
    }
    let mut complete = Vec::new();
    for &index in &ending_at[text_len] {
        if resource
            .connection_cost(nodes[index].right_id, BOS_EOS_CONTEXT_ID)
            .is_none()
        {
            continue;
        }
        for mask in 0_u8..EVIDENCE_STATES as u8 {
            if states[index][usize::from(mask)].is_some()
                && !complete.iter().any(|(_, present)| *present == mask)
            {
                complete.push((index, mask));
            }
        }
    }
    (states, complete)
}

fn evidence_kind(
    mask: u8,
    path: &[usize],
    nodes: &[Node],
    text_len: usize,
) -> ConstraintEvidenceKind {
    if mask & EVIDENCE_UNKNOWN != 0 {
        ConstraintEvidenceKind::Unknown
    } else if mask & EVIDENCE_COMPONENT != 0 {
        ConstraintEvidenceKind::SourceComponent
    } else if mask & EVIDENCE_EXACT != 0 {
        if path.len() == 1 && nodes[path[0]].span == (0..text_len) {
            ConstraintEvidenceKind::SourceWhole
        } else {
            ConstraintEvidenceKind::RuntimeComposed
        }
    } else if mask & EVIDENCE_OPAQUE != 0 {
        ConstraintEvidenceKind::OpaqueExpression
    } else {
        ConstraintEvidenceKind::Contradiction
    }
}

fn source_pos_matches(source_pos: &str, query_pos: DataFinePos) -> bool {
    DataFinePos::parse(source_pos) == Some(query_pos)
        || (source_pos == "NNBC" && query_pos == DataFinePos::Nnb)
}
