use std::ops::Range;

use kfind_data::DataFinePos;

use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern};

use super::{
    ConstraintSupport, StructuralEvidence, TokenEvidence, attached_auxiliary_is_supported,
    composed_nominal_subpath, is_predicate_nominalization, nominal_derivation_before_predicate,
    query_nominal_particle_path,
};

pub(super) fn collect_pattern_supports(
    evidence: &TokenEvidence,
    spans: &CandidateSpans,
    patterns: &[QueryMorphPattern],
    graph_nominal_host: Option<&Range<usize>>,
) -> Vec<ConstraintSupport> {
    let mut supports = Vec::new();
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        let support_start = supports.len();
        for unit in evidence.units.all() {
            if unit.span != spans.core || unit.pos != pattern.fine_pos {
                continue;
            }
            let allowed = match unit.evidence {
                StructuralEvidence::Whole => true,
                StructuralEvidence::SourceComponent => pattern.component_capability.allows_source(),
                StructuralEvidence::RuntimeComponent => {
                    pattern.component_capability.allows_runtime()
                }
            };
            if allowed {
                supports.push(ConstraintSupport {
                    pattern_index,
                    evidence: unit.evidence,
                });
            }
        }
        if supports.len() == support_start && is_predicate_nominalization(pattern) {
            for unit in evidence
                .units
                .all()
                .iter()
                .filter(|unit| unit.span == spans.anchor && unit.pos.is_nominal())
            {
                supports.push(ConstraintSupport {
                    pattern_index,
                    evidence: unit.evidence,
                });
            }
        }
        if supports.len() == support_start
            && pattern.component_capability.allows_runtime()
            && supports_runtime_pattern(pattern, spans, evidence, graph_nominal_host)
        {
            supports.push(ConstraintSupport {
                pattern_index,
                evidence: StructuralEvidence::RuntimeComponent,
            });
        }
    }
    supports
}

fn requires_aligned_component_evidence(pattern: &QueryMorphPattern) -> bool {
    matches!(
        pattern.fine_pos,
        DataFinePos::Np | DataFinePos::Mm | DataFinePos::Mag | DataFinePos::Maj
    )
}

fn supports_runtime_pattern(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
    graph_nominal_host: Option<&Range<usize>>,
) -> bool {
    query_nominal_particle_path(pattern, spans)
        || composed_nominal_subpath(pattern, spans, evidence)
        || nominal_derivation_before_predicate(pattern, spans, evidence)
        || ((!requires_aligned_component_evidence(pattern) || spans.core == spans.token)
            && evidence.runtime_spans.contains(&spans.core))
        || attached_auxiliary_is_supported(pattern, spans, evidence)
        || (pattern.fine_pos.is_nominal() && evidence.has_nominal_copula_host(&spans.core))
        || supports_complete_runtime_span(pattern, spans, evidence, graph_nominal_host)
        || (!evidence.has_complete_path
            && (spans.consumed == spans.token
                || matches!(pattern.continuation, MorphContinuation::Predicate { .. })))
}

fn supports_complete_runtime_span(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
    graph_nominal_host: Option<&Range<usize>>,
) -> bool {
    match pattern.continuation {
        MorphContinuation::NominalParticles => {
            spans.core == spans.token
                || (pattern.fine_pos.is_nominal()
                    && graph_nominal_host == Some(&spans.core)
                    && spans.consumed == spans.token)
        }
        MorphContinuation::Predicate { .. } => {
            spans.consumed == spans.token
                && (spans.core.start == spans.token.start || evidence.has_whole(pattern.fine_pos))
        }
        _ => false,
    }
}
