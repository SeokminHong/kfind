use crate::structure::{ConstraintUnavailable, StructuralEvidence};
use kfind_data::{
    ComponentAnalysisRef, ComponentPart, ComponentPos as StructuralPos, ComponentResource,
    DataFinePos,
};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Unit {
    pub(super) span: Range<usize>,
    pub(super) pos: DataFinePos,
    pub(super) evidence: StructuralEvidence,
    pub(super) from_whole_nominal: bool,
}

#[derive(Debug)]
pub(super) struct StartGraph<T> {
    items: Vec<T>,
    start_offsets: Box<[usize]>,
}

impl<T> Default for StartGraph<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            start_offsets: Box::default(),
        }
    }
}

impl<T> StartGraph<T> {
    pub(super) fn from_sorted_by(
        text_len: usize,
        items: Vec<T>,
        start_of: impl Copy + Fn(&T) -> usize,
    ) -> Self {
        debug_assert!(
            items
                .windows(2)
                .all(|pair| start_of(&pair[0]) <= start_of(&pair[1]))
        );
        debug_assert!(items.iter().all(|item| start_of(item) <= text_len));

        let mut start_offsets = Vec::with_capacity(text_len + 2);
        let mut item_index = 0;
        for start in 0..text_len + 2 {
            while items
                .get(item_index)
                .is_some_and(|item| start_of(item) < start)
            {
                item_index += 1;
            }
            start_offsets.push(item_index);
        }
        Self {
            items,
            start_offsets: start_offsets.into_boxed_slice(),
        }
    }

    pub(super) fn all(&self) -> &[T] {
        &self.items
    }

    pub(super) fn starting_range(&self, start: usize) -> Range<usize> {
        let Some(next) = start.checked_add(1) else {
            return 0..0;
        };
        let (Some(&range_start), Some(&range_end)) =
            (self.start_offsets.get(start), self.start_offsets.get(next))
        else {
            return 0..0;
        };
        range_start..range_end
    }

    pub(super) fn starting_at(&self, start: usize) -> &[T] {
        &self.items[self.starting_range(start)]
    }

    pub(super) fn memory_usage(&self) -> usize {
        self.items.capacity() * std::mem::size_of::<T>()
            + self.start_offsets.len() * std::mem::size_of::<usize>()
    }
}

pub(super) type UnitGraph = StartGraph<Unit>;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct UnitPathCost {
    units: usize,
    runtime_components: usize,
}

impl UnitPathCost {
    pub(super) fn append(self, unit: &Unit) -> Self {
        Self {
            units: self.units + 1,
            runtime_components: self.runtime_components
                + usize::from(unit.evidence != StructuralEvidence::SourceComponent),
        }
    }

    pub(super) fn combine(self, suffix: Self) -> Self {
        Self {
            units: self.units + suffix.units,
            runtime_components: self.runtime_components + suffix.runtime_components,
        }
    }
}

impl StartGraph<Unit> {
    pub(super) fn minimum_path_len(
        &self,
        span: &Range<usize>,
        accepts: impl Copy + Fn(DataFinePos) -> bool,
    ) -> Option<usize> {
        if span.is_empty() {
            return Some(0);
        }
        let mut costs = vec![None; span.len() + 1];
        costs[0] = Some(0_usize);
        for offset in 0..span.len() {
            let Some(cost) = costs[offset] else {
                continue;
            };
            let start = span.start + offset;
            for unit in self
                .starting_at(start)
                .iter()
                .filter(|unit| unit.span.end <= span.end && accepts(unit.pos))
            {
                let end = unit.span.end - span.start;
                let next = cost + 1;
                if costs[end].is_none_or(|current| next < current) {
                    costs[end] = Some(next);
                }
            }
        }
        costs[span.len()]
    }

    pub(super) fn contains_on_preferred_path(
        &self,
        core: &Range<usize>,
        selected: &Range<usize>,
        accepts: impl Copy + Fn(&Unit) -> bool,
    ) -> bool {
        if core.start < selected.start
            || core.end > selected.end
            || core.is_empty()
            || selected.is_empty()
        {
            return false;
        }
        let eligible = |unit: &Unit| {
            unit.span.end <= selected.end
                && accepts(unit)
                && matches!(
                    unit.evidence,
                    StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
                )
        };

        let span_len = selected.len();
        let mut prefix_costs = vec![None; span_len + 1];
        prefix_costs[0] = Some(UnitPathCost::default());
        for offset in 0..span_len {
            let Some(prefix) = prefix_costs[offset] else {
                continue;
            };
            for unit in self
                .starting_at(selected.start + offset)
                .iter()
                .filter(|unit| eligible(unit))
            {
                let end = unit.span.end - selected.start;
                let candidate = prefix.append(unit);
                if prefix_costs[end].is_none_or(|current| candidate < current) {
                    prefix_costs[end] = Some(candidate);
                }
            }
        }
        let Some(best) = prefix_costs[span_len] else {
            return false;
        };

        let mut suffix_costs = vec![None; span_len + 1];
        suffix_costs[span_len] = Some(UnitPathCost::default());
        for offset in (0..span_len).rev() {
            for unit in self
                .starting_at(selected.start + offset)
                .iter()
                .filter(|unit| eligible(unit))
            {
                let end = unit.span.end - selected.start;
                let Some(suffix) = suffix_costs[end] else {
                    continue;
                };
                let candidate = UnitPathCost::default().append(unit).combine(suffix);
                if suffix_costs[offset].is_none_or(|current| candidate < current) {
                    suffix_costs[offset] = Some(candidate);
                }
            }
        }

        let core_start = core.start - selected.start;
        let core_end = core.end - selected.start;
        self.starting_at(core.start)
            .iter()
            .filter(|unit| unit.span == *core && eligible(unit))
            .any(|unit| {
                let (Some(prefix), Some(suffix)) =
                    (prefix_costs[core_start], suffix_costs[core_end])
                else {
                    return false;
                };
                prefix.append(unit).combine(suffix) == best
            })
    }
}
#[derive(Debug)]
pub(super) struct Edge<'a> {
    pub(super) span: Range<usize>,
    pub(super) positions: &'a [StructuralPos],
    pub(super) analysis: Option<ComponentAnalysisRef<'a>>,
}

#[derive(Debug)]
pub(super) struct EdgeGraph<'a> {
    edges: StartGraph<Edge<'a>>,
}

impl<'a> EdgeGraph<'a> {
    pub(super) fn collect(
        resource: &'a ComponentResource,
        text: &str,
        node_limit: usize,
    ) -> Result<Self, ConstraintUnavailable> {
        let mut edges = Vec::new();
        for start in text.char_indices().map(|(offset, _)| offset) {
            let mut actual = edges.len();
            resource.common_prefix_analysis_refs(&text.as_bytes()[start..], |length, analysis| {
                if length == 0 || start + length > text.len() {
                    return;
                }
                actual += 1;
                if actual <= node_limit {
                    edges.push(Edge {
                        span: start..start + length,
                        positions: analysis.positions(),
                        analysis: Some(analysis),
                    });
                }
            });
            if actual > node_limit {
                return Err(ConstraintUnavailable::NodeLimit {
                    actual,
                    limit: node_limit,
                });
            }
        }

        Ok(Self::from_edges(text.len(), edges))
    }

    fn from_edges(text_len: usize, edges: Vec<Edge<'a>>) -> Self {
        Self {
            edges: StartGraph::from_sorted_by(text_len, edges, |edge| edge.span.start),
        }
    }

    #[cfg(test)]
    pub(super) fn from_test_edges(
        text_len: usize,
        edges: Vec<(Range<usize>, &'a [StructuralPos])>,
    ) -> Self {
        let edges = edges
            .into_iter()
            .map(|(span, positions)| Edge {
                span,
                positions,
                analysis: None,
            })
            .collect();
        Self::from_edges(text_len, edges)
    }

    pub(super) fn edges(&self) -> &[Edge<'a>] {
        self.edges.all()
    }

    pub(super) fn starting_at(&self, start: usize) -> &[Edge<'a>] {
        self.edges.starting_at(start)
    }

    pub(super) fn starting_range(&self, start: usize) -> Range<usize> {
        self.edges.starting_range(start)
    }

    pub(super) fn positions<'b>(&self, edge: &Edge<'b>) -> &'b [StructuralPos] {
        edge.positions
    }

    pub(super) fn components<'b>(
        &self,
        edge: &Edge<'b>,
    ) -> impl Iterator<Item = ComponentPart<'b>> + Clone + 'b {
        edge.analysis
            .into_iter()
            .flat_map(ComponentAnalysisRef::components)
    }
}
