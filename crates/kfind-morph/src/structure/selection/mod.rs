use crate::structure::evidence::TokenEvidence;
use crate::structure::lexical::{
    adnominal_suffix_is_supported, complete_ha_predicate_path, complete_nominal_host,
    complete_nominal_particle_host, complete_predicate_adnominal_path,
    complete_predicate_connective_ji_path, complete_predicate_ending_path, complete_suffix,
    copular_frame, exact_analysis_ends_with_pos, exact_analysis_starts_with_pos,
    has_adnominal_result_complement, has_auxiliary_adnominal_split, has_complete_object_phrase,
    has_copular_adnominal_split, has_exact_fine_pos, has_exact_sequence,
    has_unambiguous_proper_name, has_verb_nominalization, nominal_particle_hosts,
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
    AdjacentDeterminer {
        excludes_pronoun: bool,
        excludes_predicate: bool,
        preserves_proper_name: bool,
        preserves_proper_name_tail: bool,
    },
    NominalWhole,
    BeforeHonorificSsi {
        preserves_anonymous_pronoun: bool,
    },
    ProperNameOrAdnominal {
        only_adnominal: bool,
    },
    VerbAdnominal,
    ConnectiveJiNominalFrame {
        fallback: Box<StructureSelection>,
    },
    SourceAlignedConnective {
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
            Self::ConnectiveJiNominalFrame { fallback }
            | Self::SourceAlignedConnective { fallback } => fallback.graph_nominal_host(),
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
        if let Self::SourceAlignedConnective { fallback } = self {
            return !(pattern.fine_pos.is_predicate()
                && support.evidence == StructuralEvidence::RuntimeComponent)
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
        if matches!(pattern.fine_pos, DataFinePos::Va | DataFinePos::Vcn)
            && support.evidence == StructuralEvidence::RuntimeComponent
            && spans.core.start > spans.token.start
            && evidence
                .leading_predicate_spans
                .iter()
                .any(|stem| stem.end >= spans.core.end)
        {
            return false;
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
            Self::AdjacentDeterminer {
                excludes_pronoun,
                excludes_predicate,
                preserves_proper_name,
                preserves_proper_name_tail,
            } => {
                (support.evidence == StructuralEvidence::Whole
                    && (pattern.fine_pos == DataFinePos::Mm
                        || (*preserves_proper_name && pattern.fine_pos == DataFinePos::Nnp)
                        || (*preserves_proper_name_tail && pattern.fine_pos == DataFinePos::Nng)))
                    || (!matches!(
                        pattern.fine_pos,
                        DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
                    ) && (!excludes_pronoun || pattern.fine_pos != DataFinePos::Np)
                        && (!excludes_predicate || !pattern.fine_pos.is_predicate()))
            }
            Self::NominalWhole => {
                support.evidence == StructuralEvidence::Whole && pattern.fine_pos.is_nominal()
            }
            Self::BeforeHonorificSsi {
                preserves_anonymous_pronoun,
            } => {
                support.evidence == StructuralEvidence::Whole
                    && (pattern.fine_pos == DataFinePos::Nnp
                        || pattern.fine_pos
                            == if *preserves_anonymous_pronoun {
                                DataFinePos::Np
                            } else {
                                DataFinePos::Mm
                            })
            }
            Self::ProperNameOrAdnominal { only_adnominal } => {
                support.evidence == StructuralEvidence::Whole
                    && (pattern.fine_pos == DataFinePos::Vv
                        || (!only_adnominal && pattern.fine_pos == DataFinePos::Nnp))
            }
            Self::VerbAdnominal => !matches!(pattern.fine_pos, DataFinePos::Va | DataFinePos::Vcn),
            Self::ConnectiveJiNominalFrame { .. } | Self::SourceAlignedConnective { .. } => {
                unreachable!("handled before shared rules")
            }
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
                            || (selected.end == spans.core.start
                                && matches!(
                                    pattern.fine_pos,
                                    DataFinePos::Nng | DataFinePos::Nnp
                                )
                                && matches!(
                                    pattern.continuation,
                                    MorphContinuation::NominalParticles
                                )
                                && spans.consumed.end == spans.token.end
                                && text[spans.core.clone()].chars().nth(1).is_some()
                                && text[selected.clone()].chars().nth(1).is_some()
                                && !evidence.has_whole_analysis(&spans.token)
                                && evidence.units.starting_at(spans.core.start).iter().any(
                                    |unit| unit.span == spans.core && unit.pos == pattern.fine_pos,
                                ))
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
                                spans,
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
                            || (spans.anchor.end < selected.end
                                && evidence.has_nominal_source_component(&spans.anchor))
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
                    || (matches!(pattern.fine_pos, DataFinePos::Np | DataFinePos::Nr)
                        && spans.core == *selected
                        && spans.consumed.end == spans.token.end)
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
        && context.next.is_some_and(|next| {
            complete_ha_predicate_path(resource, next)
                && !has_exact_fine_pos(resource, next, DataFinePos::is_nominal)
        })
    {
        return StructureSelection::Adverb;
    }
    if let Some(tail) = context.next.and_then(|next| next.strip_prefix('씨'))
        && context.current.chars().count() == 1
        && evidence.has_whole(DataFinePos::Nnp)
        && has_exact_sequence(resource, "씨", &[StructuralPos::NNB])
        && (tail.is_empty() || complete_suffix(resource, tail, StructuralPos::is_particle))
    {
        return StructureSelection::BeforeHonorificSsi {
            preserves_anonymous_pronoun: context.current == "모"
                && context.previous.is_some_and(|previous| {
                    has_exact_fine_pos(resource, previous, |pos| pos == DataFinePos::Nnp)
                }),
        };
    }
    let demonstrative = matches!(context.current, "이" | "그" | "저");
    let (
        next_starts_nominal,
        next_is_unambiguous_nominal,
        next_starts_determiner_nominal,
        next_is_counter,
    ) = context.next.map_or((false, false, false, false), |next| {
        let mut exact_nominal = false;
        let mut exact_dependent_or_counter_nominal = false;
        let mut exact_counter = false;
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
            exact_counter |=
                pos == StructuralPos::NNBC && positions[1..].iter().all(|pos| pos.is_particle());
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
        let coordinate_verb = !demonstrative
            && !exact_nominal
            && next.ends_with('고')
            && evidence.has_whole(DataFinePos::Nng)
            && !has_exact_sequence(
                resource,
                context.current,
                &[StructuralPos::VV, StructuralPos::EC],
            )
            && next.strip_suffix('고').is_some_and(|stem| {
                has_exact_sequence(resource, stem, &[StructuralPos::VV])
                    && has_exact_sequence(resource, "고", &[StructuralPos::EC])
            });
        let starts_determiner_nominal = (starts_nominal && !coordinate_verb)
            || (exact_dependent_or_counter_nominal && (demonstrative || !exact_lexical_competitor));
        (
            starts_nominal,
            unambiguous_nominal,
            starts_determiner_nominal,
            exact_counter,
        )
    });
    let particle_host = evidence.nominal_particle_hosts.last().cloned();
    let excludes_predicate = next_is_counter && evidence.adnominal_ends.is_empty();
    if (next_starts_determiner_nominal || excludes_predicate)
        && context.current.chars().count() == 1
        && evidence.has_whole(DataFinePos::Mm)
    {
        return StructureSelection::AdjacentDeterminer {
            excludes_pronoun: demonstrative && next_starts_determiner_nominal,
            excludes_predicate,
            preserves_proper_name_tail: !demonstrative
                && !next_is_counter
                && evidence.has_whole(DataFinePos::Nng)
                && context
                    .next
                    .is_some_and(|next| has_unambiguous_proper_name(resource, next))
                && context.previous.is_some_and(|previous| {
                    has_exact_fine_pos(resource, previous, |pos| pos == DataFinePos::Nnp)
                        && has_exact_sequence(
                            resource,
                            &format!("{previous}{}", context.current),
                            &[StructuralPos::NNP],
                        )
                }),
            preserves_proper_name: !demonstrative
                && !next_is_counter
                && !matches!(
                    context.current,
                    "한" | "두"
                        | "세"
                        | "네"
                        | "석"
                        | "서"
                        | "넉"
                        | "너"
                        | "열"
                        | "쉰"
                        | "몇"
                        | "수"
                )
                && evidence.has_whole(DataFinePos::Nnp)
                && context.next.is_some_and(|next| {
                    has_exact_fine_pos(resource, next, |pos| pos == DataFinePos::Nng)
                })
                && context.previous.is_some_and(|previous| {
                    complete_nominal_host(resource, previous)
                        && complete_nominal_particle_host(resource, previous).is_none()
                }),
        };
    }
    if let Some((nominal, copula)) = copular_frame(resource, context) {
        return StructureSelection::CopularFrame { nominal, copula };
    }
    let has_whole_nominal = evidence
        .units
        .all()
        .iter()
        .any(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos.is_nominal());
    let has_exact_whole_nominal =
        has_exact_fine_pos(resource, context.current, DataFinePos::is_nominal);
    if context.current.ends_with('건')
        && evidence.has_whole(DataFinePos::Nng)
        && (next_starts_nominal
            || context.previous.is_some_and(|previous| {
                exact_analysis_ends_with_pos(resource, previous, |pos| pos == StructuralPos::ETM)
            }))
        && !context
            .next
            .and_then(|next| next.strip_suffix('건'))
            .is_some_and(|stem| has_exact_fine_pos(resource, stem, DataFinePos::is_predicate))
        && !evidence.has_whole_predicate()
    {
        return StructureSelection::NominalWhole;
    }
    if has_whole_nominal
        && context.previous.is_some_and(|previous| {
            has_exact_fine_pos(resource, previous, |pos| pos == DataFinePos::Mm)
        })
        && exact_analysis_ends_with_pos(resource, context.current, |pos| pos == StructuralPos::NNBC)
        && !context.next.is_some_and(|next| {
            let (anchor, state) = if next.starts_with("있었") {
                ("있었", crate::ContinuationState::Past)
            } else {
                ("있", crate::ContinuationState::Eu)
            };
            next.strip_prefix(anchor).is_some_and(|tail| {
                (tail == "다"
                    || crate::verify_predicate_continuation(
                        state,
                        crate::PredicatePos::AuxiliaryVerb,
                        anchor,
                        tail,
                    )
                    .is_some_and(|matched| matched.consumed_bytes == tail.len()))
                    && has_exact_sequence(
                        resource,
                        context.current,
                        &[StructuralPos::VV, StructuralPos::EC],
                    )
                    && has_exact_sequence(resource, "있", &[StructuralPos::VX])
            })
        })
    {
        return StructureSelection::NominalWhole;
    }
    if evidence.has_whole(DataFinePos::Nnp)
        && !evidence.has_whole(DataFinePos::Nng)
        && evidence.has_whole_predicate()
        && context
            .next
            .is_some_and(|next| has_unambiguous_proper_name(resource, next))
    {
        if context.previous.is_some_and(|previous| {
            has_complete_object_phrase(resource, previous)
                || previous.strip_suffix("까지").is_some_and(|host| {
                    complete_nominal_host(resource, host)
                        && has_exact_sequence(resource, "까지", &[StructuralPos::JX])
                })
        }) {
            let mut adnominal = false;
            resource.common_prefix_positions(context.current.as_bytes(), |length, positions| {
                adnominal |= length == context.current.len()
                    && matches!(
                        positions,
                        [StructuralPos::VV, prefinals @ .., StructuralPos::ETM]
                            if prefinals.iter().all(|pos| *pos == StructuralPos::EP)
                    );
            });
            if adnominal {
                return StructureSelection::ProperNameOrAdnominal {
                    only_adnominal: false,
                };
            }
        }
        return StructureSelection::NominalWhole;
    }
    if evidence.adnominal_ends.contains(&context.current.len())
        && evidence.has_whole(DataFinePos::Vv)
        && evidence.has_whole(DataFinePos::Va)
        && !context
            .next
            .is_some_and(|next| has_adnominal_result_complement(resource, next))
        && context
            .previous
            .is_some_and(|previous| has_complete_object_phrase(resource, previous))
    {
        return StructureSelection::VerbAdnominal;
    }
    let previous_is_determiner = context.previous.is_some_and(|previous| {
        has_exact_fine_pos(resource, previous, |pos| pos == DataFinePos::Mm)
            && !has_exact_fine_pos(resource, previous, |pos| {
                matches!(pos, DataFinePos::Mag | DataFinePos::Maj)
            })
    });
    let current_modifies_next_nominal = next_starts_nominal && !evidence.adnominal_ends.is_empty();
    if has_whole_nominal
        && evidence.adnominal_ends.contains(&context.current.len())
        && context.next.is_some_and(|next| {
            (!evidence.has_whole_predicate()
                && has_exact_fine_pos(resource, next, |pos| {
                    matches!(pos, DataFinePos::Mag | DataFinePos::Maj)
                })
                && has_auxiliary_adnominal_split(resource, context.current))
                || (evidence.has_whole(DataFinePos::Nng)
                    && context
                        .previous
                        .is_some_and(|previous| has_unambiguous_proper_name(resource, previous))
                    && has_verb_nominalization(resource, next))
        })
    {
        return StructureSelection::NominalWhole;
    }
    if has_whole_nominal
        && evidence.has_whole(DataFinePos::Mag)
        && !next_starts_nominal
        && exact_analysis_ends_with_pos(resource, context.current, |pos| pos == StructuralPos::ETM)
        && !exact_analysis_ends_with_pos(resource, context.current, |pos| {
            matches!(pos, StructuralPos::EC | StructuralPos::EF)
        })
        && context
            .next
            .is_some_and(|next| complete_predicate_ending_path(resource, next))
    {
        return StructureSelection::Adverb;
    }
    let connective_ji_before_nominal = (has_whole_nominal || has_exact_whole_nominal)
        && (next_is_unambiguous_nominal
            || (context.current.ends_with('치')
                && context
                    .next
                    .is_some_and(|next| has_complete_object_phrase(resource, next))))
        && ((context.current.ends_with('지')
            && exact_analysis_ends_with_pos(resource, context.current, |pos| {
                pos == StructuralPos::EC
            }))
            || complete_predicate_connective_ji_path(resource, context.current));
    if previous_is_determiner
        && has_whole_nominal
        && !current_modifies_next_nominal
        && !(evidence.has_whole(DataFinePos::Mag)
            && context.previous.is_some_and(|previous| {
                has_exact_fine_pos(resource, previous, |pos| pos == DataFinePos::Nr)
            }))
    {
        return StructureSelection::NominalWhole;
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
    if let Some(previous) = context.previous
        && (exact_analysis_ends_with_pos(resource, previous, |pos| pos == StructuralPos::ETM)
            || adnominal_suffix_is_supported(resource, previous))
        && has_exact_fine_pos(resource, context.current, |pos| pos == DataFinePos::Nnb)
        && (!evidence.has_whole(DataFinePos::Mag)
            || (complete_predicate_adnominal_path(resource, previous)
                && !has_exact_fine_pos(resource, previous, DataFinePos::is_nominal)))
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
    if connective_ji_before_nominal {
        StructureSelection::ConnectiveJiNominalFrame {
            fallback: Box::new(fallback),
        }
    } else if context.current.ends_with('려')
        && evidence.has_whole(DataFinePos::Vv)
        && context
            .next
            .and_then(|next| next.strip_suffix('게'))
            .is_some_and(|stem| {
                !stem.is_empty()
                    && has_exact_sequence(resource, stem, &[StructuralPos::VV])
                    && has_exact_sequence(resource, "게", &[StructuralPos::EC])
            })
        && has_exact_sequence(
            resource,
            context.current,
            &[StructuralPos::VV, StructuralPos::EC],
        )
    {
        StructureSelection::SourceAlignedConnective {
            fallback: Box::new(fallback),
        }
    } else {
        fallback
    }
}
