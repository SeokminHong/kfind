use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use kfind_morph::{ConstraintResolver, PreparedTokenGraph};
use kfind_query::{CandidateConsumption, CandidateProgram, QueryPlan};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use super::context::needs_nominal_derivation;

pub(super) const MAX_PREPARED_EXACT_TOKEN_PROGRAMS: usize = 8;
const MAX_PREPARED_EXACT_TOKEN_GRAPHS: usize = 64;

#[derive(Debug, Default)]
pub(super) struct PreparedExactTokenGraphs {
    entries: [Vec<PreparedExactTokenGraph>; 4],
    minimum_text_bytes: usize,
    maximum_text_bytes: usize,
    remaining_memory: AtomicUsize,
}

#[derive(Debug)]
struct PreparedExactTokenGraph {
    text: Box<str>,
    graph: OnceLock<Option<Arc<PreparedTokenGraph>>>,
}

impl PreparedExactTokenGraphs {
    pub(super) fn build(plan: &QueryPlan, anchor_memory_usage: usize) -> Self {
        let program_count = plan.atoms.iter().try_fold(0usize, |count, atom| {
            count
                .checked_add(atom.programs.len())
                .filter(|count| *count <= MAX_PREPARED_EXACT_TOKEN_PROGRAMS)
        });
        if program_count.is_none() {
            return Self::default();
        }
        let mut keys = plan
            .atoms
            .iter()
            .flat_map(|atom| &atom.programs)
            .filter_map(exact_token_graph_key)
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys.dedup();
        if keys.len() > MAX_PREPARED_EXACT_TOKEN_GRAPHS {
            return Self::default();
        }

        let mut result = Self::default();
        let mut remaining_memory = plan
            .limits
            .max_matcher_bytes
            .saturating_sub(anchor_memory_usage);
        for (text, include_nominal_copula, include_nominal_derivation_predicate) in keys {
            let memory_usage = text
                .len()
                .saturating_add(std::mem::size_of::<PreparedExactTokenGraph>());
            if memory_usage > remaining_memory {
                continue;
            }
            remaining_memory -= memory_usage;
            result.entries
                [prepared_token_slot(include_nominal_copula, include_nominal_derivation_predicate)]
            .push(PreparedExactTokenGraph {
                text: text.into_owned().into_boxed_str(),
                graph: OnceLock::new(),
            });
        }
        for entries in &mut result.entries {
            entries.shrink_to_fit();
        }
        result.remaining_memory = AtomicUsize::new(remaining_memory);
        result.minimum_text_bytes = result
            .entries
            .iter()
            .flatten()
            .map(|entry| entry.text.len())
            .min()
            .unwrap_or(0);
        result.maximum_text_bytes = result
            .entries
            .iter()
            .flatten()
            .map(|entry| entry.text.len())
            .max()
            .unwrap_or(0);
        result
    }

    pub(super) fn get_or_prepare(
        &self,
        text: &str,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
        resolver: &ConstraintResolver,
        node_limit: usize,
    ) -> Option<Arc<PreparedTokenGraph>> {
        if text.len() < self.minimum_text_bytes || text.len() > self.maximum_text_bytes {
            return None;
        }
        let entries = &self.entries
            [prepared_token_slot(include_nominal_copula, include_nominal_derivation_predicate)];
        let index = entries
            .binary_search_by(|entry| entry.text.as_ref().cmp(text))
            .ok()?;
        entries[index]
            .graph
            .get_or_init(|| {
                let graph = resolver
                    .prepare_token_graph_for_candidate(
                        text,
                        node_limit,
                        include_nominal_copula,
                        include_nominal_derivation_predicate,
                    )
                    .ok()?;
                self.reserve_graph_memory(graph.memory_usage())
                    .then(|| Arc::new(graph))
            })
            .clone()
    }

    fn reserve_graph_memory(&self, memory_usage: usize) -> bool {
        self.remaining_memory
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
                remaining.checked_sub(memory_usage)
            })
            .is_ok()
    }
}

fn exact_token_graph_key(branch: &CandidateProgram) -> Option<(Cow<'_, str>, bool, bool)> {
    let patterns = branch.structural_patterns();
    if patterns.is_empty() {
        return None;
    }
    let raw_text = std::str::from_utf8(&branch.anchor).ok()?;
    let text = if is_nfc(raw_text) {
        Cow::Borrowed(raw_text)
    } else {
        Cow::Owned(raw_text.nfc().collect())
    };
    let include_nominal_derivation_predicate = needs_nominal_derivation(patterns);
    let include_nominal_copula = matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: kfind_morph::PredicatePos::Copula,
            ..
        }
    );
    Some((
        text,
        include_nominal_copula,
        include_nominal_derivation_predicate,
    ))
}

const fn prepared_token_slot(
    include_nominal_copula: bool,
    include_nominal_derivation_predicate: bool,
) -> usize {
    include_nominal_copula as usize | ((include_nominal_derivation_predicate as usize) << 1)
}

#[cfg(test)]
impl PreparedExactTokenGraphs {
    pub(super) fn is_empty(&self) -> bool {
        self.entries.iter().all(Vec::is_empty)
    }

    pub(super) fn preparation_state(&self, text: &str) -> Option<Option<bool>> {
        self.entries[prepared_token_slot(false, false)]
            .iter()
            .find(|entry| entry.text.as_ref() == text)
            .map(|entry| entry.graph.get().map(Option::is_some))
    }
}
