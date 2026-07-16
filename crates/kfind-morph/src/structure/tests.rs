use std::io::Cursor;

use kfind_data::{
    MecabSourceMorphologyEntry, decode_component_resource, encode_component_resource,
    parse_mecab_connection_matrix,
};

use super::*;
use crate::{CandidateTokenRelation, ComponentCapability, MorphContinuation};

#[test]
fn ordinary_adverb_context_rejects_a_runtime_nominal_prefix() {
    let resolver = resolver();
    let context = BoundedTokenContext {
        previous: None,
        current: "매일",
        next: Some("보고"),
    };

    let noun = resolver.resolve_candidate(
        context,
        spans(0.."매".len(), 0.."매일".len()),
        &[component_pattern(DataFinePos::Nng, "매")],
        128,
    );
    let adverb = resolver.resolve_candidate(
        context,
        spans(0.."매일".len(), 0.."매일".len()),
        &[whole_pattern(DataFinePos::Mag, "매일")],
        128,
    );

    assert_eq!(noun.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(adverb.outcome, ConstraintOutcome::Supported);
}

#[test]
fn copular_arrangement_selects_the_nominal_prefix_over_the_adverb() {
    let resolver = resolver();
    let context = BoundedTokenContext {
        previous: Some("아니라"),
        current: "매일",
        next: Some("수도"),
    };

    let noun = resolver.resolve_candidate(
        context,
        spans(0.."매".len(), 0.."매일".len()),
        &[component_pattern(DataFinePos::Nng, "매")],
        128,
    );
    let adverb = resolver.resolve_candidate(
        context,
        spans(0.."매일".len(), 0.."매일".len()),
        &[whole_pattern(DataFinePos::Mag, "매일")],
        128,
    );

    assert_eq!(noun.outcome, ConstraintOutcome::Supported);
    assert_eq!(adverb.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn semantic_alternatives_with_one_structure_do_not_become_ambiguous() {
    let resolver = resolver();
    let patterns = [
        component_pattern(DataFinePos::Vv, "걷"),
        component_pattern(DataFinePos::Vv, "걸"),
    ];
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("걸었고"),
        spans(0.."걸".len(), 0.."걸었고".len()),
        &patterns,
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert_eq!(decision.supported.len(), 2);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn whole_inflected_analysis_supports_a_predicate_stem_program() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Va, "곱").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("곱아"),
        CandidateSpans {
            core: 0.."곱".len(),
            anchor: 0.."곱아".len(),
            consumed: 0.."곱아".len(),
            token: 0.."곱아".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn longest_nominal_particle_host_hides_an_inner_component() {
    let resolver = resolver();
    let inner = resolver.resolve_candidate(
        BoundedTokenContext::current("매일을"),
        spans(0.."매".len(), 0.."매일을".len()),
        &[component_pattern(DataFinePos::Nng, "매")],
        128,
    );
    let host = resolver.resolve_candidate(
        BoundedTokenContext::current("매일을"),
        spans(0.."매일".len(), 0.."매일을".len()),
        &[component_pattern(DataFinePos::Nng, "매일")],
        128,
    );

    assert_eq!(inner.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(host.outcome, ConstraintOutcome::Supported);
}

fn resolver() -> ConstraintResolver {
    let entries = [
        atomic("매", "NNG"),
        atomic("매일", "MAG"),
        atomic("매일", "NNG"),
        atomic("을", "JKO"),
        atomic("보고", "VV+EC"),
        atomic("아니라", "VCN+EC"),
        atomic("수도", "NNB+JX"),
        expression("일", "VCP+ETM", "이/VCP/*+ᆯ/ETM/*"),
        expression("걸었고", "VV+EP+EC", "걸/VV/*+었/EP/*+고/EC/*"),
        atomic("곱아", "VA"),
    ];
    let matrix =
        parse_mecab_connection_matrix("matrix", Cursor::new("1 1\n0 0 0\n")).expect("valid matrix");
    let bytes = encode_component_resource([9; 32], &entries, &matrix, b"char", b"unk")
        .expect("valid resource");
    let resource =
        decode_component_resource("fixture", bytes, &[9; 32]).expect("decodable resource");
    ConstraintResolver::new(Arc::new(resource))
}

fn atomic(surface: &str, pos: &str) -> MecabSourceMorphologyEntry {
    expression(surface, pos, "*")
}

fn expression(surface: &str, pos: &str, expression: &str) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 0,
        right_id: 0,
        word_cost: 0,
        analysis_type: "*".to_owned(),
        start_pos: pos.split('+').next().unwrap_or(pos).to_owned(),
        end_pos: pos.split('+').next_back().unwrap_or(pos).to_owned(),
        expression: expression.to_owned(),
    }
}

fn whole_pattern(pos: DataFinePos, lexical_form: &str) -> QueryMorphPattern {
    QueryMorphPattern::new(pos, lexical_form)
}

fn component_pattern(pos: DataFinePos, lexical_form: &str) -> QueryMorphPattern {
    QueryMorphPattern::new(pos, lexical_form).with_candidate_contract(
        CandidateTokenRelation::Whole,
        MorphContinuation::Exact,
        ComponentCapability::SourceAndRuntime,
    )
}

fn spans(core: Range<usize>, consumed: Range<usize>) -> CandidateSpans {
    let token_end = consumed.end;
    CandidateSpans {
        anchor: core.clone(),
        core,
        consumed,
        token: 0..token_end,
    }
}
