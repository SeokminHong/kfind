use crate::structure::StructuralEvidence;
use crate::structure::evidence::{TokenEvidence, minimum_unit_path};
use crate::structure::selection::components::modifier_led_nominal_component_is_on_preferred_path;
use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern};
use kfind_data::DataFinePos;
use std::ops::Range;

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
    let modifier_without_nominal_tail = pattern.fine_pos == DataFinePos::Mm
        && spans.core != spans.token
        && !modifier_has_complete_nominal_tail(&spans.core, &spans.token, evidence);
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
    let unlexicalized_internal_one_syllable_nominal =
        matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
            && pattern.lexical_form.chars().count() == 1
            && matches!(pattern.continuation, MorphContinuation::NominalParticles)
            && spans.core.start > spans.token.start
            && spans.consumed == spans.core
            && !evidence.has_whole_analysis(&spans.token);
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
        && !modifier_without_nominal_tail
        && !trailing_predicate_subspan
        && !internal_runtime_predicate
        && !modifier_before_predicate
        && !trailing_exact_subspan
        && !trailing_nominal_chain
        && !nominal_after_predicate
        && !unlexicalized_internal_one_syllable_nominal
        && !terminal_nominal_in_predicate_frame
}

fn modifier_has_complete_nominal_tail(
    core: &Range<usize>,
    token: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    if core.start != token.start || core.end >= token.end {
        return false;
    }
    evidence
        .units
        .all()
        .iter()
        .map(|unit| unit.span.end)
        .filter(|&host_end| host_end > core.end && host_end <= token.end)
        .any(|host_end| {
            minimum_unit_path(&(core.end..host_end), evidence, DataFinePos::is_nominal).is_some()
                && minimum_unit_path(&(host_end..token.end), evidence, DataFinePos::is_particle)
                    .is_some()
        })
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

fn whole_predicate_continuation(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    evidence.units.all().iter().any(|unit| {
        unit.span == (spans.core.start..spans.token.end)
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
