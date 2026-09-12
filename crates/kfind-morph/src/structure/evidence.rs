use crate::structure::graph::{EdgeGraph, Unit, UnitGraph};
use crate::structure::lexical::numeric_unit_path;
use crate::structure::paths::nominal::{
    adnominal_derivation_suffix_starts, derivational_suffix_starts, nominal_copula_hosts,
    nominal_derivation_predicate_prefixes, runtime_nominal_derivation_spans,
};
use crate::structure::paths::numeral::{hangul_numeral_spans, numeral_sequence_spans};
use crate::structure::paths::predicate::{
    PredicateComponent, attached_auxiliary_spans, compound_predicate_components,
    leading_predicate_spans,
};
use crate::structure::paths::{
    CommonPathFacts, complete_edges, forward_positions, forward_positions_with_prefix,
};
use crate::structure::{ConstraintUnavailable, StructuralEvidence};
use kfind_data::{ComponentPos as StructuralPos, ComponentResource, DataFinePos};
use std::ops::Range;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NominalSourceComponent {
    host_start: usize,
    host_end: usize,
    component_start: usize,
    component_end: usize,
    pos: DataFinePos,
}

#[derive(Debug, Default)]
pub(super) struct TokenEvidence {
    pub(super) units: UnitGraph,
    unambiguous_nominal_source_components: Box<[NominalSourceComponent]>,
    pub(super) nominal_particle_hosts: Box<[Range<usize>]>,
    pub(super) complete_nominal_particle_host: Option<Range<usize>>,
    pub(super) has_whole_nominal_source_components: bool,
    pub(super) runtime_spans: Vec<Range<usize>>,
    pub(super) compound_predicate_components: Box<[PredicateComponent]>,
    pub(super) attached_auxiliary_spans: Box<[Range<usize>]>,
    pub(super) nominal_copula_hosts: Box<[Range<usize>]>,
    pub(super) adnominal_ends: Vec<usize>,
    pub(super) has_complete_path: bool,
    pub(super) leading_predicate_spans: Box<[Range<usize>]>,
    pub(super) runtime_nominal_derivation_spans: Box<[Range<usize>]>,
    pub(super) nominal_derivation_predicate_prefixes: Box<[Range<usize>]>,
    pub(super) derivational_suffix_starts: Box<[usize]>,
    pub(super) adnominal_derivation_suffix_starts: Box<[usize]>,
    pub(super) numeric_spans: Box<[Range<usize>]>,
    pub(super) numeric_dependent_tail: Option<Range<usize>>,
    pub(super) has_numeral_sequence: bool,
}

impl TokenEvidence {
    pub(super) fn memory_usage(&self) -> usize {
        self.units.memory_usage()
            + self.unambiguous_nominal_source_components.len()
                * std::mem::size_of::<NominalSourceComponent>()
            + self.nominal_particle_hosts.len() * std::mem::size_of::<Range<usize>>()
            + self.runtime_spans.capacity() * std::mem::size_of::<Range<usize>>()
            + self.compound_predicate_components.len() * std::mem::size_of::<PredicateComponent>()
            + self.attached_auxiliary_spans.len() * std::mem::size_of::<Range<usize>>()
            + self.nominal_copula_hosts.len() * std::mem::size_of::<Range<usize>>()
            + self.adnominal_ends.capacity() * std::mem::size_of::<usize>()
            + self.leading_predicate_spans.len() * std::mem::size_of::<Range<usize>>()
            + self.runtime_nominal_derivation_spans.len() * std::mem::size_of::<Range<usize>>()
            + self.nominal_derivation_predicate_prefixes.len() * std::mem::size_of::<Range<usize>>()
            + self.derivational_suffix_starts.len() * std::mem::size_of::<usize>()
            + self.adnominal_derivation_suffix_starts.len() * std::mem::size_of::<usize>()
            + self.numeric_spans.len() * std::mem::size_of::<Range<usize>>()
    }

    pub(super) fn collect(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<Self, ConstraintUnavailable> {
        if text.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            Self::collect_mode::<true>(
                resource,
                text,
                node_limit,
                include_attached_auxiliary,
                include_nominal_copula,
                include_nominal_derivation_predicate,
            )
        } else {
            Self::collect_mode::<false>(
                resource,
                text,
                node_limit,
                include_attached_auxiliary,
                include_nominal_copula,
                include_nominal_derivation_predicate,
            )
        }
    }

    fn collect_mode<const NUMERIC: bool>(
        resource: &ComponentResource,
        text: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<Self, ConstraintUnavailable> {
        let numeric_end = if NUMERIC {
            text.bytes().take_while(u8::is_ascii_digit).count()
        } else {
            0
        };
        let numeric_path = if NUMERIC {
            numeric_unit_path(resource, text)
        } else {
            None
        };
        let numeric_unit = numeric_path.as_ref().map(|path| path.unit.clone());
        let graph = EdgeGraph::collect(resource, text, node_limit)?;
        let path_facts = CommonPathFacts::collect(text, &graph);
        let nominal_paths = path_facts.nominal_paths(text);
        let edges = graph.edges();
        let mixed_numeral_spans = if NUMERIC
            && numeric_unit.is_none()
            && graph
                .starting_at(numeric_end)
                .iter()
                .any(|edge| graph.positions(edge) == [StructuralPos::NR])
        {
            numeral_sequence_spans(text.len(), numeric_end, &graph, true)
        } else {
            Vec::new()
        };
        let numeric_prefix = numeric_unit
            .as_ref()
            .map(|unit| unit.start)
            .or_else(|| (!mixed_numeral_spans.is_empty()).then_some(numeric_end));
        let forward = numeric_prefix.map_or_else(
            || forward_positions(text.len(), &graph),
            |prefix_end| forward_positions_with_prefix(text.len(), &graph, prefix_end),
        );
        let complete = complete_edges(text.len(), &graph, &forward);
        let has_complete_path = forward[text.len()];
        let leading_predicate_spans = leading_predicate_spans(&graph, &path_facts.ending_suffix);
        let compound_predicate_components = compound_predicate_components(
            text.len(),
            &graph,
            &path_facts.predicate_connective_boundaries,
        );
        let runtime_nominal_derivation_spans = runtime_nominal_derivation_spans(&graph, &complete);
        let nominal_derivation_predicate_prefixes = if include_nominal_derivation_predicate {
            nominal_derivation_predicate_prefixes(text.len(), &graph, &path_facts.ending_suffix)
        } else {
            Box::default()
        };
        let derivational_suffix_starts = derivational_suffix_starts(&graph, &complete);
        let adnominal_derivation_suffix_starts =
            adnominal_derivation_suffix_starts(text.len(), &graph, &path_facts.nominal_prefix);
        let attached_auxiliary_spans = if include_attached_auxiliary {
            attached_auxiliary_spans(
                text.len(),
                &graph,
                &path_facts.predicate_connective_boundaries,
                &path_facts.ending_suffix,
            )
        } else {
            Box::default()
        };
        let nominal_copula_hosts = if include_nominal_copula {
            nominal_copula_hosts(text, &graph, &path_facts.nominal_prefix)
        } else {
            Box::default()
        };
        let mut units = Vec::new();
        let mut atomic_nominal_analyses = Vec::new();
        let mut nominal_source_component_candidates = Vec::new();
        let mut has_whole_nominal_source_components = false;
        let mut runtime_spans = Vec::new();
        let mut adnominal_ends = Vec::new();
        for (index, edge) in edges.iter().enumerate() {
            let eligible = if has_complete_path {
                complete[index]
            } else {
                forward[edge.span.start]
            };
            if !eligible {
                continue;
            }
            runtime_spans.push(edge.span.clone());
            let positions = graph.positions(edge);
            if positions.last() == Some(&StructuralPos::ETM) {
                adnominal_ends.push(edge.span.end);
            }
            let whole_edge = edge.span == (0..text.len());
            let has_one_position = positions
                .iter()
                .filter_map(|position| position.fine_pos())
                .count()
                == 1;
            let nominal_analysis_pos = has_one_position
                .then(|| positions.iter().find_map(|position| position.fine_pos()))
                .flatten()
                .filter(|pos| pos.is_nominal());
            let whole_nominal_analysis = whole_edge && nominal_analysis_pos.is_some();
            let components = graph.components(edge);
            if let Some(pos) = nominal_analysis_pos
                && components.clone().next().is_none()
            {
                atomic_nominal_analyses.push((edge.span.start, edge.span.end, pos));
            }
            for pos in positions.iter().filter_map(|position| position.fine_pos()) {
                units.push(Unit {
                    span: edge.span.clone(),
                    pos,
                    evidence: if whole_edge && has_one_position {
                        StructuralEvidence::Whole
                    } else {
                        StructuralEvidence::RuntimeComponent
                    },
                    from_whole_nominal: false,
                });
            }
            for component in components {
                if component.pos == "ETM" {
                    adnominal_ends.push(edge.span.start + component.span.end);
                }
                let Some(pos) = DataFinePos::parse(component.pos) else {
                    continue;
                };
                let span =
                    edge.span.start + component.span.start..edge.span.start + component.span.end;
                let from_whole_nominal = whole_nominal_analysis && pos.is_nominal();
                has_whole_nominal_source_components |= from_whole_nominal;
                if pos.is_nominal()
                    && span.start == edge.span.start
                    && let Some(host_pos) = nominal_analysis_pos
                {
                    nominal_source_component_candidates.push((
                        NominalSourceComponent {
                            host_start: edge.span.start,
                            host_end: edge.span.end,
                            component_start: span.start,
                            component_end: span.end,
                            pos,
                        },
                        host_pos,
                    ));
                }
                units.push(Unit {
                    span,
                    pos,
                    evidence: StructuralEvidence::SourceComponent,
                    from_whole_nominal,
                });
            }
        }
        if let Some(unit) = numeric_unit.as_ref() {
            for (index, edge) in edges.iter().enumerate() {
                let eligible = if has_complete_path {
                    complete[index]
                } else {
                    forward[edge.span.start]
                };
                if !eligible {
                    continue;
                }
                if edge.span == *unit && graph.positions(edge).contains(&StructuralPos::NNBC) {
                    units.push(Unit {
                        span: unit.clone(),
                        pos: DataFinePos::Nnb,
                        evidence: if edge.span == (0..text.len()) {
                            StructuralEvidence::Whole
                        } else {
                            StructuralEvidence::RuntimeComponent
                        },
                        from_whole_nominal: false,
                    });
                }
                for component in graph.components(edge).filter(|part| {
                    part.pos == "NNBC"
                        && edge.span.start + part.span.start == unit.start
                        && edge.span.start + part.span.end == unit.end
                }) {
                    units.push(Unit {
                        span: edge.span.start + component.span.start
                            ..edge.span.start + component.span.end,
                        pos: DataFinePos::Nnb,
                        evidence: StructuralEvidence::SourceComponent,
                        from_whole_nominal: false,
                    });
                }
            }
        }
        let (numeric_spans, has_numeral_sequence) = if let Some(unit) = numeric_unit {
            (vec![unit].into_boxed_slice(), false)
        } else if !mixed_numeral_spans.is_empty() {
            (mixed_numeral_spans.into_boxed_slice(), true)
        } else if !NUMERIC
            && graph
                .starting_at(0)
                .iter()
                .any(|edge| graph.positions(edge) == [StructuralPos::NR])
        {
            let spans = hangul_numeral_spans(text.len(), &graph);
            let has_numeral_sequence = !spans.is_empty();
            (spans.into_boxed_slice(), has_numeral_sequence)
        } else {
            (Vec::new().into_boxed_slice(), false)
        };
        units.sort_unstable_by_key(|unit| {
            (
                unit.span.start,
                unit.span.end,
                unit.pos,
                unit.evidence as u8,
                !unit.from_whole_nominal,
            )
        });
        units.dedup_by(|current, previous| {
            let same_unit = current.span == previous.span
                && current.pos == previous.pos
                && current.evidence == previous.evidence;
            if same_unit {
                let from_whole_nominal = current.from_whole_nominal || previous.from_whole_nominal;
                current.from_whole_nominal = from_whole_nominal;
                previous.from_whole_nominal = from_whole_nominal;
            }
            same_unit
        });
        runtime_spans.sort_unstable_by_key(|span| (span.start, span.end));
        runtime_spans.dedup();
        adnominal_ends.sort_unstable();
        adnominal_ends.dedup();
        atomic_nominal_analyses.sort_unstable();
        atomic_nominal_analyses.dedup();
        let mut unambiguous_nominal_source_components = nominal_source_component_candidates
            .into_iter()
            .filter_map(|(component, host_pos)| {
                atomic_nominal_analyses
                    .binary_search(&(component.host_start, component.host_end, host_pos))
                    .is_err()
                    .then_some(component)
            })
            .collect::<Vec<_>>();
        unambiguous_nominal_source_components.sort_unstable();
        unambiguous_nominal_source_components.dedup();
        Ok(Self {
            units: UnitGraph::from_sorted_by(text.len(), units, |unit| unit.span.start),
            unambiguous_nominal_source_components: unambiguous_nominal_source_components
                .into_boxed_slice(),
            nominal_particle_hosts: nominal_paths.particle_hosts,
            complete_nominal_particle_host: nominal_paths.complete_particle_host,
            has_whole_nominal_source_components,
            runtime_spans,
            compound_predicate_components,
            attached_auxiliary_spans,
            nominal_copula_hosts,
            adnominal_ends,
            has_complete_path,
            leading_predicate_spans,
            runtime_nominal_derivation_spans,
            nominal_derivation_predicate_prefixes,
            derivational_suffix_starts,
            adnominal_derivation_suffix_starts,
            numeric_spans,
            numeric_dependent_tail: numeric_path.and_then(|path| path.dependent_tail),
            has_numeral_sequence,
        })
    }

    pub(super) fn has_whole(&self, pos: DataFinePos) -> bool {
        self.units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos == pos)
    }

    pub(super) fn has_whole_modifier(&self) -> bool {
        [DataFinePos::Mm, DataFinePos::Mag, DataFinePos::Maj]
            .into_iter()
            .any(|pos| self.has_whole(pos))
    }

    pub(super) fn has_whole_predicate(&self) -> bool {
        self.units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos.is_predicate())
    }

    pub(super) fn has_whole_analysis(&self, span: &Range<usize>) -> bool {
        self.units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.span == *span)
    }

    pub(super) fn has_whole_nominal_source_component(
        &self,
        span: &Range<usize>,
        pos: DataFinePos,
    ) -> bool {
        self.units.all().iter().any(|unit| {
            unit.from_whole_nominal
                && unit.span == *span
                && unit.pos == pos
                && unit.evidence == StructuralEvidence::SourceComponent
        })
    }

    pub(super) fn has_unambiguous_nominal_source_component(
        &self,
        host: &Range<usize>,
        component: &Range<usize>,
        pos: DataFinePos,
    ) -> bool {
        self.unambiguous_nominal_source_components
            .binary_search(&NominalSourceComponent {
                host_start: host.start,
                host_end: host.end,
                component_start: component.start,
                component_end: component.end,
                pos,
            })
            .is_ok()
    }

    pub(super) fn has_predicate_ending_at(&self, end: usize) -> bool {
        self.units
            .all()
            .iter()
            .any(|unit| unit.span.end == end && unit.pos.is_predicate())
    }

    pub(super) fn has_adnominal_ending_at(&self, end: usize) -> bool {
        self.adnominal_ends.binary_search(&end).is_ok()
    }

    pub(super) fn has_nominal_copula_host(&self, span: &Range<usize>) -> bool {
        self.nominal_copula_hosts
            .binary_search_by_key(&(span.start, span.end), |host| (host.start, host.end))
            .is_ok()
    }
}

pub(super) fn minimum_unit_path(
    span: &Range<usize>,
    evidence: &TokenEvidence,
    accepts: impl Copy + Fn(DataFinePos) -> bool,
) -> Option<usize> {
    evidence.units.minimum_path_len(span, accepts)
}
