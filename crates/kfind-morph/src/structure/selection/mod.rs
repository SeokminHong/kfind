use crate::structure::evidence::TokenEvidence;
use crate::structure::lexical::{
    adnominal_suffix_is_supported, complete_ha_predicate_path, complete_nominal_host,
    complete_nominal_particle_host, complete_predicate_connective_ji_path,
    complete_predicate_ending_path, copular_frame, exact_analysis_ends_with_pos,
    exact_analysis_starts_with_pos, has_copular_adnominal_split, has_exact_fine_pos,
    nominal_particle_hosts,
};
use crate::structure::selection::components::{
    attached_nominal_suffix_is_supported, component_is_shadowed_by_predicate,
    compound_predicate_component_is_supported, modifier_led_nominal_component_is_on_preferred_path,
    nominal_component_is_supported, proper_noun_dependent_noun_frame,
    runtime_nominal_component_is_supported,
};
use crate::structure::selection::runtime::{
    composed_nominal_subpath, is_predicate_nominalization, nominal_derivation_before_predicate,
    query_nominal_particle_path, runtime_position_is_supported,
};
use crate::structure::{BoundedTokenContext, ConstraintSupport, StructuralEvidence};
use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern};
use kfind_data::{ComponentPos as StructuralPos, ComponentResource, DataFinePos};
use std::ops::Range;

pub(super) mod components;
pub(super) mod runtime;

#[derive(Clone, Debug)]
pub(in crate::structure) enum StructureSelection {
    Whole,
    Adverb,
    AdjacentDeterminer,
    DeterminerWhole,
    ConnectiveJiNominalFrame {
        fallback: Box<StructureSelection>,
    },
    NominalSpan {
        selected: Range<usize>,
        allow_components: bool,
        allow_whole_nominal_source_components: bool,
    },
    CopularFrame {
        nominal: Range<usize>,
        copula: Range<usize>,
    },
    DependentNoun,
    NumericUnit {
        unit: Range<usize>,
    },
    NumericUnitDependentNoun {
        unit: Range<usize>,
        tail: Range<usize>,
    },
    NumeralSequence {
        fallback: Box<StructureSelection>,
    },
    AdnominalDerivation {
        fallback: Box<StructureSelection>,
    },
    RuntimeCompatible {
        graph_nominal_host: Option<Range<usize>>,
    },
}

impl StructureSelection {
    pub(in crate::structure) fn graph_nominal_host(&self) -> Option<&Range<usize>> {
        match self {
            Self::RuntimeCompatible { graph_nominal_host } => graph_nominal_host.as_ref(),
            Self::NumeralSequence { fallback } => fallback.graph_nominal_host(),
            Self::AdnominalDerivation { fallback } => fallback.graph_nominal_host(),
            Self::ConnectiveJiNominalFrame { fallback } => fallback.graph_nominal_host(),
            _ => None,
        }
    }

    pub(in crate::structure) fn accepts(
        &self,
        support: &ConstraintSupport,
        spans: &CandidateSpans,
        patterns: &[QueryMorphPattern],
        text: &str,
        evidence: &TokenEvidence,
    ) -> bool {
        let Some(pattern) = patterns.get(support.pattern_index) else {
            return false;
        };
        if let Self::ConnectiveJiNominalFrame { fallback } = self {
            return !pattern.fine_pos.is_predicate()
                && fallback.accepts(support, spans, patterns, text, evidence);
        }
        if query_nominal_particle_path(pattern, spans) {
            return true;
        }
        if composed_nominal_subpath(pattern, spans, evidence) {
            return true;
        }
        if nominal_derivation_before_predicate(pattern, spans, evidence) {
            return true;
        }
        let selected_nominal_particle_tail = matches!(
            self,
            Self::NominalSpan { selected, .. } if selected.end == spans.core.start
        ) && !evidence.has_whole_predicate();
        if !selected_nominal_particle_tail
            && compound_predicate_component_is_supported(pattern, spans, evidence)
        {
            return true;
        }
        match self {
            Self::Whole => support.evidence == StructuralEvidence::Whole,
            Self::Adverb => {
                support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Mag
            }
            Self::AdjacentDeterminer => {
                (support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Mm)
                    || !matches!(
                        pattern.fine_pos,
                        DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
                    )
            }
            Self::DeterminerWhole => {
                support.evidence == StructuralEvidence::Whole && pattern.fine_pos.is_nominal()
            }
            Self::ConnectiveJiNominalFrame { .. } => unreachable!("handled before shared rules"),
            Self::NominalSpan {
                selected,
                allow_components,
                allow_whole_nominal_source_components,
            } => {
                (support.evidence == StructuralEvidence::Whole
                    && spans.core == spans.token
                    && spans.consumed == spans.token)
                    || (pattern.fine_pos.is_nominal()
                        && !component_is_shadowed_by_predicate(
                            support.evidence,
                            pattern,
                            spans,
                            evidence,
                        )
                        && ((*allow_whole_nominal_source_components
                            && support.evidence == StructuralEvidence::SourceComponent
                            && evidence.has_whole_nominal_source_component(
                                &spans.core,
                                pattern.fine_pos,
                            ))
                            || (matches!(
                                pattern.fine_pos,
                                DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
                            ) && !evidence.has_whole_analysis(&spans.token)
                                && (modifier_led_nominal_component_is_on_preferred_path(
                                    &spans.core,
                                    selected,
                                    text,
                                    evidence,
                                ) || (selected.start == spans.token.start
                                    && selected.end == spans.core.start
                                    && modifier_led_nominal_component_is_on_preferred_path(
                                        &spans.core,
                                        &spans.token,
                                        text,
                                        evidence,
                                    ))))
                            || spans.core == *selected
                            || (evidence.nominal_particle_hosts.contains(&spans.core)
                                && spans.consumed == spans.token
                                && matches!(
                                    pattern.continuation,
                                    MorphContinuation::NominalParticles
                                ))
                            || (spans.core.start == selected.start
                                && spans.consumed.end == selected.end
                                && evidence.units.all().iter().any(|unit| {
                                    unit.span == (spans.core.end..selected.end)
                                        && unit.pos.is_particle()
                                }))
                            || attached_nominal_suffix_is_supported(pattern, spans, evidence)
                            || ((nominal_component_is_supported(
                                *allow_components,
                                support.evidence,
                                &spans.core,
                                selected,
                                evidence,
                                pattern.fine_pos,
                                &pattern.lexical_form,
                            ) || proper_noun_dependent_noun_frame(
                                pattern, spans, selected, evidence,
                            )) && spans.core.start >= selected.start
                                && spans.core.end <= selected.end
                                && spans.core != *selected)))
                    || (is_predicate_nominalization(pattern)
                        && spans.anchor.start >= selected.start
                        && spans.anchor.end <= selected.end
                        && (spans.consumed.end == spans.token.end
                            || spans.anchor.start > selected.start
                            || spans.anchor.end < selected.end
                            || (spans.anchor != spans.token
                                && evidence.units.all().iter().any(|unit| {
                                    unit.span == spans.token
                                        && unit.evidence == StructuralEvidence::Whole
                                        && unit.pos.is_nominal()
                                }))))
                    || (matches!(pattern.continuation, MorphContinuation::Predicate { .. })
                        && (spans.core.start == selected.start
                            || (pattern.fine_pos == DataFinePos::Vcp
                                && spans.core.start == selected.end
                                && (spans.consumed.end > spans.core.end
                                    || matches!(
                                        pattern.continuation,
                                        MorphContinuation::Predicate {
                                            state: crate::ContinuationState::Terminal,
                                            ..
                                        }
                                    ))))
                        && spans.consumed.end == spans.token.end
                        && runtime_position_is_supported(pattern, spans, text, evidence))
                    || (matches!(
                        pattern.fine_pos,
                        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm
                    ) && spans.core.start == selected.start
                        && (spans.consumed == spans.core || spans.consumed.end == spans.token.end))
            }
            Self::CopularFrame { nominal, copula } => {
                (spans.core == *nominal && pattern.fine_pos.is_nominal())
                    || (spans.core == *copula && pattern.fine_pos == DataFinePos::Vcp)
            }
            Self::DependentNoun => {
                support.evidence == StructuralEvidence::Whole
                    && pattern.fine_pos == DataFinePos::Nnb
            }
            Self::NumericUnit { unit } => {
                matches!(pattern.fine_pos, DataFinePos::Nnb | DataFinePos::Nr)
                    && spans.core == *unit
                    && spans.consumed.end == spans.token.end
            }
            Self::NumericUnitDependentNoun { unit, tail } => {
                (matches!(pattern.fine_pos, DataFinePos::Nnb | DataFinePos::Nr)
                    && spans.core == *unit)
                    || (pattern.fine_pos == DataFinePos::Nnb
                        && spans.core == *tail
                        && spans.consumed.end == spans.token.end)
            }
            Self::NumeralSequence { fallback } => {
                (pattern.fine_pos == DataFinePos::Nr
                    && evidence.numeric_spans.contains(&spans.core))
                    || fallback.accepts(support, spans, patterns, text, evidence)
            }
            Self::AdnominalDerivation { fallback } => {
                !(support.evidence == StructuralEvidence::RuntimeComponent
                    && pattern.fine_pos.is_nominal()
                    && evidence
                        .adnominal_derivation_suffix_starts
                        .binary_search(&spans.core.start)
                        .is_ok())
                    && fallback.accepts(support, spans, patterns, text, evidence)
            }
            Self::RuntimeCompatible { graph_nominal_host } => match support.evidence {
                StructuralEvidence::Whole => true,
                StructuralEvidence::SourceComponent => {
                    !component_is_shadowed_by_predicate(support.evidence, pattern, spans, evidence)
                }
                StructuralEvidence::RuntimeComponent => {
                    !component_is_shadowed_by_predicate(support.evidence, pattern, spans, evidence)
                        && runtime_position_is_supported(pattern, spans, text, evidence)
                        && runtime_nominal_component_is_supported(
                            pattern,
                            spans,
                            evidence,
                            graph_nominal_host.as_ref(),
                        )
                }
            },
        }
    }
}

pub(in crate::structure) fn select_structure(
    resource: &ComponentResource,
    context: BoundedTokenContext<'_>,
    evidence: &TokenEvidence,
) -> StructureSelection {
    if (context.previous == Some(context.current) || context.next == Some(context.current))
        && evidence.has_whole(DataFinePos::Mag)
    {
        return StructureSelection::Adverb;
    }
    if evidence.has_whole(DataFinePos::Mag)
        && evidence
            .units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos.is_nominal())
        && context
            .next
            .is_some_and(|next| complete_ha_predicate_path(resource, next))
    {
        return StructureSelection::Adverb;
    }
    let (next_starts_nominal, next_is_unambiguous_nominal, next_starts_determiner_nominal) =
        context.next.map_or((false, false, false), |next| {
            let mut exact_nominal = false;
            let mut exact_dependent_or_counter_nominal = false;
            let mut exact_competitor = false;
            let mut exact_lexical_competitor = false;
            resource.common_prefix_positions(next.as_bytes(), |length, positions| {
                if length != next.len() {
                    return;
                }
                let Some(pos) = positions.first().copied() else {
                    return;
                };
                exact_nominal |= pos.is_nominal_tag();
                exact_dependent_or_counter_nominal |=
                    matches!(pos, StructuralPos::NNB | StructuralPos::NNBC);
                exact_competitor |= !pos.is_nominal_tag();
                exact_lexical_competitor |=
                    !pos.is_nominal_tag() && !matches!(pos, StructuralPos::XSN | StructuralPos::XR);
            });
            let complete_nominal = complete_nominal_host(resource, next)
                || complete_nominal_particle_host(resource, next).is_some();
            let unambiguous_nominal = (exact_nominal || complete_nominal)
                && !exact_competitor
                && !complete_predicate_ending_path(resource, next);
            let starts_nominal = !nominal_particle_hosts(resource, next).is_empty()
                || (!exact_competitor && (exact_nominal || complete_nominal));
            let starts_determiner_nominal =
                starts_nominal || (exact_dependent_or_counter_nominal && !exact_lexical_competitor);
            (
                starts_nominal,
                unambiguous_nominal,
                starts_determiner_nominal,
            )
        });
    let particle_host = evidence.nominal_particle_hosts.last().cloned();
    if next_starts_determiner_nominal
        && context.current.chars().count() == 1
        && evidence.has_whole(DataFinePos::Mm)
    {
        return StructureSelection::AdjacentDeterminer;
    }
    if let Some((nominal, copula)) = copular_frame(resource, context) {
        return StructureSelection::CopularFrame { nominal, copula };
    }
    let previous_is_determiner = context.previous.is_some_and(|previous| {
        has_exact_fine_pos(resource, previous, |pos| pos == DataFinePos::Mm)
    });
    let has_whole_nominal = evidence
        .units
        .all()
        .iter()
        .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos.is_nominal());
    let has_exact_whole_nominal =
        has_exact_fine_pos(resource, context.current, DataFinePos::is_nominal);
    let current_modifies_next_nominal = next_starts_nominal && !evidence.adnominal_ends.is_empty();
    let connective_ji_before_nominal = next_is_unambiguous_nominal
        && context.current.ends_with('지')
        && (exact_analysis_ends_with_pos(resource, context.current, |pos| {
            pos == StructuralPos::EC
        }) || complete_predicate_connective_ji_path(resource, context.current));
    if previous_is_determiner && has_whole_nominal && !current_modifies_next_nominal {
        return StructureSelection::DeterminerWhole;
    }
    if evidence.has_whole(DataFinePos::Mag)
        && particle_host.is_none()
        && context.next.is_some_and(|next| {
            exact_analysis_starts_with_pos(resource, next, StructuralPos::is_predicate_tag)
        })
        && has_copular_adnominal_split(resource, context.current)
    {
        return StructureSelection::Whole;
    }
    if context.previous.is_some_and(|previous| {
        exact_analysis_ends_with_pos(resource, previous, |pos| pos == StructuralPos::ETM)
            || adnominal_suffix_is_supported(resource, previous)
    }) && has_exact_fine_pos(resource, context.current, |pos| pos == DataFinePos::Nnb)
    {
        return StructureSelection::DependentNoun;
    }
    if !evidence.has_numeral_sequence {
        if let (Some(unit), Some(tail)) = (
            evidence.numeric_spans.first(),
            evidence.numeric_dependent_tail.as_ref(),
        ) {
            return StructureSelection::NumericUnitDependentNoun {
                unit: unit.clone(),
                tail: tail.clone(),
            };
        }
        if let Some(unit) = evidence.numeric_spans.first() {
            return StructureSelection::NumericUnit { unit: unit.clone() };
        }
    }
    let fallback = if let Some(host) = particle_host {
        let allow_components = false;
        let allow_whole_nominal_source_components =
            host != (0..context.current.len()) && evidence.has_whole_nominal_source_components;
        StructureSelection::NominalSpan {
            selected: host,
            allow_components,
            allow_whole_nominal_source_components,
        }
    } else {
        StructureSelection::RuntimeCompatible {
            graph_nominal_host: evidence.complete_nominal_particle_host.clone(),
        }
    };
    let fallback = if next_starts_nominal && !evidence.adnominal_derivation_suffix_starts.is_empty()
    {
        StructureSelection::AdnominalDerivation {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    };
    let fallback = if evidence.has_numeral_sequence {
        StructureSelection::NumeralSequence {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    };
    if connective_ji_before_nominal && (has_whole_nominal || has_exact_whole_nominal) {
        StructureSelection::ConnectiveJiNominalFrame {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    }
}
