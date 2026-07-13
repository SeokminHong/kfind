use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use kfind_data::{ComponentResource, DataFinePos, DecodedMorphologyResource};

mod decision;
mod evaluator;
mod unknown;

use decision::{LocalLatticeCosts, best_costs};
pub use evaluator::LocalComponentEvaluator;
use unknown::{UnknownAnalysis, UnknownDictionary};

pub const DEFAULT_LATTICE_NODE_LIMIT: usize = 4_096;
const N_BEST: usize = 4;
const BOS_EOS_CONTEXT_ID: u16 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalLatticeAnalysis<'a> {
    pub pos: &'a str,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

pub trait LocalLatticeResource {
    fn common_prefixes<'a>(
        &'a self,
        input: &[u8],
        emit: &mut dyn FnMut(usize, LocalLatticeAnalysis<'a>),
    );

    fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16>;

    fn right_contexts(&self) -> u16;

    fn left_contexts(&self) -> u16;

    fn char_def(&self) -> &[u8];

    fn unk_def(&self) -> &[u8];
}

impl LocalLatticeResource for DecodedMorphologyResource<'_> {
    fn common_prefixes<'a>(
        &'a self,
        input: &[u8],
        emit: &mut dyn FnMut(usize, LocalLatticeAnalysis<'a>),
    ) {
        self.common_prefixes(input, |length, analyses| {
            for analysis in analyses {
                emit(
                    length,
                    LocalLatticeAnalysis {
                        pos: analysis.pos,
                        left_id: analysis.left_id,
                        right_id: analysis.right_id,
                        word_cost: analysis.word_cost,
                    },
                );
            }
        });
    }

    fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        self.connection_cost(right_id, left_id)
    }

    fn right_contexts(&self) -> u16 {
        self.stats().right_contexts
    }

    fn left_contexts(&self) -> u16 {
        self.stats().left_contexts
    }

    fn char_def(&self) -> &[u8] {
        self.char_def()
    }

    fn unk_def(&self) -> &[u8] {
        self.unk_def()
    }
}

impl LocalLatticeResource for ComponentResource {
    fn common_prefixes<'a>(
        &'a self,
        input: &[u8],
        emit: &mut dyn FnMut(usize, LocalLatticeAnalysis<'a>),
    ) {
        self.common_prefixes(input, |length, analyses| {
            for analysis in analyses {
                emit(
                    length,
                    LocalLatticeAnalysis {
                        pos: analysis.pos,
                        left_id: analysis.left_id,
                        right_id: analysis.right_id,
                        word_cost: analysis.word_cost,
                    },
                );
            }
        });
    }

    fn connection_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        self.connection_cost(right_id, left_id)
    }

    fn right_contexts(&self) -> u16 {
        self.right_contexts()
    }

    fn left_contexts(&self) -> u16 {
        self.left_contexts()
    }

    fn char_def(&self) -> &[u8] {
        self.char_def()
    }

    fn unk_def(&self) -> &[u8] {
        self.unk_def()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalLatticeDecision {
    Accept,
    Reject,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalLatticeNode {
    pub span: Range<usize>,
    pub pos: Option<String>,
    pub word_cost: i32,
    pub unknown: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalLatticePath {
    pub cost: i64,
    pub includes_query: bool,
    pub nodes: Vec<LocalLatticeNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalLatticeReport {
    pub decision: LocalLatticeDecision,
    pub include_cost: Option<i64>,
    pub exclude_cost: Option<i64>,
    pub cost_margin: Option<i64>,
    pub node_count: usize,
    pub paths: Vec<LocalLatticePath>,
}

pub fn evaluate_local_component_paths(
    resource: &dyn LocalLatticeResource,
    text: &str,
    query_span: Range<usize>,
    query_pos: DataFinePos,
    node_limit: usize,
) -> Result<LocalLatticeReport, LocalLatticeError> {
    evaluate_local_paths(resource, text, query_span, query_pos, node_limit)
}

pub fn evaluate_local_component_decision(
    resource: &dyn LocalLatticeResource,
    text: &str,
    query_span: Range<usize>,
    query_pos: DataFinePos,
    node_limit: usize,
) -> Result<LocalLatticeDecision, LocalLatticeError> {
    evaluate_local_costs(resource, text, query_span, query_pos, node_limit)?.decision()
}

fn evaluate_local_costs(
    resource: &dyn LocalLatticeResource,
    text: &str,
    query_span: Range<usize>,
    query_pos: DataFinePos,
    node_limit: usize,
) -> Result<LocalLatticeCosts, LocalLatticeError> {
    let nodes = build_local_nodes(resource, text, query_span, query_pos, node_limit)?;
    best_costs(resource, text.len(), &nodes)
}

fn evaluate_local_paths(
    resource: &dyn LocalLatticeResource,
    text: &str,
    query_span: Range<usize>,
    query_pos: DataFinePos,
    node_limit: usize,
) -> Result<LocalLatticeReport, LocalLatticeError> {
    let nodes = build_local_nodes(resource, text, query_span, query_pos, node_limit)?;
    let completed = best_paths(resource, text.len(), &nodes)?;
    let costs = LocalLatticeCosts {
        include: completed
            .iter()
            .filter(|path| path.includes_query)
            .map(|path| path.cost)
            .min(),
        exclude: completed
            .iter()
            .filter(|path| !path.includes_query)
            .map(|path| path.cost)
            .min(),
    };
    let decision = costs.decision()?;
    let cost_margin = costs.margin()?;
    let paths = select_report_paths(&completed)
        .into_iter()
        .map(|path| LocalLatticePath {
            cost: path.cost,
            includes_query: path.includes_query,
            nodes: path
                .nodes
                .into_iter()
                .map(|index| nodes[index].summary())
                .collect(),
        })
        .collect();
    Ok(LocalLatticeReport {
        decision,
        include_cost: costs.include,
        exclude_cost: costs.exclude,
        cost_margin,
        node_count: nodes.len(),
        paths,
    })
}

fn build_local_nodes(
    resource: &dyn LocalLatticeResource,
    text: &str,
    query_span: Range<usize>,
    query_pos: DataFinePos,
    node_limit: usize,
) -> Result<Vec<Node>, LocalLatticeError> {
    validate_query_span(text, &query_span)?;
    let unknown = UnknownDictionary::parse(resource)?;
    build_nodes(resource, text, &query_span, query_pos, &unknown, node_limit)
}

fn validate_query_span(text: &str, query_span: &Range<usize>) -> Result<(), LocalLatticeError> {
    if query_span.start >= query_span.end
        || query_span.end > text.len()
        || !text.is_char_boundary(query_span.start)
        || !text.is_char_boundary(query_span.end)
    {
        return Err(LocalLatticeError::InvalidQuerySpan);
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Node {
    span: Range<usize>,
    pos: Option<String>,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    unknown: bool,
    query_match: bool,
}

impl Node {
    fn dictionary(
        span: Range<usize>,
        analysis: LocalLatticeAnalysis<'_>,
        query_span: &Range<usize>,
        query_pos: DataFinePos,
    ) -> Self {
        let matches_pos = analysis.pos.split('+').any(|pos| pos == query_pos.as_str());
        let query_match = matches_pos && span == *query_span;
        Self {
            span,
            pos: Some(analysis.pos.to_owned()),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            unknown: false,
            query_match,
        }
    }

    fn unknown(span: Range<usize>, analysis: &UnknownAnalysis) -> Self {
        Self {
            span,
            pos: Some(analysis.pos.clone()),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            unknown: true,
            query_match: false,
        }
    }

    fn summary(&self) -> LocalLatticeNode {
        LocalLatticeNode {
            span: self.span.clone(),
            pos: self.pos.clone(),
            word_cost: self.word_cost,
            unknown: self.unknown,
        }
    }
}

fn build_nodes(
    resource: &dyn LocalLatticeResource,
    text: &str,
    query_span: &Range<usize>,
    query_pos: DataFinePos,
    unknown: &UnknownDictionary,
    node_limit: usize,
) -> Result<Vec<Node>, LocalLatticeError> {
    let mut nodes = Vec::new();
    for (start, _) in text.char_indices() {
        let before_dictionary = nodes.len();
        resource.common_prefixes(&text.as_bytes()[start..], &mut |length, analysis| {
            let end = start + length;
            if end <= text.len() && text.is_char_boundary(end) {
                nodes.push(Node::dictionary(
                    start..end,
                    analysis,
                    query_span,
                    query_pos,
                ));
            }
        });
        let has_dictionary = nodes.len() > before_dictionary;
        for (end, analysis) in unknown.nodes_at(text, start, has_dictionary) {
            nodes.push(Node::unknown(start..end, analysis));
        }
        if nodes.len() > node_limit {
            return Err(LocalLatticeError::NodeLimit {
                actual: nodes.len(),
                limit: node_limit,
            });
        }
    }
    nodes.sort_by(|left, right| {
        (
            left.span.start,
            left.span.end,
            left.unknown,
            left.pos.as_deref(),
            left.left_id,
            left.right_id,
            left.word_cost,
        )
            .cmp(&(
                right.span.start,
                right.span.end,
                right.unknown,
                right.pos.as_deref(),
                right.left_id,
                right.right_id,
                right.word_cost,
            ))
    });
    nodes.dedup_by(|left, right| {
        left.span == right.span
            && left.pos == right.pos
            && left.left_id == right.left_id
            && left.right_id == right.right_id
            && left.word_cost == right.word_cost
            && left.unknown == right.unknown
    });
    Ok(nodes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PathState {
    cost: i64,
    includes_query: bool,
    nodes: Vec<usize>,
}

fn best_paths(
    resource: &dyn LocalLatticeResource,
    text_len: usize,
    nodes: &[Node],
) -> Result<Vec<PathState>, LocalLatticeError> {
    let mut ending_at = BTreeMap::<usize, Vec<usize>>::new();
    for (index, node) in nodes.iter().enumerate() {
        ending_at.entry(node.span.end).or_default().push(index);
    }
    let mut paths = (0..nodes.len())
        .map(|_| [Vec::<PathState>::new(), Vec::<PathState>::new()])
        .collect::<Vec<_>>();
    for (index, node) in nodes.iter().enumerate() {
        if node.span.start == 0 {
            if let Some(connection) = resource.connection_cost(BOS_EOS_CONTEXT_ID, node.left_id) {
                let state = PathState {
                    cost: i64::from(connection) + i64::from(node.word_cost),
                    includes_query: node.query_match,
                    nodes: vec![index],
                };
                insert_path(&mut paths[index][usize::from(state.includes_query)], state);
            }
        }
        if let Some(predecessors) = ending_at.get(&node.span.start) {
            for predecessor in predecessors {
                let Some(connection) = resource
                    .connection_cost(nodes[*predecessor].right_id, node.left_id)
                    .map(i64::from)
                else {
                    continue;
                };
                let predecessor_paths = paths[*predecessor].clone();
                for states in predecessor_paths {
                    for mut state in states {
                        state.cost = state
                            .cost
                            .checked_add(connection)
                            .and_then(|cost| cost.checked_add(i64::from(node.word_cost)))
                            .ok_or(LocalLatticeError::CostOverflow)?;
                        state.includes_query |= node.query_match;
                        state.nodes.push(index);
                        insert_path(&mut paths[index][usize::from(state.includes_query)], state);
                    }
                }
            }
        }
    }
    let mut completed = Vec::new();
    for index in ending_at.get(&text_len).into_iter().flatten() {
        let Some(connection) = resource
            .connection_cost(nodes[*index].right_id, BOS_EOS_CONTEXT_ID)
            .map(i64::from)
        else {
            continue;
        };
        for states in &paths[*index] {
            for state in states {
                let mut state = state.clone();
                state.cost = state
                    .cost
                    .checked_add(connection)
                    .ok_or(LocalLatticeError::CostOverflow)?;
                completed.push(state);
            }
        }
    }
    completed.sort_by(compare_paths);
    completed.dedup_by(|left, right| left.nodes == right.nodes);
    let mut retained_by_constraint = [0_usize; 2];
    completed.retain(|path| {
        let count = &mut retained_by_constraint[usize::from(path.includes_query)];
        *count += 1;
        *count <= N_BEST
    });
    if completed.is_empty() {
        return Err(LocalLatticeError::NoCompletePath);
    }
    Ok(completed)
}

fn insert_path(paths: &mut Vec<PathState>, candidate: PathState) {
    paths.push(candidate);
    paths.sort_by(compare_paths);
    paths.dedup_by(|left, right| left.nodes == right.nodes);
    paths.truncate(N_BEST);
}

fn compare_paths(left: &PathState, right: &PathState) -> Ordering {
    left.cost
        .cmp(&right.cost)
        .then_with(|| right.includes_query.cmp(&left.includes_query))
        .then_with(|| left.nodes.cmp(&right.nodes))
}

fn select_report_paths(completed: &[PathState]) -> Vec<PathState> {
    let mut selected = Vec::with_capacity(N_BEST);
    for includes_query in [true, false] {
        if let Some(index) = completed
            .iter()
            .position(|path| path.includes_query == includes_query)
        {
            selected.push(index);
        }
    }
    for index in 0..completed.len() {
        if selected.len() == N_BEST {
            break;
        }
        if !selected.contains(&index) {
            selected.push(index);
        }
    }
    selected.sort_unstable();
    selected
        .into_iter()
        .map(|index| completed[index].clone())
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalLatticeError {
    InvalidQuerySpan,
    InvalidUnknownModel,
    NodeLimit { actual: usize, limit: usize },
    CostOverflow,
    NoCompletePath,
}

impl Display for LocalLatticeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuerySpan => formatter.write_str("query span is outside the local eojeol"),
            Self::InvalidUnknownModel => {
                formatter.write_str("morphology resource has an invalid HANGUL unknown model")
            }
            Self::NodeLimit { actual, limit } => {
                write!(
                    formatter,
                    "local lattice has {actual} nodes; limit is {limit}"
                )
            }
            Self::CostOverflow => formatter.write_str("local lattice path cost overflowed"),
            Self::NoCompletePath => formatter.write_str("local lattice has no complete path"),
        }
    }
}

impl Error for LocalLatticeError {}

#[cfg(test)]
mod tests;
