use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use kfind_data::{DataFinePos, DecodedMorphologyResource, MorphologyAnalysis};

pub const DEFAULT_LATTICE_NODE_LIMIT: usize = 4_096;
const N_BEST: usize = 4;
const BOS_EOS_CONTEXT_ID: u16 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalLatticeDecision {
    Accept,
    Reject,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalLatticeNode {
    pub span: Range<usize>,
    pub pos: Option<DataFinePos>,
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

pub fn evaluate_local_lattice(
    resource: &DecodedMorphologyResource<'_>,
    text: &str,
    query_span: Range<usize>,
    query_pos: DataFinePos,
    node_limit: usize,
) -> Result<LocalLatticeReport, LocalLatticeError> {
    if query_span.start >= query_span.end
        || query_span.end > text.len()
        || !text.is_char_boundary(query_span.start)
        || !text.is_char_boundary(query_span.end)
    {
        return Err(LocalLatticeError::InvalidQuerySpan);
    }
    let unknown = UnknownModel::parse(resource)?;
    let nodes = build_nodes(resource, text, &query_span, query_pos, unknown, node_limit)?;
    let completed = best_paths(resource, text.len(), &nodes)?;
    let include_cost = completed
        .iter()
        .filter(|path| path.includes_query)
        .map(|path| path.cost)
        .min();
    let exclude_cost = completed
        .iter()
        .filter(|path| !path.includes_query)
        .map(|path| path.cost)
        .min();
    let decision = match (include_cost, exclude_cost) {
        (Some(include), Some(exclude)) => match include.cmp(&exclude) {
            Ordering::Less => LocalLatticeDecision::Accept,
            Ordering::Greater => LocalLatticeDecision::Reject,
            Ordering::Equal => LocalLatticeDecision::Ambiguous,
        },
        (Some(_), None) => LocalLatticeDecision::Accept,
        (None, Some(_)) => LocalLatticeDecision::Reject,
        (None, None) => return Err(LocalLatticeError::NoCompletePath),
    };
    let cost_margin = include_cost
        .zip(exclude_cost)
        .map(|(include, exclude)| {
            include
                .checked_sub(exclude)
                .and_then(i64::checked_abs)
                .ok_or(LocalLatticeError::CostOverflow)
        })
        .transpose()?;
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
        include_cost,
        exclude_cost,
        cost_margin,
        node_count: nodes.len(),
        paths,
    })
}

#[derive(Clone, Copy, Debug)]
struct UnknownModel {
    invoke: bool,
    group: bool,
    max_length: usize,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
}

impl UnknownModel {
    fn parse(resource: &DecodedMorphologyResource<'_>) -> Result<Self, LocalLatticeError> {
        let char_def = std::str::from_utf8(resource.char_def())
            .map_err(|_| LocalLatticeError::InvalidUnknownModel)?;
        let definition = char_def
            .lines()
            .map(strip_comment)
            .find(|line| line.split_whitespace().next() == Some("HANGUL"))
            .ok_or(LocalLatticeError::InvalidUnknownModel)?;
        let fields = definition.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(LocalLatticeError::InvalidUnknownModel);
        }
        let invoke = parse_flag(fields[1])?;
        let group = parse_flag(fields[2])?;
        let max_length = fields[3]
            .parse::<usize>()
            .map_err(|_| LocalLatticeError::InvalidUnknownModel)?;
        if max_length == 0 {
            return Err(LocalLatticeError::InvalidUnknownModel);
        }

        let unk_def = std::str::from_utf8(resource.unk_def())
            .map_err(|_| LocalLatticeError::InvalidUnknownModel)?;
        let fields = unk_def
            .lines()
            .map(strip_comment)
            .find(|line| line.split(',').next() == Some("HANGUL"))
            .map(|line| line.split(',').collect::<Vec<_>>())
            .filter(|fields| fields.len() >= 4)
            .ok_or(LocalLatticeError::InvalidUnknownModel)?;
        let model = Self {
            invoke,
            group,
            max_length,
            left_id: parse_unknown_field(fields[1])?,
            right_id: parse_unknown_field(fields[2])?,
            word_cost: parse_unknown_field(fields[3])?,
        };
        if resource
            .connection_cost(model.right_id, BOS_EOS_CONTEXT_ID)
            .is_none()
            || resource
                .connection_cost(BOS_EOS_CONTEXT_ID, model.left_id)
                .is_none()
        {
            return Err(LocalLatticeError::InvalidUnknownModel);
        }
        Ok(model)
    }
}

fn parse_flag(value: &str) -> Result<bool, LocalLatticeError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(LocalLatticeError::InvalidUnknownModel),
    }
}

fn parse_unknown_field<T: std::str::FromStr>(value: &str) -> Result<T, LocalLatticeError> {
    value
        .parse()
        .map_err(|_| LocalLatticeError::InvalidUnknownModel)
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map_or(line, |(content, _)| content)
        .trim()
}

#[derive(Clone, Debug)]
struct Node {
    span: Range<usize>,
    pos: Option<DataFinePos>,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    unknown: bool,
    query_match: bool,
}

impl Node {
    fn dictionary(
        span: Range<usize>,
        analysis: MorphologyAnalysis,
        query_span: &Range<usize>,
        query_pos: DataFinePos,
    ) -> Self {
        let query_match = analysis.pos == query_pos
            && span.start <= query_span.start
            && span.end >= query_span.end;
        Self {
            span,
            pos: Some(analysis.pos),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            unknown: false,
            query_match,
        }
    }

    fn unknown(span: Range<usize>, model: UnknownModel) -> Self {
        Self {
            span,
            pos: None,
            left_id: model.left_id,
            right_id: model.right_id,
            word_cost: model.word_cost,
            unknown: true,
            query_match: false,
        }
    }

    fn summary(&self) -> LocalLatticeNode {
        LocalLatticeNode {
            span: self.span.clone(),
            pos: self.pos,
            word_cost: self.word_cost,
            unknown: self.unknown,
        }
    }
}

fn build_nodes(
    resource: &DecodedMorphologyResource<'_>,
    text: &str,
    query_span: &Range<usize>,
    query_pos: DataFinePos,
    unknown: UnknownModel,
    node_limit: usize,
) -> Result<Vec<Node>, LocalLatticeError> {
    let mut nodes = Vec::new();
    for (start, character) in text.char_indices() {
        let before_dictionary = nodes.len();
        resource.common_prefixes(&text.as_bytes()[start..], |length, analyses| {
            let end = start + length;
            if end <= text.len() && text.is_char_boundary(end) {
                nodes.extend(
                    analyses.iter().copied().map(|analysis| {
                        Node::dictionary(start..end, analysis, query_span, query_pos)
                    }),
                );
            }
        });
        let has_dictionary = nodes.len() > before_dictionary;
        if is_hangul(character) && (unknown.invoke || !has_dictionary) {
            add_unknown_nodes(&mut nodes, text, start, unknown);
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
            left.pos,
            left.left_id,
            left.right_id,
            left.word_cost,
        )
            .cmp(&(
                right.span.start,
                right.span.end,
                right.unknown,
                right.pos,
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

fn add_unknown_nodes(nodes: &mut Vec<Node>, text: &str, start: usize, model: UnknownModel) {
    let ends = text[start..]
        .char_indices()
        .take_while(|(_, character)| is_hangul(*character))
        .map(|(offset, character)| start + offset + character.len_utf8())
        .collect::<Vec<_>>();
    for end in ends.iter().copied().take(model.max_length) {
        nodes.push(Node::unknown(start..end, model));
    }
    if model.group && ends.len() > model.max_length {
        if let Some(end) = ends.last().copied() {
            nodes.push(Node::unknown(start..end, model));
        }
    }
}

fn is_hangul(character: char) -> bool {
    matches!(
        character,
        '\u{ac00}'..='\u{d7a3}' | '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}'
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PathState {
    cost: i64,
    includes_query: bool,
    nodes: Vec<usize>,
}

fn best_paths(
    resource: &DecodedMorphologyResource<'_>,
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
