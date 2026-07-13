use super::{
    BOS_EOS_CONTEXT_ID, LocalLatticeDecision, LocalLatticeError, LocalLatticeResource, Node,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct LocalLatticeCosts {
    pub include: Option<i64>,
    pub exclude: Option<i64>,
}

impl LocalLatticeCosts {
    pub fn decision(self) -> Result<LocalLatticeDecision, LocalLatticeError> {
        Ok(match (self.include, self.exclude) {
            (Some(include), Some(exclude)) => match include.cmp(&exclude) {
                std::cmp::Ordering::Less => LocalLatticeDecision::Accept,
                std::cmp::Ordering::Greater => LocalLatticeDecision::Reject,
                std::cmp::Ordering::Equal => LocalLatticeDecision::Ambiguous,
            },
            (Some(_), None) => LocalLatticeDecision::Accept,
            (None, Some(_)) => LocalLatticeDecision::Reject,
            (None, None) => return Err(LocalLatticeError::NoCompletePath),
        })
    }

    pub fn margin(self) -> Result<Option<i64>, LocalLatticeError> {
        self.include
            .zip(self.exclude)
            .map(|(include, exclude)| {
                include
                    .checked_sub(exclude)
                    .and_then(i64::checked_abs)
                    .ok_or(LocalLatticeError::CostOverflow)
            })
            .transpose()
    }
}

pub(super) fn best_costs(
    resource: &dyn LocalLatticeResource,
    text_len: usize,
    nodes: &[Node],
) -> Result<LocalLatticeCosts, LocalLatticeError> {
    let mut ending_at = vec![Vec::<usize>::new(); text_len + 1];
    for (index, node) in nodes.iter().enumerate() {
        ending_at[node.span.end].push(index);
    }
    let mut costs = vec![[None::<i64>; 2]; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if node.span.start == 0 {
            if let Some(connection) = resource.connection_cost(BOS_EOS_CONTEXT_ID, node.left_id) {
                update_minimum(
                    &mut costs[index][usize::from(node.query_match)],
                    i64::from(connection) + i64::from(node.word_cost),
                );
            }
        }
        for predecessor in &ending_at[node.span.start] {
            let Some(connection) = resource
                .connection_cost(nodes[*predecessor].right_id, node.left_id)
                .map(i64::from)
            else {
                continue;
            };
            for includes_query in [false, true] {
                let Some(predecessor_cost) = costs[*predecessor][usize::from(includes_query)]
                else {
                    continue;
                };
                let candidate = predecessor_cost
                    .checked_add(connection)
                    .and_then(|cost| cost.checked_add(i64::from(node.word_cost)))
                    .ok_or(LocalLatticeError::CostOverflow)?;
                update_minimum(
                    &mut costs[index][usize::from(includes_query || node.query_match)],
                    candidate,
                );
            }
        }
    }

    let mut complete = [None::<i64>; 2];
    for index in &ending_at[text_len] {
        let Some(connection) = resource
            .connection_cost(nodes[*index].right_id, BOS_EOS_CONTEXT_ID)
            .map(i64::from)
        else {
            continue;
        };
        for includes_query in [false, true] {
            let Some(path_cost) = costs[*index][usize::from(includes_query)] else {
                continue;
            };
            let candidate = path_cost
                .checked_add(connection)
                .ok_or(LocalLatticeError::CostOverflow)?;
            update_minimum(&mut complete[usize::from(includes_query)], candidate);
        }
    }
    let result = LocalLatticeCosts {
        exclude: complete[0],
        include: complete[1],
    };
    result.decision()?;
    Ok(result)
}

fn update_minimum(current: &mut Option<i64>, candidate: i64) {
    if current.is_none_or(|cost| candidate < cost) {
        *current = Some(candidate);
    }
}
