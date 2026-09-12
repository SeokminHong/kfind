use crate::structure::StructuralEvidence;
use crate::structure::evidence::{TokenEvidence, minimum_unit_path};
use crate::structure::graph::Unit;
use crate::structure::paths::predicate::PredicateComponent;
use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern};
use kfind_data::DataFinePos;
use std::ops::Range;
use std::sync::OnceLock;

const NIKL_ATTACHED_NOMINAL_SUFFIXES: &str =
    include_str!("../../../../../data/rules/nikl-attached-nominal-suffixes.tsv");

pub(in crate::structure) fn compound_predicate_component_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    matches!(pattern.continuation, MorphContinuation::Predicate { .. })
        && !evidence.has_whole_modifier()
        && spans.core.start > spans.token.start
        && spans.consumed.start == spans.core.start
        && spans.consumed.end == spans.token.end
        && evidence
            .compound_predicate_components
            .binary_search(&PredicateComponent {
                start: spans.core.start,
                end: spans.core.end,
                pos: pattern.fine_pos,
            })
            .is_ok()
}

pub(in crate::structure) fn component_is_shadowed_by_predicate(
    support: StructuralEvidence,
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    let modifier_inside_derived_nominal = pattern.fine_pos == DataFinePos::Mm
        && evidence
            .runtime_nominal_derivation_spans
            .iter()
            .any(|nominal| nominal.start == spans.core.start && nominal.end > spans.core.end);
    if modifier_inside_derived_nominal {
        return true;
    }

    let unsupported_runtime_component = support == StructuralEvidence::RuntimeComponent
        && (((pattern.fine_pos.is_nominal() && pattern.lexical_form.as_ref() == "못")
            || (matches!(pattern.fine_pos, DataFinePos::Nng | DataFinePos::Nnp)
                && pattern.lexical_form.chars().count() == 1))
            && evidence
                .runtime_nominal_derivation_spans
                .contains(&spans.core)
            || matches!(pattern.fine_pos, DataFinePos::Mag | DataFinePos::Maj));
    let source_backed_nonderivational_nominal = support == StructuralEvidence::SourceComponent
        && pattern.fine_pos.is_nominal()
        && pattern.lexical_form.as_ref() == "못";
    (unsupported_runtime_component || source_backed_nonderivational_nominal)
        && evidence
            .leading_predicate_spans
            .iter()
            .any(|predicate| predicate.start == spans.core.start && predicate.end > spans.core.end)
}

pub(in crate::structure) fn nominal_component_is_supported(
    allow_components: bool,
    support: StructuralEvidence,
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
    pos: DataFinePos,
    lexical_form: &str,
) -> bool {
    if allow_components {
        return true;
    }
    match support {
        StructuralEvidence::SourceComponent => {
            evidence.has_unambiguous_nominal_source_component(selected, core, pos)
                || nominal_source_component_is_on_preferred_path(core, selected, evidence)
        }
        StructuralEvidence::RuntimeComponent => {
            lexical_form.chars().count() > 1
                && nominal_component_is_on_preferred_path(core, selected, evidence)
        }
        StructuralEvidence::Whole => false,
    }
}

fn nominal_component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    component_is_on_preferred_path(core, selected, evidence, |unit| {
        unit.pos.is_nominal() && unit.span != *selected
    })
}

fn nominal_source_component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    component_is_on_preferred_path(core, selected, evidence, |unit| unit.pos.is_nominal())
}

pub(in crate::structure) fn modifier_led_nominal_component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    text: &str,
    evidence: &TokenEvidence,
) -> bool {
    if core.start == selected.start {
        return false;
    }
    let mut has_leading_modifier = false;
    for unit in evidence.units.all() {
        if unit.span == *selected && unit.evidence == StructuralEvidence::Whole {
            return false;
        }
        has_leading_modifier |= unit.pos == DataFinePos::Mm
            && unit.span.start == selected.start
            && matches!(
                unit.evidence,
                StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
            );
    }
    if !has_leading_modifier {
        return false;
    }
    let direct_modifier = selected.start..core.start;
    let single_syllable_direct_modifier = core.end == selected.end
        && evidence.units.all().iter().any(|unit| {
            unit.pos == DataFinePos::Mm
                && unit.span == direct_modifier
                && text
                    .get(unit.span.clone())
                    .is_some_and(|surface| surface.chars().count() == 1)
        });
    if single_syllable_direct_modifier
        && !evidence.units.all().iter().any(|unit| {
            unit.pos == DataFinePos::Nr
                && unit.span == direct_modifier
                && matches!(
                    unit.evidence,
                    StructuralEvidence::SourceComponent | StructuralEvidence::RuntimeComponent
                )
        })
    {
        return false;
    }
    component_is_on_preferred_path(core, selected, evidence, |unit| {
        (unit.pos == DataFinePos::Mm && unit.span.start == selected.start)
            || (matches!(
                unit.pos,
                DataFinePos::Nng | DataFinePos::Nnp | DataFinePos::Nnb
            ) && unit.span.start > selected.start)
    })
}

fn component_is_on_preferred_path(
    core: &Range<usize>,
    selected: &Range<usize>,
    evidence: &TokenEvidence,
    accepts: impl Copy + Fn(&Unit) -> bool,
) -> bool {
    evidence
        .units
        .contains_on_preferred_path(core, selected, accepts)
}

pub(in crate::structure) fn runtime_nominal_component_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
    graph_nominal_host: Option<&Range<usize>>,
) -> bool {
    let Some(host) = graph_nominal_host else {
        return true;
    };
    if !pattern.fine_pos.is_nominal()
        || spans.core == *host
        || spans.core.start < host.start
        || spans.core.end > host.end
    {
        return true;
    }
    if proper_noun_dependent_noun_frame(pattern, spans, host, evidence) {
        return true;
    }
    if attached_nominal_suffix_is_supported(pattern, spans, evidence) {
        return true;
    }
    if spans.core.start == host.start && pattern.lexical_form.chars().count() > 1 {
        return true;
    }
    pattern.lexical_form.chars().count() > 1
        && nominal_component_is_on_preferred_path(&spans.core, host, evidence)
}

pub(in crate::structure) fn proper_noun_dependent_noun_frame(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    host: &Range<usize>,
    evidence: &TokenEvidence,
) -> bool {
    pattern.fine_pos == DataFinePos::Nnb
        && evidence.has_complete_path
        && pattern.lexical_form.chars().count() == 1
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && spans.core.end == host.end
        && spans.consumed.end > spans.core.end
        && evidence.units.all().iter().any(|unit| {
            unit.pos == DataFinePos::Nnp
                && unit.span.start == host.start
                && unit.span.end == spans.core.start
                && unit.span.len() > spans.core.len()
        })
        && evidence.units.all().iter().any(|unit| {
            unit.pos.is_particle()
                && unit.span.start == spans.core.end
                && unit.span.end <= spans.consumed.end
        })
}

pub(in crate::structure) fn attached_nominal_suffix_is_supported(
    pattern: &QueryMorphPattern,
    spans: &CandidateSpans,
    evidence: &TokenEvidence,
) -> bool {
    let nominal_host = spans.token.start..spans.core.end;
    pattern.fine_pos == DataFinePos::Nng
        && pattern.lexical_form.chars().count() == 1
        && matches!(pattern.continuation, MorphContinuation::NominalParticles)
        && evidence.has_complete_path
        && spans.core.start > spans.token.start
        && spans.consumed.end == spans.token.end
        && spans.consumed.end > spans.core.end
        && !evidence.units.all().iter().any(|unit| {
            unit.span == nominal_host && unit.evidence != StructuralEvidence::SourceComponent
        })
        && is_nikl_attached_nominal_suffix(&pattern.lexical_form)
        && minimum_unit_path(
            &(spans.token.start..spans.core.start),
            evidence,
            DataFinePos::is_nominal,
        )
        .is_some()
        && minimum_unit_path(
            &(spans.core.end..spans.consumed.end),
            evidence,
            DataFinePos::is_particle,
        )
        .is_some()
}

fn is_nikl_attached_nominal_suffix(lexical_form: &str) -> bool {
    static SURFACES: OnceLock<Box<[&'static str]>> = OnceLock::new();
    let surfaces = SURFACES.get_or_init(|| {
        let mut surfaces = NIKL_ATTACHED_NOMINAL_SUFFIXES
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once('\t').map(|(surface, _)| surface))
            .filter(|surface| !surface.is_empty())
            .collect::<Vec<_>>();
        surfaces.sort_unstable();
        surfaces.dedup();
        surfaces.into_boxed_slice()
    });
    surfaces.binary_search(&lexical_form).is_ok()
}
