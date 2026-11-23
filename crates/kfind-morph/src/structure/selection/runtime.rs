use crate::structure::StructuralEvidence;
use crate::structure::evidence::{TokenEvidence, minimum_unit_path};
use crate::structure::selection::components::modifier_led_nominal_component_is_on_preferred_path;
use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern};
use kfind_data::DataFinePos;
use std::ops::Range;

const COMPOUNDS_WITH_INITIAL_HANJA: &[&str] =
    include!("../../../../../data/rules/stdict-initial-hanja-compounds.rs");

pub(in crate::structure) fn runtime_position_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    text: &str,
    evidence: &TokenEvidence,
) -> bool {
    let starts_token = spans.core.start == spans.token.start;
    let leading_only = matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm | DataFinePos::Mag
    );
    let unsupported_modifier = pattern.fine_pos == DataFinePos::Mm
        && spans.core != spans.token
        && !has_preferred_modifier_tail(&spans.core, &spans.token, evidence);
    let modifier_inside_longer_analysis = pattern.fine_pos == DataFinePos::Mm
        && starts_token
        && spans.core.end < spans.token.end
        && (evidence
            .leading_predicate_spans
            .iter()
            .any(|stem| stem.end > spans.core.end)
            || evidence.units.all().iter().any(|unit| {
                unit.pos == DataFinePos::Nnp
                    && unit.span.start == spans.token.start
                    && unit.span.end > spans.core.end
                    && minimum_unit_path(
                        &(unit.span.end..spans.token.end),
                        evidence,
                        DataFinePos::is_nominal,
                    )
                    .is_some()
            }));
    let pronoun_inside_predicate = pattern.fine_pos == DataFinePos::Np
        && starts_token
        && spans.consumed == spans.core
        && spans.core.end < spans.token.end
        && (evidence
            .leading_predicate_spans
            .iter()
            .any(|predicate| predicate.end > spans.core.end)
            || evidence
                .runtime_nominal_derivation_spans
                .iter()
                .any(|root| {
                    root.end > spans.core.end
                        && evidence
                            .derivational_suffix_starts
                            .binary_search(&root.end)
                            .is_ok()
                }));
    let predicate = matches!(pattern.continuation, MorphContinuation::Predicate { .. });
    let terminal_predicate_component = matches!(
        pattern.continuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            ..
        }
    ) && (spans.anchor.end > spans.core.end
        || spans.core.len() > pattern.lexical_form.len());
    let whole_predicate_continuation = whole_predicate_continuation(pattern, spans, evidence);
    let copula_nominal_host = copula_has_complete_nominal_host(pattern, spans, evidence);
    let attached_auxiliary = attached_auxiliary_is_supported(pattern, spans, evidence);
    let trailing_predicate_subspan = predicate
        && spans.consumed.end != spans.token.end
        && !terminal_predicate_component
        && !whole_predicate_continuation;
    let internal_runtime_predicate = predicate
        && (pattern.fine_pos != DataFinePos::Vcp || evidence.has_whole(DataFinePos::Mag))
        && spans.core.start != spans.token.start
        && spans.consumed == spans.core
        && !attached_auxiliary;
    let competes_with_nominal_copula = predicate
        && pattern.fine_pos != DataFinePos::Vcp
        && !attached_auxiliary
        && !starts_token
        && evidence.has_nominal_copula_host(&(spans.token.start..spans.core.start));
    let modifier_before_predicate = predicate
        && !copula_nominal_host
        && !attached_auxiliary
        && spans.core.start != spans.token.start
        && evidence.units.all().iter().any(|unit| {
            unit.span.end == spans.core.start
                && matches!(unit.pos, DataFinePos::Mag | DataFinePos::Maj)
        });
    let exact_component_prefix = (matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Nr | DataFinePos::Mm
    ) || (pattern.fine_pos == DataFinePos::Mag
        && evidence.has_complete_path)
        || (matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
            && pattern.lexical_form.chars().count() > 1))
        && starts_token
        && !evidence
            .units
            .all()
            .iter()
            .any(|unit| unit.evidence == StructuralEvidence::Whole);
    let trailing_exact_subspan = matches!(pattern.continuation, MorphContinuation::Exact)
        && spans.consumed.end != spans.token.end
        && !exact_component_prefix;
    let multi_syllable_nominal_component = matches!(
        pattern.fine_pos,
        DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
    ) && pattern.lexical_form.chars().count() > 1;
    let trailing_nominal_chain =
        matches!(pattern.continuation, MorphContinuation::NominalParticles)
            && spans.consumed.end != spans.token.end
            && !evidence.has_nominal_copula_host(&spans.core)
            && !exact_component_prefix
            && !multi_syllable_nominal_component;
    let nominal_after_predicate = pattern.fine_pos.is_nominal()
        && pattern.lexical_form.chars().count() == 1
        && spans.consumed.end > spans.core.end
        && evidence.has_predicate_ending_at(spans.core.start);
    let unsupported_internal_nominal = pattern.fine_pos.is_nominal()
        && pattern.lexical_form.chars().count() == 1
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && !starts_token
        && spans.consumed == spans.core
        && {
            let mut whole = evidence
                .units
                .starting_at(spans.token.start)
                .iter()
                .filter(|unit| {
                    unit.span == spans.token && unit.evidence == StructuralEvidence::Whole
                });
            match whole.next() {
                Some(unit) => {
                    unit.pos.is_modifier()
                        && (evidence.has_whole(DataFinePos::Mag)
                            || evidence.has_whole(DataFinePos::Maj))
                        && whole.all(|unit| unit.pos.is_modifier())
                }
                None => matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp),
            }
        };
    let repeated_adverb_tail = pattern.fine_pos.is_nominal()
        && !starts_token
        && spans.consumed == spans.core
        && spans.core.end == spans.token.end
        && text[spans.token.start..spans.core.start] == text[spans.core.clone()]
        && !evidence.has_numeral_sequence
        && evidence.has_whole(DataFinePos::Mag);
    let glued_dependent_noun =
        pattern.fine_pos == DataFinePos::Nnb && evidence.has_adnominal_ending_at(spans.core.start);
    let terminal_nominal_in_predicate_frame = pattern.fine_pos.is_nominal()
        && pattern.lexical_form.chars().count() == 1
        && spans.core.start > spans.token.start
        && spans.core.end == spans.token.end
        && evidence.units.all().iter().any(|unit| {
            unit.pos.is_predicate()
                && ((unit.span.start == spans.token.start && unit.span.end <= spans.core.start)
                    || (unit.span.start < spans.core.start && unit.span.end >= spans.core.end))
        })
        && !glued_dependent_noun
        && !modifier_led_nominal_component_is_on_preferred_path(
            &spans.core,
            &spans.token,
            text,
            evidence,
        );

    (!leading_only || starts_token)
        && !unsupported_modifier
        && !modifier_inside_longer_analysis
        && !pronoun_inside_predicate
        && !trailing_predicate_subspan
        && !internal_runtime_predicate
        && !competes_with_nominal_copula
        && !modifier_before_predicate
        && !trailing_exact_subspan
        && !trailing_nominal_chain
        && !nominal_after_predicate
        && !unsupported_internal_nominal
        && !repeated_adverb_tail
        && !terminal_nominal_in_predicate_frame
}

pub(super) fn has_complete_nominal_tail(
    core: &Range<usize>,
    token: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    if core.start != token.start || core.end >= token.end {
        return false;
    }
    let span = core.end..token.end;
    let tails = nominal_tail_costs::<false>(&span, evidence);
    first_nominal_tail_cost::<false>(&span, &tails, evidence).is_some()
}

fn has_preferred_modifier_tail(
    core: &Range<usize>,
    token: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    if core.start != token.start || core.end >= token.end {
        return false;
    }
    let span = core.end..token.end;
    let tails = nominal_tail_costs::<true>(&span, evidence);
    let Some(modifier_tail) = first_nominal_tail_cost::<true>(&span, &tails, evidence) else {
        return false;
    };
    !evidence.units.starting_at(token.start).iter().any(|unit| {
        unit.aligned_pos
            && unit.pos.is_nominal()
            && unit.span.end > core.end
            && unit.span.end <= token.end
            && tails[unit.span.end - core.end]
                .is_some_and(|nominal_tail| nominal_tail < modifier_tail)
    })
}

fn first_nominal_tail_cost<const ALIGNED: bool>(
    span: &Range<usize>,
    tails: &[Option<usize>],
    evidence: &TokenEvidence,
) -> Option<usize> {
    evidence
        .units
        .starting_at(span.start)
        .iter()
        .filter(|unit| {
            (!ALIGNED || unit.aligned_pos) && unit.pos.is_nominal() && unit.span.end <= span.end
        })
        .filter_map(|unit| tails[unit.span.end - span.start].map(|count| count + 1))
        .min()
}

fn nominal_tail_costs<const ALIGNED: bool>(
    span: &Range<usize>,
    evidence: &TokenEvidence,
) -> Vec<Option<usize>> {
    // A zero-cost suffix contains only particles, so another particle can precede it.
    let mut tails = vec![None; span.len() + 1];
    tails[span.len()] = Some(0);
    for offset in (1..span.len()).rev() {
        tails[offset] = evidence
            .units
            .starting_at(span.start + offset)
            .iter()
            .filter(|unit| (!ALIGNED || unit.aligned_pos) && unit.span.end <= span.end)
            .filter_map(|unit| {
                let count = tails[unit.span.end - span.start]?;
                if unit.pos.is_nominal() {
                    Some(count + 1)
                } else if unit.pos.is_particle() && count == 0 {
                    Some(0)
                } else {
                    None
                }
            })
            .min();
    }
    tails
}

pub(in crate::structure) fn attached_auxiliary_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos == DataFinePos::Vx
        && !evidence.has_whole(DataFinePos::Mag)
        && !evidence.has_whole(DataFinePos::Maj)
        && evidence.attached_auxiliary_spans.iter().any(|frame| {
            frame == &spans.core
                || (pattern.lexical_form.as_ref() == "지"
                    && frame == &spans.token
                    && frame.start < spans.core.start
                    && spans.core.end <= frame.end)
        })
}

pub(super) fn whole_predicate_continuation(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    evidence
        .units
        .starting_at(spans.core.start)
        .iter()
        .any(|unit| {
            unit.span.end == spans.token.end
                && unit.pos == pattern.fine_pos
                && unit.pos.is_predicate()
        })
}

fn copula_has_complete_nominal_host(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos == DataFinePos::Vcp
        && evidence.has_complete_path
        && !evidence.has_whole(DataFinePos::Mag)
        && spans.core.start > spans.token.start
        && evidence
            .units
            .all()
            .iter()
            .any(|unit| unit.span == (spans.token.start..spans.core.start) && unit.pos.is_nominal())
}

pub(in crate::structure) fn is_predicate_nominalization(pattern: &QueryMorphPattern) -> bool {
    matches!(
        pattern.continuation,
        MorphContinuation::Predicate {
            nominal_particles: true,
            ..
        }
    )
}

pub(in crate::structure) fn nominalization_conflicts_with_nominal_host(
    spans: &CandidateSpans,
    text: &str,
    evidence: &TokenEvidence,
    support: StructuralEvidence,
) -> bool {
    let whole_host = evidence
        .units
        .starting_at(spans.token.start)
        .iter()
        .find(|unit| unit.evidence == StructuralEvidence::Whole && unit.pos.is_nominal())
        .map(|unit| &unit.span);
    let mut hosts = whole_host.into_iter().chain(
        evidence
            .nominal_particle_hosts
            .iter()
            .filter(|host| spans.consumed.end < host.end),
    );
    let Some(first) = hosts.next() else {
        return false;
    };
    (support == StructuralEvidence::RuntimeComponent
        && !evidence.has_nominal_source_component(&spans.anchor))
        || (spans.anchor.start == spans.token.start
            && spans.anchor.len() == '가'.len_utf8()
            && std::iter::once(first).chain(hosts).any(|host| {
                COMPOUNDS_WITH_INITIAL_HANJA
                    .binary_search(&&text[host.clone()])
                    .is_ok()
            }))
}

pub(in crate::structure) fn query_nominal_particle_path(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
) -> bool {
    pattern.fine_pos.is_nominal()
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && spans.core.start == spans.token.start
        && spans.core.end < spans.consumed.end
        && spans.consumed == spans.token
}

pub(in crate::structure) fn nominal_derivation_before_predicate(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos.is_nominal()
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && spans.core.start == spans.token.start
        && evidence
            .nominal_derivation_predicate_prefixes
            .binary_search_by_key(&(spans.core.start, spans.core.end), |prefix| {
                (prefix.start, prefix.end)
            })
            .is_ok()
}

pub(in crate::structure) fn composed_nominal_subpath(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    if !pattern.fine_pos.is_nominal()
        || !matches!(pattern.continuation, MorphContinuation::NominalParticles)
        || evidence
            .derivational_suffix_starts
            .binary_search(&spans.core.end)
            .is_ok()
        || minimum_unit_path(&spans.core, evidence, DataFinePos::is_nominal)
            .is_none_or(|units| units < 2)
    {
        return false;
    }
    evidence
        .units
        .all()
        .iter()
        .map(|unit| unit.span.end)
        .filter(|&host_end| host_end >= spans.core.end && host_end <= spans.token.end)
        .any(|host_end| {
            minimum_unit_path(
                &(spans.token.start..host_end),
                evidence,
                DataFinePos::is_nominal,
            )
            .is_some()
                && minimum_unit_path(
                    &(host_end..spans.token.end),
                    evidence,
                    DataFinePos::is_particle,
                )
                .is_some()
        })
}
