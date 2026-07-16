use kfind_data::{
    MecabSourceMorphologyEntry, decode_component_resource, encode_component_resource,
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
fn whole_adverb_outranks_a_graph_built_nominal_particle_host() {
    let resolver = resolver();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext {
            previous: None,
            current: "너무",
            next: Some("보고"),
        },
        spans(0.."너무".len(), 0.."너무".len()),
        &[whole_pattern(DataFinePos::Mag, "너무")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
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
fn consumed_predicate_prefix_is_valid_inside_the_surrounding_token() {
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
        BoundedTokenContext::current("곱아다"),
        CandidateSpans {
            core: 0.."곱".len(),
            anchor: 0.."곱아".len(),
            consumed: 0.."곱아".len(),
            token: 0.."곱아다".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn copula_prefix_remains_valid_after_a_nominal_host() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Vcp, "이").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("학생일"),
        CandidateSpans {
            core: "학생".len().."학생이".len(),
            anchor: "학생".len().."학생일".len(),
            consumed: "학생".len().."학생일".len(),
            token: 0.."학생일".len(),
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

#[test]
fn exact_nominal_particle_host_outranks_a_longer_runtime_decomposition() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Nng, "학교").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("학교에서"),
        spans(0.."학교".len(), 0.."학교에서".len()),
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn exact_nominal_token_survives_a_graph_only_decomposition() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Nng, "선거운동").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("선거운동"),
        spans(0.."선거운동".len(), 0.."선거운동".len()),
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn graph_only_nominal_token_still_rejects_an_internal_substring() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Nng, "거운동").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("선거운동"),
        CandidateSpans {
            core: "선".len().."선거운동".len(),
            anchor: "선".len().."선거운동".len(),
            consumed: "선".len().."선거운동".len(),
            token: 0.."선거운동".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn multisyllable_nominal_prefix_survives_a_graph_built_particle_host() {
    let resolver = resolver();
    let core = 0.."둥그스름".len();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("둥그스름하게"),
        CandidateSpans {
            core: core.clone(),
            anchor: core.clone(),
            consumed: core,
            token: 0.."둥그스름하게".len(),
        },
        &[component_pattern(DataFinePos::Nng, "둥그스름")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn dependent_noun_after_a_proper_noun_consumes_its_particle() {
    let resolver = resolver();
    let core = "요코".len().."요코씨".len();
    let pattern = QueryMorphPattern::new(DataFinePos::Nnb, "씨").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("요코씨는"),
        CandidateSpans {
            core: core.clone(),
            anchor: core.clone(),
            consumed: core.start.."요코씨는".len(),
            token: 0.."요코씨는".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn one_syllable_suffix_without_a_proper_noun_frame_is_rejected() {
    let resolver = resolver();
    let core = "날".len().."날씨".len();
    let pattern = QueryMorphPattern::new(DataFinePos::Nnb, "씨").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("날씨는"),
        CandidateSpans {
            core: core.clone(),
            anchor: core.clone(),
            consumed: core.start.."날씨는".len(),
            token: 0.."날씨는".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn competing_predicate_and_nominal_continuations_remain_available() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Vv, "들").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("들지"),
        spans(0.."들".len(), 0.."들지".len()),
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn same_host_competition_still_requires_the_program_to_consume_its_ending() {
    let resolver = resolver();
    let bare = QueryMorphPattern::new(DataFinePos::Vv, "걸").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let completed = resolver.resolve_candidate(
        BoundedTokenContext::current("걸을"),
        CandidateSpans {
            core: 0.."걸".len(),
            anchor: 0.."걸을".len(),
            consumed: 0.."걸을".len(),
            token: 0.."걸을".len(),
        },
        std::slice::from_ref(&bare),
        128,
    );
    let incomplete = resolver.resolve_candidate(
        BoundedTokenContext::current("걸을"),
        CandidateSpans {
            core: 0.."걸".len(),
            anchor: 0.."걸".len(),
            consumed: 0.."걸".len(),
            token: 0.."걸을".len(),
        },
        &[bare],
        128,
    );

    assert_eq!(completed.outcome, ConstraintOutcome::Supported);
    assert_eq!(incomplete.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn adnominal_frame_selects_a_dependent_noun_over_a_homographic_predicate() {
    let resolver = resolver();
    let context = BoundedTokenContext {
        previous: Some("하는"),
        current: "걸",
        next: Some("보고"),
    };
    let noun = resolver.resolve_candidate(
        context,
        spans(0.."걸".len(), 0.."걸".len()),
        &[whole_pattern(DataFinePos::Nnb, "걸")],
        128,
    );
    let predicate = resolver.resolve_candidate(
        context,
        spans(0.."걸".len(), 0.."걸".len()),
        &[component_pattern(DataFinePos::Vv, "걸")],
        128,
    );

    assert_eq!(noun.outcome, ConstraintOutcome::Supported);
    assert_eq!(predicate.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn predicate_nominalization_aligns_with_whole_and_source_nominal_spans() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Vv, "걷").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: true,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let whole = resolver.resolve_candidate(
        BoundedTokenContext::current("걷기와"),
        CandidateSpans {
            core: 0.."걷".len(),
            anchor: 0.."걷기".len(),
            consumed: 0.."걷기와".len(),
            token: 0.."걷기와".len(),
        },
        std::slice::from_ref(&pattern),
        128,
    );
    let component = resolver.resolve_candidate(
        BoundedTokenContext::current("발걸음"),
        CandidateSpans {
            core: "발".len().."발걸".len(),
            anchor: "발".len().."발걸음".len(),
            consumed: "발".len().."발걸음".len(),
            token: 0.."발걸음".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(whole.outcome, ConstraintOutcome::Supported);
    assert_eq!(component.outcome, ConstraintOutcome::Supported);
}

#[test]
fn predicate_ending_path_consumes_an_open_ended_ending_sequence() {
    let resolver = resolver();

    assert!(resolver.supports_predicate_ending_path(
        "걷더니",
        "걷".len(),
        crate::PredicatePos::Verb,
        128,
    ));
    assert!(!resolver.supports_predicate_ending_path(
        "걷사람",
        "걷".len(),
        crate::PredicatePos::Verb,
        128,
    ));
}

#[test]
fn auxiliary_sequence_requires_an_auxiliary_predicate() {
    let resolver = resolver();

    assert!(resolver.supports_auxiliary_sequence("놓을", 128));
    assert!(!resolver.supports_auxiliary_sequence("능하게", 128));
}

#[test]
fn a_different_whole_predicate_blocks_a_prefix_fallback() {
    let resolver = resolver();

    assert!(resolver.whole_predicate_conflicts("걸려", "걸".len(), crate::PredicatePos::Verb,));
    assert!(!resolver.whole_predicate_conflicts("걷더니", "걷".len(), crate::PredicatePos::Verb,));
    assert!(resolver.whole_predicate_conflicts_at(
        "미친다",
        "미".len().."미친".len(),
        crate::PredicatePos::Verb,
    ));
}

#[test]
fn predicate_ending_does_not_become_a_terminal_nominal_component() {
    let resolver = resolver();
    let start = "입니".len();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("입니다"),
        CandidateSpans {
            core: start.."입니다".len(),
            anchor: start.."입니다".len(),
            consumed: start.."입니다".len(),
            token: 0.."입니다".len(),
        },
        &[component_pattern(DataFinePos::Nng, "다")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn glued_dependent_noun_after_an_adnominal_ending_remains_supported() {
    let resolver = resolver();
    let start = "공부한".len();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("공부한지"),
        CandidateSpans {
            core: start.."공부한지".len(),
            anchor: start.."공부한지".len(),
            consumed: start.."공부한지".len(),
            token: 0.."공부한지".len(),
        },
        &[component_pattern(DataFinePos::Nnb, "지")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
}

#[test]
fn whole_token_predicate_program_does_not_require_a_dictionary_surface() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Va, "정의롭").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("정의롭지"),
        spans(0.."정의롭".len(), 0.."정의롭지".len()),
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn runtime_surface_with_another_pos_does_not_support_the_query_pattern() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Vv, "가").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("가계"),
        CandidateSpans {
            core: 0.."가".len(),
            anchor: 0.."가".len(),
            consumed: 0.."가".len(),
            token: 0.."가계".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn different_nominal_and_predicate_hosts_do_not_force_ambiguity() {
    let resolver = resolver();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("때문에"),
        CandidateSpans {
            core: 0.."때".len(),
            anchor: 0.."때".len(),
            consumed: 0.."때".len(),
            token: 0.."때문에".len(),
        },
        &[component_pattern(DataFinePos::Nng, "때")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn exact_modifier_inside_an_unknown_token_is_not_a_component() {
    let resolver = resolver();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("유면한"),
        CandidateSpans {
            core: "유면".len().."유면한".len(),
            anchor: "유면".len().."유면한".len(),
            consumed: "유면".len().."유면한".len(),
            token: 0.."유면한".len(),
        },
        &[whole_pattern(DataFinePos::Mm, "한")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

fn resolver() -> ConstraintResolver {
    let entries = [
        atomic("매", "NNG"),
        atomic("매일", "MAG"),
        atomic("매일", "NNG"),
        atomic("너", "NNG"),
        atomic("무", "JX"),
        atomic("너무", "MAG"),
        atomic("을", "JKO"),
        atomic("학교", "NNG"),
        atomic("에", "NNG"),
        atomic("에서", "JKB"),
        atomic("서", "JKB"),
        atomic("둥그스름", "NNG"),
        atomic("하", "NNG"),
        atomic("게", "JKB"),
        atomic("요코", "NNP"),
        atomic("씨", "NNB"),
        atomic("요코씨", "NNP"),
        atomic("날", "NNG"),
        atomic("날씨", "NNG"),
        atomic("는", "JX"),
        atomic("을", "ETM"),
        atomic("보고", "VV+EC"),
        atomic("아니라", "VCN+EC"),
        atomic("수도", "NNB+JX"),
        expression("일", "VCP+ETM", "이/VCP/*+ᆯ/ETM/*"),
        expression("걸었고", "VV+EP+EC", "걸/VV/*+었/EP/*+고/EC/*"),
        expression("곱아", "VA+EC", "곱/VA/*+아/EC/*"),
        atomic("다", "EF"),
        atomic("들", "VV"),
        atomic("들", "NNB"),
        atomic("지", "EC"),
        atomic("지", "JX"),
        atomic("정의", "NNG"),
        atomic("롭지", "NNG"),
        atomic("가", "NNG"),
        atomic("계", "NNG"),
        atomic("가계", "NNB"),
        atomic("때", "NNG"),
        atomic("때", "VV"),
        atomic("때문", "NNB"),
        atomic("문에", "EC"),
        atomic("에", "JKB"),
        atomic("유", "NNG"),
        atomic("면", "NNG"),
        atomic("유면", "NNG"),
        atomic("면한", "NNG"),
        atomic("한", "MM"),
        atomic("걸", "VV"),
        atomic("걸", "NNB"),
        expression("하는", "VV+ETM", "하/VV/*+는/ETM/*"),
        atomic("걷", "VV"),
        atomic("더니", "EC"),
        atomic("사람", "NNG"),
        atomic("걸", "VV"),
        expression("걸려", "VV+EC", "걸리/VV/*+어/EC/*"),
        expression("미친다", "VV+EF", "미치/VV/*+ᆫ다/EF/*"),
        atomic("입니", "VCP+EF"),
        expression("입니다", "VCP+EF", "이/VCP/*+ᆸ니다/EF/*"),
        atomic("다", "NNG"),
        atomic("공부한", "NNG+XSV+ETM"),
        atomic("지", "NNB"),
        atomic("걷기", "NNG"),
        atomic("와", "JC"),
        expression("발걸음", "NNG", "발/NNG/*+걸음/NNG/*"),
        atomic("선거", "NNG"),
        atomic("운동", "NNG"),
        atomic("놓", "VX"),
        atomic("을", "ETM"),
        atomic("능하", "VA"),
        atomic("게", "EC"),
    ];
    let bytes = encode_component_resource([9; 32], &entries).expect("valid resource");
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
