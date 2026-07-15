use std::io::Cursor;
use std::ops::Range;

use kfind_data::{
    DataFinePos, MecabSourceMorphologyEntry, decode_morphology_graph_resource,
    encode_morphology_graph_resource, parse_mecab_connection_matrix,
};

use crate::{ContinuationState, FinePos};

use super::*;

#[test]
fn whole_source_analysis_is_supported_without_using_costs() {
    let resolver = resolver(&[atomic("학교", "NNG", -9_999)]);
    let pattern = exact_pattern("학교", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let resolution = resolver.resolve(
        "학교",
        0.."학교".len(),
        0.."학교".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(resolution.proof.known_node_count, 1);
    assert_eq!(resolution.supported.analyses.len(), 1);
    assert_eq!(
        resolution.supported.analyses[0].span_relation,
        ConstraintSpanRelation::Whole
    );
    assert!(ProductPolicy::Whole.accepts(&resolution, std::slice::from_ref(&pattern)));
}

#[test]
fn compact_decision_matches_diagnostic_resolution() {
    let resolver = resolver(&[atomic("학교", "NNG", 0)]);
    let pattern = exact_pattern("학교", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let spans = CandidateSpans {
        core: 0.."학교".len(),
        anchor: 0.."학교".len(),
        consumed: 0.."학교".len(),
        token: 0.."학교".len(),
    };
    let context = BoundedTokenContext::current("학교");
    let prepared = resolver.prepare_token("학교", DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT);
    let decision = resolver.decide_prepared_candidate(
        &prepared,
        context,
        spans.clone(),
        std::slice::from_ref(&pattern),
    );
    let resolution = resolver.resolve_prepared_candidate(
        &prepared,
        BoundedTokenContext::current("학교"),
        spans,
        std::slice::from_ref(&pattern),
    );

    assert_eq!(decision, resolution.decision());
    for policy in [
        ProductPolicy::Whole,
        ProductPolicy::ExplicitComponent,
        ProductPolicy::PossibleAnalysis,
        ProductPolicy::UnambiguousAnalysis,
    ] {
        assert_eq!(
            policy.accepts_decision(&decision, std::slice::from_ref(&pattern)),
            policy.accepts(&resolution, std::slice::from_ref(&pattern))
        );
    }
}

#[test]
fn source_component_is_preserved_for_an_explicit_policy_decision() {
    let resolver = resolver(&[entry(
        "대학교",
        "NNG",
        "Compound",
        "NNG",
        "NNG",
        "대/NNG/*+학교/NNG/*",
        0,
    )]);
    let hidden = exact_pattern("학교", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let exposed = exact_pattern("학교", DataFinePos::Nng, ComponentCapability::Source);
    let spans = CandidateSpans {
        core: "대".len().."대학교".len(),
        anchor: "대".len().."대학교".len(),
        consumed: "대".len().."대학교".len(),
        token: 0.."대학교".len(),
    };
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("대학교"),
        spans,
        std::slice::from_ref(&hidden),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    );
    assert_eq!(
        resolution.supported.analyses[0].span_relation,
        ConstraintSpanRelation::SourceComponent
    );
    assert!(!ProductPolicy::Whole.accepts(&resolution, std::slice::from_ref(&hidden)));
    assert!(!ProductPolicy::ExplicitComponent.accepts(&resolution, std::slice::from_ref(&hidden)));
    assert!(ProductPolicy::ExplicitComponent.accepts(&resolution, std::slice::from_ref(&exposed)));
    assert!(ProductPolicy::PossibleAnalysis.accepts(&resolution, std::slice::from_ref(&hidden)));
    assert!(
        !ProductPolicy::UnambiguousAnalysis.accepts(&resolution, std::slice::from_ref(&hidden))
    );
}

#[test]
fn runtime_component_requires_runtime_capability_under_explicit_policy() {
    let resolver = resolver(&[
        atomic("산", "NNG", 8_000),
        atomic("속", "NNG", -8_000),
        noun_compound_transition(),
    ]);
    let source_only = exact_pattern("속", DataFinePos::Nng, ComponentCapability::Source);
    let runtime = exact_pattern(
        "속",
        DataFinePos::Nng,
        ComponentCapability::SourceAndRuntime,
    );
    let resolution = resolver.resolve(
        "산속",
        "산".len().."산속".len(),
        "산".len().."산속".len(),
        &source_only,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    );
    assert_eq!(
        resolution.supported.analyses[0].span_relation,
        ConstraintSpanRelation::RuntimeComponent
    );
    assert!(
        !ProductPolicy::ExplicitComponent.accepts(&resolution, std::slice::from_ref(&source_only))
    );
    assert!(ProductPolicy::ExplicitComponent.accepts(&resolution, std::slice::from_ref(&runtime)));
}

#[test]
fn productive_derivation_can_supply_runtime_lexical_identity() {
    let resolver = resolver(&[
        atomic("실행", "NNG", 0),
        atomic("하", "XSV", 0),
        atomic("지", "EC", 0),
        entry(
            "가하",
            "VV",
            "Compound",
            "NNG",
            "XSV",
            "가/NNG/*+하/XSV/*",
            0,
        ),
        entry("오지", "VV", "Compound", "XSV", "EC", "오/XSV/*+지/EC/*", 0),
    ]);
    let pattern = predicate_pattern("실행하", ContinuationState::AOrEo);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("실행하지"),
        CandidateSpans {
            core: 0.."실행하".len(),
            anchor: 0.."실행하".len(),
            consumed: 0.."실행하지".len(),
            token: 0.."실행하지".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert!(resolution.supported.analyses.iter().any(|analysis| {
        analysis.evidence == ConstraintEvidenceKind::RuntimeComposed
            && analysis.lexical_source_node_indices.len() == 2
    }));
}

#[test]
fn productive_lexical_identity_can_span_three_connected_nodes() {
    let resolver = resolver(&[
        atomic("초", "XPN", 0),
        atomic("실행", "NNG", 0),
        atomic("하", "XSV", 0),
        entry(
            "가나다",
            "VV",
            "Compound",
            "XPN",
            "XSV",
            "가/XPN/*+나/NNG/*+다/XSV/*",
            0,
        ),
    ]);
    let pattern = predicate_pattern("초실행하", ContinuationState::AOrEo);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("초실행하"),
        CandidateSpans {
            core: 0.."초실행하".len(),
            anchor: 0.."초실행하".len(),
            consumed: 0.."초실행하".len(),
            token: 0.."초실행하".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert!(resolution.supported.analyses.iter().any(|analysis| {
        analysis.evidence == ConstraintEvidenceKind::RuntimeComposed
            && analysis.lexical_source_node_indices.len() == 3
    }));
}

#[test]
fn fused_derivational_ending_uses_the_enclosing_token_span() {
    let resolver = resolver(&[
        atomic("접근", "NNG", 0),
        entry(
            "할",
            "XSV+ETM",
            "Inflect",
            "XSV",
            "ETM",
            "하/XSV/*+ᆯ/ETM/*",
            0,
        ),
        entry(
            "가하",
            "VV",
            "Compound",
            "NNG",
            "XSV",
            "가/NNG/*+하/XSV/*",
            0,
        ),
    ]);
    let pattern = predicate_pattern("접근하", ContinuationState::Terminal);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("접근할"),
        CandidateSpans {
            core: 0.."접근할".len(),
            anchor: 0.."접근할".len(),
            consumed: 0.."접근할".len(),
            token: 0.."접근할".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert!(resolution.supported.analyses.iter().any(|analysis| {
        analysis.evidence == ConstraintEvidenceKind::OpaqueExpression
            && analysis.span_relation == ConstraintSpanRelation::RuntimeComponent
    }));
}

#[test]
fn scoring_only_duplicates_collapse_to_one_structural_analysis() {
    let resolver = resolver(&[
        atomic("매일", "MAG", -30_000),
        atomic("매일", "MAG", 30_000),
        atomic("매일", "NNG", 0),
    ]);
    let pattern = exact_pattern("매일", DataFinePos::Mag, ComponentCapability::WholeOnly);
    let resolution = resolver.resolve(
        "매일",
        0.."매일".len(),
        0.."매일".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::LexicalCompetition)
    );
    assert_eq!(resolution.proof.paths.len(), 1);
    assert_eq!(resolution.supported.analyses.len(), 1);
}

#[test]
fn dense_connection_matrix_does_not_license_an_unobserved_transition() {
    let resolver = resolver(&[atomic("산", "NNG", 8_000), atomic("속", "NNG", -8_000)]);
    let pattern = exact_pattern("속", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let resolution = resolver.resolve(
        "산속",
        "산".len().."산속".len(),
        "산".len().."산속".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Unavailable(ConstraintUnavailable::UnknownOnly)
    );
}

#[test]
fn pattern_union_does_not_treat_sibling_pos_candidates_as_contradictions() {
    let resolver = resolver(&[atomic("아니", "VCN", 0)]);
    let patterns = QueryMorphPattern::from_fine_pos(FinePos::Adjective, "아니");
    let resolution = resolver.resolve_patterns(
        "아니",
        0.."아니".len(),
        0.."아니".len(),
        &patterns,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(resolution.supported.analyses[0].pattern_index, 1);
}

#[test]
fn lexical_identity_is_required_in_addition_to_pos() {
    let resolver = resolver(&[atomic("매일", "MAG", 0)]);
    let pattern = exact_pattern("내일", DataFinePos::Mag, ComponentCapability::WholeOnly);
    let resolution = resolver.resolve(
        "매일",
        0.."매일".len(),
        0.."매일".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn unprojectable_expression_is_ambiguous_without_an_invented_span() {
    let resolver = resolver(&[entry(
        "갔다",
        "VV+EP+EF",
        "Inflect",
        "VV",
        "EF",
        "가/VV/*+었/EP/*+다/EF/*",
        0,
    )]);
    let pattern = exact_pattern("가", DataFinePos::Vv, ComponentCapability::SourceAndRuntime);
    let resolution = resolver.resolve(
        "갔다",
        0.."갔".len(),
        0.."갔".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::OpaqueExpression)
    );
    assert_eq!(
        resolution.supported.analyses[0].span_relation,
        ConstraintSpanRelation::OpaqueExpression
    );
    assert!(!ProductPolicy::PossibleAnalysis.accepts(&resolution, std::slice::from_ref(&pattern)));
}

#[test]
fn opaque_component_is_stable_when_the_enclosing_node_is_returned() {
    let resolver = resolver(&[entry(
        "갔다",
        "VV+EP+EF",
        "Inflect",
        "VV",
        "EF",
        "가/VV/*+었/EP/*+다/EF/*",
        0,
    )]);
    let pattern = predicate_pattern("가", ContinuationState::Past);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("갔다"),
        CandidateSpans {
            core: 0.."갔".len(),
            anchor: 0.."갔다".len(),
            consumed: 0.."갔다".len(),
            token: 0.."갔다".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert!(resolution.supported.analyses.iter().any(|analysis| {
        analysis.evidence == ConstraintEvidenceKind::OpaqueExpression
            && analysis.span_relation == ConstraintSpanRelation::RuntimeComponent
    }));
}

#[test]
fn opaque_anchor_tail_advances_the_predicate_continuation() {
    let text = "새로움의";
    let resolver = resolver(&[
        entry(
            "새로움",
            "VA+ETN",
            "Inflect",
            "VA",
            "ETN",
            "새롭/VA/*+ᄆ/ETN/*",
            0,
        ),
        atomic("의", "JKG", 0),
        entry(
            "가의",
            "VA+ETN+JKG",
            "Inflect",
            "VA",
            "JKG",
            "가/VA/*+ᄆ/ETN/*+의/JKG/*",
            0,
        ),
    ]);
    let pattern = QueryMorphPattern::new(DataFinePos::Va, "새롭").with_branch_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: ContinuationState::Terminal,
            nominal_particles: true,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        CandidateSpans {
            core: 0.."새로움".len(),
            anchor: 0.."새로움".len(),
            consumed: 0..text.len(),
            token: 0..text.len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(
        resolver.decide_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: 0.."새로움".len(),
                anchor: 0.."새로움".len(),
                consumed: 0..text.len(),
                token: 0..text.len(),
            },
            std::slice::from_ref(&pattern),
            DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        ),
        resolution.decision()
    );
    assert!(resolution.supported.analyses.iter().any(|analysis| {
        analysis
            .continuation
            .units
            .iter()
            .map(|unit| unit.pos.as_str())
            .eq(["ETN", "JKG"])
    }));
}

#[test]
fn opaque_query_anchor_ending_is_not_reconsumed_as_external_continuation() {
    let text = "불어서";
    let resolver = resolver(&[
        entry("불어", "VV+EC", "Inflect", "VV", "EC", "불/VV/*+ㅓ/EC/*", 0),
        atomic("서", "EC", 0),
        entry(
            "가어서",
            "VV+EC+EC",
            "Inflect",
            "VV",
            "EC",
            "가/VV/*+어/EC/*+서/EC/*",
            0,
        ),
    ]);
    let pattern = predicate_pattern("불", ContinuationState::AOrEo);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        CandidateSpans {
            core: 0.."불어".len(),
            anchor: 0.."불어".len(),
            consumed: 0..text.len(),
            token: 0..text.len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Supported,
        "{resolution:#?}"
    );
    let continuations = resolution
        .supported
        .analyses
        .iter()
        .map(|analysis| {
            analysis
                .continuation
                .units
                .iter()
                .map(|unit| unit.pos.as_str())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert!(
        continuations.iter().any(|positions| positions == &["EC"]),
        "{continuations:?}"
    );
}

#[test]
fn alternative_opaque_lexical_identity_remains_ambiguous() {
    let resolver = resolver(&[
        entry(
            "걸었다",
            "VV+EP+EF",
            "Inflect",
            "VV",
            "EF",
            "걷/VV/*+었/EP/*+다/EF/*",
            0,
        ),
        entry(
            "걸었다",
            "VV+EP+EF",
            "Inflect",
            "VV",
            "EF",
            "걸/VV/*+었/EP/*+다/EF/*",
            0,
        ),
    ]);
    let pattern = predicate_pattern("걷", ContinuationState::Past);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("걸었다"),
        CandidateSpans {
            core: 0.."걸".len(),
            anchor: 0.."걸었다".len(),
            consumed: 0.."걸었다".len(),
            token: 0.."걸었다".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::LexicalCompetition)
    );
    assert!(ProductPolicy::PossibleAnalysis.accepts(&resolution, std::slice::from_ref(&pattern)));
    assert!(
        !ProductPolicy::UnambiguousAnalysis.accepts(&resolution, std::slice::from_ref(&pattern))
    );
}

#[test]
fn nominal_continuation_is_proved_by_pos_transitions() {
    let text = "학교는";
    let resolver = resolver(&[
        atomic("학교", "NNG", 0),
        atomic("는", "JX", 0),
        entry(
            "가는",
            "NNG+JX",
            "Preanalysis",
            "NNG",
            "JX",
            "가/NNG/*+는/JX/*",
            0,
        ),
    ]);
    let pattern = nominal_pattern("학교", DataFinePos::Nng);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        CandidateSpans {
            core: 0.."학교".len(),
            anchor: 0.."학교".len(),
            consumed: 0..text.len(),
            token: 0..text.len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(
        resolution.supported.analyses[0].continuation.units[0].pos,
        "JX"
    );
}

#[test]
fn nominal_continuation_rejects_a_non_particle_suffix() {
    let text = "학교가다";
    let resolver = resolver(&[
        atomic("학교", "NNG", 0),
        atomic("가다", "VV", 0),
        entry(
            "학교가다",
            "NNG+VV",
            "Preanalysis",
            "NNG",
            "VV",
            "학교/NNG/*+가다/VV/*",
            0,
        ),
    ]);
    let pattern = nominal_pattern("학교", DataFinePos::Nng);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        CandidateSpans {
            core: 0.."학교".len(),
            anchor: 0.."학교".len(),
            consumed: 0..text.len(),
            token: 0..text.len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn nominal_continuation_validates_particle_allomorph_and_case_order() {
    let resolver = resolver(&[
        atomic("기", "NNG", 0),
        atomic("이", "JKS", 0),
        atomic("이", "JX", 0),
        atomic("가", "JKS", 0),
        atomic("를", "JKO", 0),
        atomic("를", "JX", 0),
        entry(
            "산이",
            "NNG+JKS",
            "Preanalysis",
            "NNG",
            "JKS",
            "산/NNG/*+이/JKS/*",
            0,
        ),
        entry(
            "이가",
            "JKS+JKO",
            "Preanalysis",
            "JKS",
            "JKO",
            "이/JKS/*+가/JKO/*",
            0,
        ),
        entry(
            "이는",
            "JKS+JX",
            "Preanalysis",
            "JKS",
            "JX",
            "이/JKS/*+는/JX/*",
            0,
        ),
    ]);
    let pattern = nominal_pattern("기", DataFinePos::Nng);
    let resolve = |text: &str| {
        resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: 0.."기".len(),
                anchor: 0.."기".len(),
                consumed: 0..text.len(),
                token: 0..text.len(),
            },
            std::slice::from_ref(&pattern),
            DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        )
    };

    assert_eq!(resolve("기가").outcome, ConstraintOutcome::Supported);
    assert_eq!(resolve("기이").outcome, ConstraintOutcome::Contradicted);
    assert_eq!(resolve("기가를").outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn predicate_continuation_uses_the_query_state_and_pos_dfa() {
    let text = "가었다";
    let resolver = resolver(&[
        atomic("가", "VV", 0),
        atomic("었", "EP", 0),
        atomic("다", "EF", 0),
        entry(
            "가었다",
            "VV+EP+EF",
            "Preanalysis",
            "VV",
            "EF",
            "가/VV/*+었/EP/*+다/EF/*",
            0,
        ),
    ]);
    let pattern = predicate_pattern("가", ContinuationState::Past);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        CandidateSpans {
            core: 0.."가".len(),
            anchor: 0.."가".len(),
            consumed: 0..text.len(),
            token: 0..text.len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(
        resolution.supported.analyses[0]
            .continuation
            .units
            .iter()
            .map(|unit| unit.pos.as_str())
            .collect::<Vec<_>>(),
        ["EP", "EF"]
    );

    let terminal = predicate_pattern("가", ContinuationState::Terminal);
    let rejected = resolver.resolve_candidate(
        BoundedTokenContext::current(text),
        CandidateSpans {
            core: 0.."가".len(),
            anchor: 0.."가".len(),
            consumed: 0..text.len(),
            token: 0..text.len(),
        },
        std::slice::from_ref(&terminal),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    assert_eq!(rejected.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn repeated_context_selects_the_adverb_analysis_without_a_surface_registry() {
    let resolver = resolver(&[atomic("매일", "MAG", 0), atomic("매일", "NNG", 0)]);
    let adverb = exact_pattern("매일", DataFinePos::Mag, ComponentCapability::WholeOnly);
    let noun = exact_pattern("매일", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let context = BoundedTokenContext {
        previous: Some("매일"),
        current: "매일",
        next: None,
    };
    let spans = CandidateSpans {
        core: 0.."매일".len(),
        anchor: 0.."매일".len(),
        consumed: 0.."매일".len(),
        token: 0.."매일".len(),
    };
    let adverb_resolution = resolver.resolve_candidate(
        context,
        spans.clone(),
        std::slice::from_ref(&adverb),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    let noun_resolution = resolver.resolve_candidate(
        context,
        spans,
        std::slice::from_ref(&noun),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(adverb_resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(
        adverb_resolution.supported.analyses[0].context,
        Some(ConstraintContextProof::RepeatedToken {
            side: AdjacentSide::Previous
        })
    );
    assert_eq!(noun_resolution.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn particle_context_selects_the_complete_nominal_host() {
    let resolver = resolver(&[
        atomic("매일", "NNG", 0),
        atomic("매일", "MAG", 0),
        atomic("매", "NNG", 0),
        atomic("일", "NNG", 0),
        atomic("을", "JKO", 0),
        entry(
            "산을",
            "NNG+JKO",
            "Preanalysis",
            "NNG",
            "JKO",
            "산/NNG/*+을/JKO/*",
            0,
        ),
        noun_compound_transition(),
    ]);
    let context = BoundedTokenContext::current("매일을");
    let host = nominal_pattern("매일", DataFinePos::Nng);
    let component = nominal_pattern("매", DataFinePos::Nng);
    let adverb = exact_pattern(
        "매일",
        DataFinePos::Mag,
        ComponentCapability::SourceAndRuntime,
    );
    let resolve = |pattern: &QueryMorphPattern, core: Range<usize>, consumed: Range<usize>| {
        resolver.resolve_candidate(
            context,
            CandidateSpans {
                anchor: core.clone(),
                core,
                consumed,
                token: 0.."매일을".len(),
            },
            std::slice::from_ref(pattern),
            DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        )
    };

    let host_resolution = resolve(&host, 0.."매일".len(), 0.."매일을".len());
    assert_eq!(host_resolution.outcome, ConstraintOutcome::Supported);
    assert!(matches!(
        host_resolution.supported.analyses[0].context,
        Some(ConstraintContextProof::NominalParticleHost { .. })
    ));
    assert_eq!(
        resolve(&component, 0.."매".len(), 0.."매".len()).outcome,
        ConstraintOutcome::Contradicted
    );
    assert_eq!(
        resolve(&adverb, 0.."매일".len(), 0.."매일".len()).outcome,
        ConstraintOutcome::Contradicted
    );
}

#[test]
fn particle_context_preserves_multiple_structural_hosts() {
    let resolver = resolver(&[
        atomic("산", "NNG", 0),
        atomic("산길", "NNG", 0),
        atomic("길", "JX", 0),
        atomic("을", "JKO", 0),
        entry(
            "산길",
            "NNG+JX",
            "Preanalysis",
            "NNG",
            "JX",
            "산/NNG/*+길/JX/*",
            0,
        ),
        entry(
            "산을",
            "NNG+JKO",
            "Preanalysis",
            "NNG",
            "JKO",
            "산/NNG/*+을/JKO/*",
            0,
        ),
        entry(
            "길을",
            "JX+JKO",
            "Preanalysis",
            "JX",
            "JKO",
            "길/JX/*+을/JKO/*",
            0,
        ),
    ]);
    let context = BoundedTokenContext::current("산길을");
    let resolve = |lexical_form: &str, core: Range<usize>| {
        resolver.resolve_candidate(
            context,
            CandidateSpans {
                anchor: core.clone(),
                core,
                consumed: 0.."산길을".len(),
                token: 0.."산길을".len(),
            },
            std::slice::from_ref(&nominal_pattern(lexical_form, DataFinePos::Nng)),
            DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        )
    };

    let short = resolve("산", 0.."산".len());
    let long = resolve("산길", 0.."산길".len());

    assert_eq!(short.outcome, ConstraintOutcome::Supported);
    assert_eq!(long.outcome, ConstraintOutcome::Supported);
    assert!(matches!(
        short.supported.analyses[0].context,
        Some(ConstraintContextProof::NominalParticleHost { ref selected })
            if *selected == (0.."산".len())
    ));
    assert!(matches!(
        long.supported.analyses[0].context,
        Some(ConstraintContextProof::NominalParticleHost { ref selected })
            if *selected == (0.."산길".len())
    ));
}

#[test]
fn particle_context_does_not_filter_an_unrelated_whole_analysis() {
    let resolver = resolver(&[
        atomic("그대로", "MAG", 0),
        entry(
            "그대로",
            "NP+JKB",
            "Preanalysis",
            "NP",
            "JKB",
            "그대/NP/*+로/JKB/*",
            0,
        ),
    ]);
    let pattern = exact_pattern("그대로", DataFinePos::Mag, ComponentCapability::WholeOnly);
    let resolution = resolver.resolve(
        "그대로",
        0.."그대로".len(),
        0.."그대로".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::LexicalCompetition)
    );
    assert!(ProductPolicy::Whole.accepts(&resolution, std::slice::from_ref(&pattern)));
    assert!(
        !ProductPolicy::UnambiguousAnalysis.accepts(&resolution, std::slice::from_ref(&pattern))
    );
}

#[test]
fn particle_shaped_suffix_does_not_hide_a_complete_predicate_analysis() {
    let resolver = resolver(&[
        atomic("생성", "NNG", 0),
        atomic("하", "XSV", 0),
        atomic("고", "EC", 0),
        atomic("하고", "JKB", 0),
        entry(
            "산하고",
            "NNG+JKB",
            "Preanalysis",
            "NNG",
            "JKB",
            "산/NNG/*+하고/JKB/*",
            0,
        ),
        entry(
            "가하",
            "NNG+XSV",
            "Preanalysis",
            "NNG",
            "XSV",
            "가/NNG/*+하/XSV/*",
            0,
        ),
        entry(
            "하고",
            "XSV+EC",
            "Preanalysis",
            "XSV",
            "EC",
            "하/XSV/*+고/EC/*",
            0,
        ),
    ]);
    let pattern = predicate_pattern("생성하", ContinuationState::AOrEo);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext::current("생성하고"),
        CandidateSpans {
            core: 0.."생성하".len(),
            anchor: 0.."생성하".len(),
            consumed: 0.."생성하고".len(),
            token: 0.."생성하고".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
}

#[test]
fn copular_context_selects_the_unique_nominal_prefix() {
    let resolver = resolver(&[
        atomic("아니라", "VCN+EC", 0),
        atomic("매", "NNG", 0),
        entry(
            "일",
            "VCP+ETM",
            "Inflect",
            "VCP",
            "ETM",
            "이/VCP/*+ᆯ/ETM/*",
            0,
        ),
        atomic("매일", "NNG", 0),
        atomic("것", "NNB", 0),
        entry(
            "가일",
            "NNG+VCP+ETM",
            "Preanalysis",
            "NNG",
            "ETM",
            "가/NNG/*+이/VCP/*+ㄹ/ETM/*",
            0,
        ),
    ]);
    let context = BoundedTokenContext {
        previous: Some("아니라"),
        current: "매일",
        next: Some("것"),
    };
    let prefix = exact_pattern(
        "매",
        DataFinePos::Nng,
        ComponentCapability::SourceAndRuntime,
    );
    let whole = exact_pattern("매일", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let prefix_resolution = resolver.resolve_candidate(
        context,
        CandidateSpans {
            core: 0.."매".len(),
            anchor: 0.."매".len(),
            consumed: 0.."매".len(),
            token: 0.."매일".len(),
        },
        std::slice::from_ref(&prefix),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    let whole_resolution = resolver.resolve_candidate(
        context,
        CandidateSpans {
            core: 0.."매일".len(),
            anchor: 0.."매일".len(),
            consumed: 0.."매일".len(),
            token: 0.."매일".len(),
        },
        std::slice::from_ref(&whole),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(prefix_resolution.outcome, ConstraintOutcome::Supported);
    assert_eq!(
        prefix_resolution.supported.analyses[0].context,
        Some(ConstraintContextProof::CopularFrame {
            role: CopularFrameRole::Nominal,
            selected: 0.."매".len(),
        })
    );
    assert_eq!(whole_resolution.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn path_limit_counts_distinct_support_proofs_not_irrelevant_prefix_combinations() {
    let resolver = resolver(&[
        atomic("산", "NNG", 0),
        entry("산", "NNG", "Compound", "NNG", "NNG", "*", 0),
        atomic("속", "NNG", 0),
        entry("속", "NNG", "Compound", "NNG", "NNG", "*", 0),
        noun_compound_transition(),
    ]);
    let pattern = exact_pattern(
        "속",
        DataFinePos::Nng,
        ComponentCapability::SourceAndRuntime,
    );
    let resolution = resolver.resolve_candidate_with_limits(
        BoundedTokenContext::current("산속"),
        CandidateSpans {
            core: "산".len().."산속".len(),
            anchor: "산".len().."산속".len(),
            consumed: "산".len().."산속".len(),
            token: 0.."산속".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        2,
    );

    assert_eq!(
        resolution.outcome,
        ConstraintOutcome::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    );
    assert_eq!(resolution.supported.analyses.len(), 2);

    let limited = resolver.resolve_candidate_with_limits(
        BoundedTokenContext::current("산속"),
        CandidateSpans {
            core: "산".len().."산속".len(),
            anchor: "산".len().."산속".len(),
            consumed: "산".len().."산속".len(),
            token: 0.."산속".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        1,
    );
    assert_eq!(
        limited.outcome,
        ConstraintOutcome::Unavailable(ConstraintUnavailable::PathLimit {
            actual: 2,
            limit: 1,
        })
    );
    let limited_decision = resolver.decide_candidate_with_limits(
        BoundedTokenContext::current("산속"),
        CandidateSpans {
            core: "산".len().."산속".len(),
            anchor: "산".len().."산속".len(),
            consumed: "산".len().."산속".len(),
            token: 0.."산속".len(),
        },
        std::slice::from_ref(&pattern),
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        1,
    );
    assert_eq!(limited_decision, limited.decision());
}

#[test]
fn unknown_paths_are_used_only_when_no_known_complete_path_exists() {
    let resolver = resolver(&[atomic("학교", "NNG", 0)]);
    let unknown_pattern = exact_pattern("미", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let unknown = resolver.resolve(
        "미등록",
        0.."미".len(),
        0.."미".len(),
        &unknown_pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        unknown.outcome,
        ConstraintOutcome::Unavailable(ConstraintUnavailable::UnknownOnly)
    );
    assert!(unknown.proof.unknown_node_count > 0);
    assert!(
        unknown
            .proof
            .paths
            .iter()
            .all(|path| path.evidence == ConstraintEvidenceKind::Unknown)
    );
}

#[test]
fn invalid_spans_and_node_limits_are_observable() {
    let resolver = resolver(&[atomic("산", "NNG", 0), atomic("산속", "NNG", 0)]);
    let pattern = exact_pattern("산", DataFinePos::Nng, ComponentCapability::WholeOnly);
    assert_eq!(
        resolver
            .resolve(
                "산속",
                1..2,
                1..2,
                &pattern,
                DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT
            )
            .outcome,
        ConstraintOutcome::Unavailable(ConstraintUnavailable::InvalidPattern)
    );
    assert!(matches!(
        resolver
            .resolve("산속", 0.."산".len(), 0.."산".len(), &pattern, 1)
            .outcome,
        ConstraintOutcome::Unavailable(ConstraintUnavailable::NodeLimit { limit: 1, .. })
    ));
}

#[test]
fn adjective_patterns_preserve_the_negative_copula_candidate() {
    assert_eq!(
        QueryMorphPattern::from_fine_pos(FinePos::Adjective, "아니")
            .into_iter()
            .map(|pattern| pattern.fine_pos)
            .collect::<Vec<_>>(),
        [DataFinePos::Va, DataFinePos::Vcn]
    );
}

#[test]
fn non_copular_current_token_does_not_build_adjacent_graphs() {
    let resolver = resolver(&[
        atomic("학교", "NNG", 0),
        atomic("가", "NNG", 0),
        atomic("가", "VV", 0),
        atomic("나", "NNG", 0),
        atomic("나", "VV", 0),
    ]);
    let pattern = exact_pattern("학교", DataFinePos::Nng, ComponentCapability::WholeOnly);
    let resolution = resolver.resolve_candidate(
        BoundedTokenContext {
            previous: Some("가"),
            current: "학교",
            next: Some("나"),
        },
        CandidateSpans {
            core: 0.."학교".len(),
            anchor: 0.."학교".len(),
            consumed: 0.."학교".len(),
            token: 0.."학교".len(),
        },
        std::slice::from_ref(&pattern),
        1,
    );

    assert_eq!(resolution.outcome, ConstraintOutcome::Supported);
}

fn exact_pattern(
    lexical_form: &str,
    fine_pos: DataFinePos,
    capability: ComponentCapability,
) -> QueryMorphPattern {
    QueryMorphPattern::new(fine_pos, lexical_form).with_branch_contract(
        CandidateTokenRelation::Whole,
        MorphContinuation::Exact,
        capability,
    )
}

fn nominal_pattern(lexical_form: &str, fine_pos: DataFinePos) -> QueryMorphPattern {
    QueryMorphPattern::new(fine_pos, lexical_form).with_branch_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    )
}

fn predicate_pattern(lexical_form: &str, state: ContinuationState) -> QueryMorphPattern {
    QueryMorphPattern::new(DataFinePos::Vv, lexical_form).with_branch_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    )
}

fn resolver(entries: &[MecabSourceMorphologyEntry]) -> ConstraintResolver {
    let matrix = parse_mecab_connection_matrix(
        "matrix.def",
        Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
    )
    .unwrap();
    let bytes = encode_morphology_graph_resource(
        [5; 32],
        entries,
        &matrix,
        b"DEFAULT 0 1 0\nHANGUL 1 1 8\n0xAC00..0xD7A3 HANGUL\n",
        b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    let resource = decode_morphology_graph_resource("fixture", bytes, &[5; 32]).unwrap();
    ConstraintResolver::new(Arc::new(resource))
}

fn atomic(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
    entry(surface, pos, "*", "*", "*", "*", word_cost)
}

fn noun_compound_transition() -> MecabSourceMorphologyEntry {
    entry(
        "가나",
        "NNG",
        "Compound",
        "NNG",
        "NNG",
        "가/NNG/*+나/NNG/*",
        0,
    )
}

fn entry(
    surface: &str,
    pos: &str,
    analysis_type: &str,
    start_pos: &str,
    end_pos: &str,
    expression: &str,
    word_cost: i32,
) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost,
        analysis_type: analysis_type.to_owned(),
        start_pos: start_pos.to_owned(),
        end_pos: end_pos.to_owned(),
        expression: expression.to_owned(),
    }
}
