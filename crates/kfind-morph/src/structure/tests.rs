use kfind_data::{
    MecabSourceMorphologyEntry, decode_component_resource, encode_component_resource,
};
use proptest::prelude::*;

use super::*;
use crate::{CandidateTokenRelation, ComponentCapability, MorphContinuation};

#[test]
fn edge_graph_start_index_matches_a_linear_scan_at_every_byte() {
    let resolver = resolver();
    let text = "공부한";
    let graph = EdgeGraph::collect(resolver.resource(), text, 4_096).expect("bounded graph");

    for start in 0..=text.len() {
        let indexed = graph
            .starting_at(start)
            .iter()
            .map(|edge| (edge.span.clone(), edge.pos))
            .collect::<Vec<_>>();
        let linear = graph
            .edges()
            .iter()
            .filter(|edge| edge.span.start == start)
            .map(|edge| (edge.span.clone(), edge.pos))
            .collect::<Vec<_>>();
        assert_eq!(indexed, linear, "start={start}");
    }
}

#[test]
fn nominal_path_facts_match_direct_resource_traversal() {
    let resolver = resolver_from_entries(vec![
        atomic("나", "NNG"),
        atomic("나", "JX"),
        atomic("나나", "NNG"),
        atomic("나나", "JX"),
        atomic("나나나", "JX"),
        atomic("맨", "XPN"),
        atomic("맨나", "XPN+NNG"),
    ]);

    for text in ["나나", "나나나", "맨나나"] {
        let graph = EdgeGraph::collect(resolver.resource(), text, 4_096).expect("bounded graph");
        let facts = CommonPathFacts::collect(text, &graph).nominal_paths(text);

        assert_eq!(
            facts.particle_hosts.as_ref(),
            nominal_particle_hosts(resolver.resource(), text)
        );
        assert_eq!(
            facts.complete_particle_host,
            complete_nominal_particle_host(resolver.resource(), text)
        );
    }
}

fn arbitrary_dense_nominal_resource() -> impl Strategy<Value = (String, Vec<(String, &'static str)>)>
{
    (1_usize..=12).prop_flat_map(|text_len| {
        let positions = [
            "NNG", "NNP", "JX", "JX+JC", "XPN", "XSN", "XR", "VV", "VV+EC", "VA+EP+EC", "EP", "EC",
            "EF", "ETM", "VX+EF", "XSV+ETM", "XPN+NNG",
        ];
        prop::collection::vec((1_usize..=text_len, 0_usize..positions.len()), 1..64).prop_map(
            move |entries| {
                (
                    "나".repeat(text_len),
                    entries
                        .into_iter()
                        .map(|(length, pos)| ("나".repeat(length), positions[pos]))
                        .collect(),
                )
            },
        )
    })
}

proptest! {
    #[test]
    fn nominal_path_facts_match_direct_resource_on_arbitrary_dense_prefixes(
        (text, entries) in arbitrary_dense_nominal_resource(),
    ) {
        let resolver = resolver_from_entries(
            entries
                .iter()
                .map(|(surface, pos)| atomic(surface, pos)),
        );
        let graph = EdgeGraph::collect(resolver.resource(), &text, 4_096).expect("bounded graph");
        let common = CommonPathFacts::collect(&text, &graph);
        let facts = common.nominal_paths(&text);
        let direct_hosts = nominal_particle_hosts(resolver.resource(), &text);

        prop_assert_eq!(facts.particle_hosts.as_ref(), direct_hosts.as_slice());
        prop_assert_eq!(
            facts.complete_particle_host,
            complete_nominal_particle_host(resolver.resource(), &text)
        );
        prop_assert_eq!(
            common.predicate_connective_boundaries,
            reference_predicate_connective_boundaries(text.len(), graph.edges()),
        );
        for start in text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(text.len()))
        {
            prop_assert_eq!(
                common.ending_suffix[start],
                complete_suffix(resolver.resource(), &text[start..], |pos| pos.starts_with('E')),
            );
            prop_assert_eq!(
                common.particle_suffix[start],
                complete_suffix(resolver.resource(), &text[start..], |pos| pos.starts_with('J')),
            );
            prop_assert_eq!(
                common.nominal_prefix[start].iter().any(|&reachable| reachable),
                complete_suffix(resolver.resource(), &text[..start], |pos| {
                    DataFinePos::parse(pos).is_some_and(DataFinePos::is_nominal)
                    || matches!(pos, "XPN" | "XSN" | "XR")
                }),
            );
            prop_assert_eq!(
                common.exact_nominal_end[start],
                has_exact_fine_pos(resolver.resource(), &text[..start], DataFinePos::is_nominal),
            );
        }
    }
}

fn reference_predicate_connective_boundaries(text_len: usize, edges: &[Edge<'_>]) -> Vec<bool> {
    let mut predicate_path = vec![false; text_len + 1];
    let mut connective_boundary = vec![false; text_len + 1];
    for edge in edges {
        let ends_in_connective = if edge.span.start == 0 {
            predicate_path_ends_in_connective(edge.pos)
        } else if predicate_path[edge.span.start] {
            ending_path_ends_in_connective(edge.pos)
        } else {
            None
        };
        if let Some(ends_in_connective) = ends_in_connective {
            if ends_in_connective {
                connective_boundary[edge.span.end] = true;
            } else {
                predicate_path[edge.span.end] = true;
            }
        }
    }
    connective_boundary
}

proptest! {
    #[test]
    fn edge_graph_start_index_matches_arbitrary_sorted_edges(
        text_len in 0_usize..=256,
        starts in prop::collection::vec(0_usize..256, 0..512),
    ) {
        let mut starts = starts
            .into_iter()
            .filter(|start| *start < text_len)
            .collect::<Vec<_>>();
        starts.sort_unstable();
        let edges = starts
            .into_iter()
            .map(|start| Edge {
                span: start..start + 1,
                pos: "NNG",
                components: Vec::new(),
            })
            .collect::<Vec<_>>();
        let graph = EdgeGraph::from_edges(text_len, edges);

        for start in 0..=text_len {
            let indexed = graph
                .starting_at(start)
                .iter()
                .map(|edge| edge.span.clone())
                .collect::<Vec<_>>();
            let linear = graph
                .edges()
                .iter()
                .filter(|edge| edge.span.start == start)
                .map(|edge| edge.span.clone())
                .collect::<Vec<_>>();
            prop_assert_eq!(indexed, linear);
        }
        prop_assert!(graph.starting_at(usize::MAX).is_empty());
    }
}

fn arbitrary_unit_graph() -> impl Strategy<Value = (usize, Vec<Unit>, Range<usize>)> {
    (1_usize..=64).prop_flat_map(|text_len| {
        (
            Just(text_len),
            prop::collection::vec(
                (
                    0_usize..text_len,
                    1_usize..=text_len,
                    any::<bool>(),
                    any::<bool>(),
                ),
                0..256,
            ),
            0_usize..text_len,
            1_usize..=text_len,
        )
            .prop_map(|(text_len, raw_units, core_start, core_end)| {
                let mut units = raw_units
                    .into_iter()
                    .filter(|(start, end, _, _)| start < end)
                    .map(|(start, end, nominal, source)| Unit {
                        span: start..end,
                        pos: if nominal {
                            DataFinePos::Nng
                        } else {
                            DataFinePos::Vv
                        },
                        evidence: if source {
                            StructuralEvidence::SourceComponent
                        } else {
                            StructuralEvidence::RuntimeComponent
                        },
                        from_whole_nominal: false,
                    })
                    .collect::<Vec<_>>();
                units.sort_unstable_by_key(|unit| {
                    (
                        unit.span.start,
                        unit.span.end,
                        unit.pos,
                        unit.evidence as u8,
                    )
                });
                let core_start = core_start.min(text_len - 1);
                let core_end = core_end.max(core_start + 1).min(text_len);
                (text_len, units, core_start..core_end)
            })
    })
}

proptest! {
    #[test]
    fn indexed_unit_paths_match_linear_reference(
        (text_len, units, core) in arbitrary_unit_graph(),
    ) {
        let graph = UnitGraph::from_sorted_by(text_len, units, |unit| unit.span.start);
        let selected = 0..text_len;

        prop_assert_eq!(
            graph.minimum_path_len(&selected, DataFinePos::is_nominal),
            linear_minimum_path_len(&graph, &selected, DataFinePos::is_nominal),
        );
        prop_assert_eq!(
            graph.contains_on_preferred_path(&core, &selected, |_| true),
            linear_preferred_path_contains(&graph, &core, &selected),
        );
    }
}

fn linear_minimum_path_len(
    graph: &UnitGraph,
    span: &Range<usize>,
    accepts: impl Fn(DataFinePos) -> bool,
) -> Option<usize> {
    let mut costs = vec![None; span.len() + 1];
    costs[0] = Some(0_usize);
    for offset in 0..span.len() {
        let Some(cost) = costs[offset] else {
            continue;
        };
        let start = span.start + offset;
        for unit in graph.all().iter().filter(|unit| {
            unit.span.start == start && unit.span.end <= span.end && accepts(unit.pos)
        }) {
            let end = unit.span.end - span.start;
            let candidate = cost + 1;
            if costs[end].is_none_or(|current| candidate < current) {
                costs[end] = Some(candidate);
            }
        }
    }
    costs[span.len()]
}

fn linear_preferred_path_contains(
    graph: &UnitGraph,
    core: &Range<usize>,
    selected: &Range<usize>,
) -> bool {
    let mut edges = graph
        .all()
        .iter()
        .filter(|unit| unit.span.end <= selected.end)
        .map(|unit| {
            (
                unit.span.start,
                unit.span.end,
                UnitPathCost::default().append(unit),
            )
        })
        .collect::<Vec<_>>();
    edges.sort_unstable_by_key(|(start, end, cost)| (*start, *end, *cost));
    edges.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);

    let mut prefix = vec![None; selected.len() + 1];
    prefix[0] = Some(UnitPathCost::default());
    for offset in 0..selected.len() {
        let Some(cost) = prefix[offset] else {
            continue;
        };
        let start = selected.start + offset;
        for (_, end, edge_cost) in edges
            .iter()
            .filter(|(edge_start, _, _)| *edge_start == start)
        {
            let end = *end - selected.start;
            let candidate = cost.combine(*edge_cost);
            if prefix[end].is_none_or(|current| candidate < current) {
                prefix[end] = Some(candidate);
            }
        }
    }
    let Some(best) = prefix[selected.len()] else {
        return false;
    };

    let mut suffix = vec![None; selected.len() + 1];
    suffix[selected.len()] = Some(UnitPathCost::default());
    for offset in (1..=selected.len()).rev() {
        let Some(cost) = suffix[offset] else {
            continue;
        };
        let end = selected.start + offset;
        for (start, _, edge_cost) in edges.iter().filter(|(_, edge_end, _)| *edge_end == end) {
            let start = *start - selected.start;
            let candidate = edge_cost.combine(cost);
            if suffix[start].is_none_or(|current| candidate < current) {
                suffix[start] = Some(candidate);
            }
        }
    }

    let core_start = core.start - selected.start;
    let core_end = core.end - selected.start;
    edges.iter().any(|(start, end, edge_cost)| {
        if *start != core.start || *end != core.end {
            return false;
        }
        let (Some(prefix), Some(suffix)) = (prefix[core_start], suffix[core_end]) else {
            return false;
        };
        prefix.combine(*edge_cost).combine(suffix) == best
    })
}

#[test]
fn exact_pronoun_copula_ending_path_requires_the_complete_source_sequence() {
    let supported = resolver_from_entries([
        expression("누군가", "NP+VCP+EF", "누구/NP/*+이/VCP/*+ᆫ가/EF/*"),
        atomic("무언가", "NP+VCP+EC"),
    ]);
    assert!(supported.has_exact_pronoun_copula_ending_path("누군가"));
    assert!(supported.has_exact_pronoun_copula_ending_path("무언가"));

    for pos in ["NP", "NP+EF", "NP+VCP", "NNG+VCP+EF", "NP+VCP+ETM"] {
        let resolver = resolver_from_entries([atomic("누군가", pos)]);
        assert!(
            !resolver.has_exact_pronoun_copula_ending_path("누군가"),
            "unexpectedly accepted {pos}"
        );
    }
}

#[test]
fn lost_span_copula_ending_requires_an_unaligned_complete_source_sequence() {
    let resolver = resolver_from_entries([
        expression("걸까", "NNB+VCP+EF", "것/NNB/*+이/VCP/*+ᆯ까/EF/*"),
        expression("것이까", "NNB+VCP+EF", "것/NNB/*+이/VCP/*+까/EF/*"),
    ]);

    assert!(resolver.has_exact_lost_span_copula_ending_path("걸까"));
    assert!(!resolver.has_exact_lost_span_copula_ending_path("것이까"));

    for pos in [
        "NNB+EF",
        "NNB+VCP",
        "NNB+VCP+ETM",
        "NNB+VCP+EC+VCP",
        "VV+VCP+EF",
    ] {
        let resolver = resolver_from_entries([atomic("걸까", pos)]);
        assert!(
            !resolver.has_exact_lost_span_copula_ending_path("걸까"),
            "unexpectedly accepted {pos}"
        );
    }
}

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
fn runtime_path_does_not_join_an_adverb_to_an_attached_predicate() {
    let resolver = resolver_from_entries([
        atomic("못", "MAG"),
        atomic("못", "NNG"),
        atomic("못했", "VA"),
        atomic("했다", "NNG"),
        atomic("다", "EF"),
    ]);
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("못했다"),
        CandidateSpans {
            core: 0.."못".len(),
            anchor: 0.."못".len(),
            consumed: 0.."못".len(),
            token: 0.."못했다".len(),
        },
        &[component_pattern(DataFinePos::Mag, "못")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn noun_derivation_requires_source_aligned_components() {
    let resolver = resolver_from_entries([
        atomic("못", "MAG"),
        atomic("못", "NNG"),
        atomic("공부", "NNG"),
        atomic("하", "XSV"),
        atomic("하다", "VV"),
        atomic("다", "EF"),
        expression("못하다", "VA+EF", "못하/VA/*+다/EF/*"),
        expression("못하다", "NNG+XSA+EF", "못/NNG/*+하/XSA/*+다/EF/*"),
        expression("공부하다", "NNG+XSV+EF", "공부/NNG/*+하/XSV/*+다/EF/*"),
    ]);
    let unsupported = resolver.resolve_candidate(
        BoundedTokenContext::current("못하다"),
        CandidateSpans {
            core: 0.."못".len(),
            anchor: 0.."못".len(),
            consumed: 0.."못".len(),
            token: 0.."못하다".len(),
        },
        &[
            component_pattern(DataFinePos::Nng, "못"),
            component_pattern(DataFinePos::Nnp, "못"),
            component_pattern(DataFinePos::Nnb, "못"),
        ],
        128,
    );
    let supported = resolver.resolve_candidate(
        BoundedTokenContext::current("공부하다"),
        CandidateSpans {
            core: 0.."공부".len(),
            anchor: 0.."공부".len(),
            consumed: 0.."공부".len(),
            token: 0.."공부하다".len(),
        },
        &[component_pattern(DataFinePos::Nng, "공부")],
        128,
    );

    assert_eq!(unsupported.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&unsupported));
    assert_eq!(supported.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&supported));
    assert!(
        supported
            .supported
            .iter()
            .any(|support| support.evidence == StructuralEvidence::SourceComponent)
    );
}

#[test]
fn multisyllable_runtime_nominal_derivation_survives_a_whole_predicate() {
    let resolver = resolver_from_entries([
        atomic("재미", "NNG"),
        atomic("있", "VA"),
        atomic("어요", "EF"),
        atomic("재미있", "VA"),
    ]);
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("재미있어요"),
        CandidateSpans {
            core: 0.."재미".len(),
            anchor: 0.."재미".len(),
            consumed: 0.."재미".len(),
            token: 0.."재미있어요".len(),
        },
        &[component_pattern(DataFinePos::Nng, "재미")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn derived_nominal_prefix_requires_a_complete_derivational_predicate() {
    let resolver = resolver_from_entries([
        atomic("잠식", "NNG"),
        atomic("당", "XSN"),
        atomic("당하", "XSV"),
        atomic("하", "XSV"),
        atomic("기", "ETN"),
    ]);
    let core = 0.."잠식당".len();
    let supported = resolver.resolve_candidate(
        BoundedTokenContext::current("잠식당하기"),
        CandidateSpans {
            core: core.clone(),
            anchor: core.clone(),
            consumed: core,
            token: 0.."잠식당하기".len(),
        },
        &[nominal_pattern(DataFinePos::Nng, "잠식당")],
        128,
    );
    let internal = resolver.resolve_candidate(
        BoundedTokenContext::current("잠식당하기"),
        CandidateSpans {
            core: "잠".len().."잠식당".len(),
            anchor: "잠".len().."잠식당".len(),
            consumed: "잠".len().."잠식당".len(),
            token: 0.."잠식당하기".len(),
        },
        &[nominal_pattern(DataFinePos::Nng, "식당")],
        128,
    );
    let incomplete = resolver.resolve_candidate(
        BoundedTokenContext::current("잠식당하"),
        CandidateSpans {
            core: 0.."잠식당".len(),
            anchor: 0.."잠식당".len(),
            consumed: 0.."잠식당".len(),
            token: 0.."잠식당하".len(),
        },
        &[nominal_pattern(DataFinePos::Nng, "잠식당")],
        128,
    );

    assert_eq!(supported.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&supported));
    assert_eq!(internal.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(incomplete.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn runtime_path_does_not_join_a_noun_to_an_attached_predicate() {
    let resolver = resolver_from_entries([
        atomic("못", "NNG"),
        atomic("못하", "VA"),
        atomic("하다", "VV"),
        atomic("하다", "NNG"),
        atomic("하", "XSV"),
        atomic("하", "JKV"),
        atomic("다", "EF"),
        atomic("다", "JX"),
    ]);
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("못하다"),
        CandidateSpans {
            core: 0.."못".len(),
            anchor: 0.."못".len(),
            consumed: 0.."못".len(),
            token: 0.."못하다".len(),
        },
        &[component_pattern(DataFinePos::Nng, "못")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn one_syllable_nominal_particle_path_survives_a_whole_predicate() {
    let resolver = resolver_from_entries([
        atomic("벼", "NNG"),
        atomic("를", "JKO"),
        atomic("벼를", "VV+ETM"),
    ]);
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("벼를"),
        CandidateSpans {
            core: 0.."벼".len(),
            anchor: 0.."벼".len(),
            consumed: 0.."벼를".len(),
            token: 0.."벼를".len(),
        },
        &[component_pattern(DataFinePos::Nng, "벼")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn complete_ha_predicate_path_selects_an_adjacent_adverb() {
    let resolver = resolver_from_entries([
        atomic("못", "MAG"),
        atomic("못", "NNG"),
        atomic("하", "VV"),
        atomic("했", "VV+EP"),
        atomic("박", "VV"),
        atomic("이브리드", "NNG"),
        atomic("겠", "EP"),
        atomic("았", "EP"),
        atomic("어요", "EF"),
    ]);
    let ha_context = BoundedTokenContext {
        previous: None,
        current: "못",
        next: Some("하겠어요"),
    };
    let other_predicate_context = BoundedTokenContext {
        next: Some("박았어요"),
        ..ha_context
    };

    let noun = resolver.resolve_candidate(
        ha_context,
        spans(0.."못".len(), 0.."못".len()),
        &[whole_pattern(DataFinePos::Nng, "못")],
        128,
    );
    let adverb = resolver.resolve_candidate(
        ha_context,
        spans(0.."못".len(), 0.."못".len()),
        &[whole_pattern(DataFinePos::Mag, "못")],
        128,
    );
    let noun_before_other_predicate = resolver.resolve_candidate(
        other_predicate_context,
        spans(0.."못".len(), 0.."못".len()),
        &[whole_pattern(DataFinePos::Nng, "못")],
        128,
    );

    assert_eq!(noun.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(adverb.outcome, ConstraintOutcome::Supported);
    assert_eq!(
        noun_before_other_predicate.outcome,
        ConstraintOutcome::Supported
    );

    let past_context = BoundedTokenContext {
        previous: None,
        current: "못",
        next: Some("했어요"),
    };
    let noun_before_past = resolver.resolve_candidate(
        past_context,
        spans(0.."못".len(), 0.."못".len()),
        &[whole_pattern(DataFinePos::Nng, "못")],
        128,
    );
    assert_eq!(noun_before_past.outcome, ConstraintOutcome::Contradicted);
    assert!(!complete_ha_predicate_path(
        resolver.resource(),
        "하이브리드"
    ));
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
        std::slice::from_ref(&pattern),
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
}

#[test]
fn source_aligned_compound_predicate_tails_survive_competing_whole_paths() {
    let resolver = resolver_from_entries([
        atomic("올라가", "VV"),
        atomic("올라", "VV+EC"),
        atomic("가", "VV"),
        atomic("생겨나", "VV"),
        atomic("생겨", "VV+EC"),
        atomic("나", "VV"),
        atomic("들어와서", "NNG"),
        atomic("들어", "VV+EC"),
        atomic("와", "VV"),
        atomic("서", "EC"),
        atomic("는", "JX"),
        atomic("끌어가", "VV"),
        expression("끌어가", "VV+EC+VX", "끌/VV/*+어/EC/*+가/VX/*"),
    ]);
    for (text, core, lexical_form, pos) in [
        (
            "올라가",
            "올라".len().."올라가".len(),
            "가",
            DataFinePos::Vv,
        ),
        (
            "생겨나",
            "생겨".len().."생겨나".len(),
            "나",
            DataFinePos::Vv,
        ),
        (
            "들어와서는",
            "들어".len().."들어와".len(),
            "오",
            DataFinePos::Vv,
        ),
        (
            "끌어가",
            "끌어".len().."끌어가".len(),
            "가",
            DataFinePos::Vx,
        ),
    ] {
        let consumed = core.start..text.len();
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                anchor: core.clone(),
                core,
                consumed,
                token: 0..text.len(),
            },
            &[predicate_pattern(pos, lexical_form)],
            128,
        );

        assert_eq!(decision.outcome, ConstraintOutcome::Supported, "{text}");
    }
}

#[test]
fn compound_predicate_tail_requires_a_complete_typed_path() {
    let pattern = predicate_pattern(DataFinePos::Vv, "가");
    for entries in [
        [
            atomic("올라", "NNG+EC"),
            atomic("가", "VV"),
            atomic("말", "NNG"),
        ],
        [
            atomic("올라", "VV+EC"),
            atomic("가", "VV"),
            atomic("말", "NNG"),
        ],
    ] {
        let resolver = resolver_from_entries(entries);
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current("올라가말"),
            CandidateSpans {
                core: "올라".len().."올라가".len(),
                anchor: "올라".len().."올라가".len(),
                consumed: "올라".len().."올라가".len(),
                token: 0.."올라가말".len(),
            },
            std::slice::from_ref(&pattern),
            128,
        );

        assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    }

    let resolver = resolver_from_entries([
        atomic("그러나", "MAJ"),
        atomic("그러", "VV+EC"),
        atomic("나", "VV+EF"),
    ]);
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("그러나"),
        CandidateSpans {
            core: "그러".len().."그러나".len(),
            anchor: "그러".len().."그러나".len(),
            consumed: "그러".len().."그러나".len(),
            token: 0.."그러나".len(),
        },
        std::slice::from_ref(&pattern),
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);

    let resolver = resolver_from_entries([
        atomic("친", "VV+ETM"),
        atomic("구", "EC"),
        atomic("가", "VV"),
        atomic("친구", "NNG"),
        atomic("가", "JKS"),
    ]);
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("친구가"),
        CandidateSpans {
            core: "친구".len().."친구가".len(),
            anchor: "친구".len().."친구가".len(),
            consumed: "친구".len().."친구가".len(),
            token: 0.."친구가".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
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
fn copula_components_follow_complete_nominal_hosts() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Vcp, "이").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    for (text, core, anchor, consumed) in [
        ("동안이었습니다", 6..9, 6..21, 6..21),
        ("끝인가", 3..6, 3..6, 3..9),
        ("곳인", 3..6, 3..6, 3..6),
        ("공학입니다", 6..9, 6..15, 6..15),
    ] {
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core,
                anchor,
                consumed,
                token: 0..text.len(),
            },
            std::slice::from_ref(&pattern),
            128,
        );

        assert_eq!(decision.outcome, ConstraintOutcome::Supported, "{text}");
        assert!(ProductPolicy::RecallFirst.accepts(&decision), "{text}");
    }
}

#[test]
fn nominal_hosts_before_complete_copula_suffixes_are_supported() {
    let resolver = resolver();
    for (text, host, pos) in [
        ("결과이다", "결과", DataFinePos::Nng),
        ("왕친입니다", "왕친", DataFinePos::Nnp),
        ("고체이긴", "고체", DataFinePos::Nng),
        ("것이었다", "것", DataFinePos::Nnb),
        ("바튼반도이다", "바튼반도", DataFinePos::Nnp),
    ] {
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: 0..host.len(),
                anchor: 0..host.len(),
                consumed: 0..host.len(),
                token: 0..text.len(),
            },
            &[nominal_pattern(pos, host)],
            128,
        );

        assert_eq!(decision.outcome, ConstraintOutcome::Supported, "{text}");
        assert!(ProductPolicy::RecallFirst.accepts(&decision), "{text}");
    }
}

#[test]
fn nominal_copula_hosts_do_not_skip_or_overlap_the_copula() {
    let resolver = resolver();
    for (text, host, pos) in [
        ("홍씨이다", "홍", DataFinePos::Nnp),
        ("맛있다", "맛", DataFinePos::Nng),
        ("이다", "이", DataFinePos::Nng),
        ("반는", "반", DataFinePos::Nng),
    ] {
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: 0..host.len(),
                anchor: 0..host.len(),
                consumed: 0..host.len(),
                token: 0..text.len(),
            },
            &[nominal_pattern(pos, host)],
            128,
        );

        assert_eq!(decision.outcome, ConstraintOutcome::Contradicted, "{text}");
        assert!(!ProductPolicy::RecallFirst.accepts(&decision), "{text}");
    }
}

#[test]
fn whole_adverb_still_rejects_a_copula_suffix() {
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
        BoundedTokenContext {
            previous: None,
            current: "매일",
            next: Some("보고"),
        },
        CandidateSpans {
            core: 3..6,
            anchor: 3..6,
            consumed: 3..6,
            token: 0..6,
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&decision));
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
fn competing_nominal_particle_hosts_are_all_preserved() {
    let resolver = resolver_from_entries([
        atomic("후", "NNG"),
        atomic("후에", "NNP"),
        atomic("에", "JKB"),
        atomic("도", "JX"),
    ]);
    let shorter = resolver.resolve_candidate(
        BoundedTokenContext::current("후에도"),
        spans(0.."후".len(), 0.."후에도".len()),
        &[nominal_pattern(DataFinePos::Nng, "후")],
        128,
    );
    let longer = resolver.resolve_candidate(
        BoundedTokenContext::current("후에도"),
        spans(0.."후에".len(), 0.."후에도".len()),
        &[nominal_pattern(DataFinePos::Nnp, "후에")],
        128,
    );

    assert_eq!(shorter.outcome, ConstraintOutcome::Supported);
    assert_eq!(longer.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&shorter));
    assert!(ProductPolicy::RecallFirst.accepts(&longer));
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
fn whole_nominal_source_component_outranks_a_shorter_particle_host() {
    let resolver = resolver();
    let core = "자본".len().."자본주의".len();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("자본주의"),
        CandidateSpans {
            core: core.clone(),
            anchor: core.clone(),
            consumed: core,
            token: 0.."자본주의".len(),
        },
        &[component_pattern(DataFinePos::Nng, "주의")],
        128,
    );
    let crossing = resolver.resolve_candidate(
        BoundedTokenContext::current("자본주의"),
        CandidateSpans {
            core: "자".len().."자본주".len(),
            anchor: "자".len().."자본주".len(),
            consumed: "자".len().."자본주".len(),
            token: 0.."자본주의".len(),
        },
        &[component_pattern(DataFinePos::Nng, "본주")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(
        decision
            .supported
            .iter()
            .any(|support| support.evidence == StructuralEvidence::SourceComponent)
    );
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
    assert_eq!(crossing.outcome, ConstraintOutcome::Contradicted);
    assert!(!ProductPolicy::RecallFirst.accepts(&crossing));
}

#[test]
fn modifier_led_nominal_path_preserves_exact_tail_components() {
    let resolver = resolver_from_entries([
        atomic("어", "VV"),
        atomic("느", "NNG"),
        atomic("어느", "MM"),
        atomic("어느", "NP"),
        atomic("날", "NNG"),
        atomic("날", "JKO"),
        atomic("매", "MM"),
        atomic("매", "NNG"),
        atomic("일", "NNG"),
        atomic("일", "JKO"),
        atomic("매일", "MAG"),
        atomic("아무", "MM"),
        atomic("아무", "NP"),
        atomic("나", "NP"),
        atomic("나", "JKO"),
        atomic("칠", "MM"),
        atomic("칠", "NR"),
        atomic("월", "NNG"),
        atomic("월", "NNBC"),
        atomic("소", "MM"),
        atomic("소", "NNG"),
        atomic("년", "NNG"),
        atomic("년", "NNB"),
        atomic("은", "JX"),
    ]);
    let day = "어느".len().."어느날".len();
    let day_decision = resolver.resolve_candidate(
        BoundedTokenContext::current("어느날"),
        CandidateSpans {
            core: day.clone(),
            anchor: day.clone(),
            consumed: day,
            token: 0.."어느날".len(),
        },
        &[component_pattern(DataFinePos::Nng, "날")],
        128,
    );
    let every_day = "매".len().."매일".len();
    let every_day_decision = resolver.resolve_candidate(
        BoundedTokenContext::current("매일"),
        CandidateSpans {
            core: every_day.clone(),
            anchor: every_day.clone(),
            consumed: every_day,
            token: 0.."매일".len(),
        },
        &[component_pattern(DataFinePos::Nng, "일")],
        128,
    );
    let anyone = "아무".len().."아무나".len();
    let anyone_decision = resolver.resolve_candidate(
        BoundedTokenContext::current("아무나"),
        CandidateSpans {
            core: anyone.clone(),
            anchor: anyone.clone(),
            consumed: anyone,
            token: 0.."아무나".len(),
        },
        &[component_pattern(DataFinePos::Np, "나")],
        128,
    );
    let month = "칠".len().."칠월".len();
    let month_decision = resolver.resolve_candidate(
        BoundedTokenContext::current("칠월"),
        CandidateSpans {
            core: month.clone(),
            anchor: month.clone(),
            consumed: month,
            token: 0.."칠월".len(),
        },
        &[component_pattern(DataFinePos::Nng, "월")],
        128,
    );
    let year = "소".len().."소년".len();
    let boy_decision = resolver.resolve_candidate(
        BoundedTokenContext::current("소년은"),
        CandidateSpans {
            core: year.clone(),
            anchor: year.clone(),
            consumed: "소".len().."소년은".len(),
            token: 0.."소년은".len(),
        },
        &[component_pattern(DataFinePos::Nng, "년")],
        128,
    );

    assert_eq!(day_decision.outcome, ConstraintOutcome::Supported);
    assert_eq!(every_day_decision.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(anyone_decision.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(month_decision.outcome, ConstraintOutcome::Supported);
    assert_eq!(boy_decision.outcome, ConstraintOutcome::Contradicted);
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
fn complete_graph_nominal_host_consumes_its_particle() {
    let resolver = resolver();
    let host = 0.."선거운동".len();
    let pattern = nominal_pattern(DataFinePos::Nng, "선거운동");
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("선거운동과"),
        CandidateSpans {
            core: host.clone(),
            anchor: host.clone(),
            consumed: 0.."선거운동과".len(),
            token: 0.."선거운동과".len(),
        },
        std::slice::from_ref(&pattern),
        128,
    );
    let incomplete = resolver.resolve_candidate(
        BoundedTokenContext::current("선거운동과"),
        CandidateSpans {
            core: host.clone(),
            anchor: host.clone(),
            consumed: host,
            token: 0.."선거운동과".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&decision));
    assert_eq!(incomplete.outcome, ConstraintOutcome::Contradicted);
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
fn attached_nominal_suffix_requires_a_complete_nominal_particle_path() {
    let resolver = resolver_from_entries([
        atomic("책임", "NNG"),
        atomic("하", "NNG"),
        atomic("하", "JKV"),
        atomic("에서", "JKB"),
        atomic("날", "NNG"),
        atomic("씨", "NNG"),
        atomic("날씨", "NNG"),
        atomic("는", "JX"),
        atomic("빙원", "NNG"),
        atomic("옆", "NNG"),
        atomic("에", "JKB"),
    ]);
    let target_start = "책임".len();
    let target_end = "책임하".len();
    let supported = resolver.resolve_candidate(
        BoundedTokenContext::current("책임하에서"),
        CandidateSpans {
            core: target_start..target_end,
            anchor: target_start..target_end,
            consumed: target_start.."책임하에서".len(),
            token: 0.."책임하에서".len(),
        },
        &[nominal_pattern(DataFinePos::Nng, "하")],
        128,
    );
    let lexicalized_start = "날".len();
    let lexicalized_end = "날씨".len();
    let lexicalized = resolver.resolve_candidate(
        BoundedTokenContext::current("날씨는"),
        CandidateSpans {
            core: lexicalized_start..lexicalized_end,
            anchor: lexicalized_start..lexicalized_end,
            consumed: lexicalized_start.."날씨는".len(),
            token: 0.."날씨는".len(),
        },
        &[nominal_pattern(DataFinePos::Nng, "씨")],
        128,
    );
    let without_particle = resolver.resolve_candidate(
        BoundedTokenContext::current("책임하"),
        CandidateSpans {
            core: target_start..target_end,
            anchor: target_start..target_end,
            consumed: target_start..target_end,
            token: 0..target_end,
        },
        &[nominal_pattern(DataFinePos::Nng, "하")],
        128,
    );
    let independent_start = "빙원".len();
    let independent_end = "빙원옆".len();
    let independent = resolver.resolve_candidate(
        BoundedTokenContext::current("빙원옆에"),
        CandidateSpans {
            core: independent_start..independent_end,
            anchor: independent_start..independent_end,
            consumed: independent_start.."빙원옆에".len(),
            token: 0.."빙원옆에".len(),
        },
        &[nominal_pattern(DataFinePos::Nng, "옆")],
        128,
    );

    assert_eq!(supported.outcome, ConstraintOutcome::Supported);
    assert!(ProductPolicy::RecallFirst.accepts(&supported));
    assert_eq!(lexicalized.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(without_particle.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(independent.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn ascii_number_supports_only_an_aligned_numeric_unit() {
    let resolver = resolver();
    let year_start = "2014".len();
    let year = resolver.resolve_candidate(
        BoundedTokenContext::current("2014년"),
        CandidateSpans {
            core: year_start.."2014년".len(),
            anchor: year_start.."2014년".len(),
            consumed: year_start.."2014년".len(),
            token: 0.."2014년".len(),
        },
        &[component_pattern(DataFinePos::Nnb, "년")],
        128,
    );
    let thousand_start = "4".len();
    let thousand = resolver.resolve_candidate(
        BoundedTokenContext::current("4천"),
        CandidateSpans {
            core: thousand_start.."4천".len(),
            anchor: thousand_start.."4천".len(),
            consumed: thousand_start.."4천".len(),
            token: 0.."4천".len(),
        },
        &[component_pattern(DataFinePos::Nr, "천")],
        128,
    );

    assert_eq!(year.outcome, ConstraintOutcome::Supported);
    assert_eq!(thousand.outcome, ConstraintOutcome::Supported);
}

#[test]
fn ascii_numeric_unit_consumes_a_complete_particle_chain() {
    let resolver = resolver();
    let unit_start = "197".len();
    let unit_end = "197명".len();
    let pattern = QueryMorphPattern::new(DataFinePos::Nnb, "명").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("197명이"),
        CandidateSpans {
            core: unit_start..unit_end,
            anchor: unit_start..unit_end,
            consumed: unit_start.."197명이".len(),
            token: 0.."197명이".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
}

#[test]
fn ascii_numeric_unit_keeps_an_exact_dependent_noun_tail() {
    let resolver = resolver();
    let unit = "1".len().."1년".len();
    let tail = "1년".len().."1년간".len();
    for text in ["1년간", "1년간의"] {
        let tail_decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: tail.clone(),
                anchor: tail.clone(),
                consumed: tail.start..text.len(),
                token: 0..text.len(),
            },
            &[nominal_pattern(DataFinePos::Nnb, "간")],
            128,
        );
        let unit_decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                core: unit.clone(),
                anchor: unit.clone(),
                consumed: unit.clone(),
                token: 0..text.len(),
            },
            &[component_pattern(DataFinePos::Nnb, "년")],
            128,
        );

        assert_eq!(
            tail_decision.outcome,
            ConstraintOutcome::Supported,
            "{text}"
        );
        assert_eq!(
            unit_decision.outcome,
            ConstraintOutcome::Supported,
            "{text}"
        );
    }
}

#[test]
fn ascii_numeric_unit_prefers_a_longer_complete_unit_over_a_tail_split() {
    let resolver = resolver_from_entries([
        atomic("시", "NNBC"),
        atomic("간", "NNB"),
        atomic("시간", "NNBC"),
    ]);
    let unit = "10".len().."10시간".len();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("10시간"),
        CandidateSpans {
            core: unit.clone(),
            anchor: unit.clone(),
            consumed: unit,
            token: 0.."10시간".len(),
        },
        &[component_pattern(DataFinePos::Nnb, "시간")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
}

#[test]
fn ascii_numeric_unit_rejects_an_ordinary_noun_tail() {
    let resolver = resolver();
    let core = "197명".len().."197명사".len();
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("197명사"),
        CandidateSpans {
            core: core.clone(),
            anchor: core.clone(),
            consumed: core,
            token: 0.."197명사".len(),
        },
        &[component_pattern(DataFinePos::Nng, "사")],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn numeric_unit_rule_rejects_nonnumeric_and_nonparticle_neighbors() {
    let resolver = resolver();

    for text in ["소년", "추천", "익명이", "197명사"] {
        assert!(
            numeric_unit_path(resolver.resource(), text).is_none(),
            "{text}"
        );
    }
}

#[test]
fn hangul_numeral_sequences_support_only_aligned_numerals() {
    let resolver = resolver();
    let cases = [
        ("수십만의", "수십".len().."수십만".len(), "만"),
        ("십일월에", "십".len().."십일".len(), "일"),
        ("백명", 0.."백".len(), "백"),
    ];

    for (text, core, query) in cases {
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                anchor: core.clone(),
                core: core.clone(),
                consumed: core,
                token: 0..text.len(),
            },
            &[component_pattern(DataFinePos::Nr, query)],
            128,
        );
        assert_eq!(decision.outcome, ConstraintOutcome::Supported, "{text}");
    }
}

#[test]
fn ascii_prefixed_numeral_sequences_require_a_dependent_unit() {
    let resolver = resolver();
    let cases = [
        ("5천톤의", "5".len().."5천".len(), "천"),
        ("6백미터", "6".len().."6백".len(), "백"),
    ];

    for (text, core, query) in cases {
        let decision = resolver.resolve_candidate(
            BoundedTokenContext::current(text),
            CandidateSpans {
                anchor: core.clone(),
                core: core.clone(),
                consumed: core,
                token: 0..text.len(),
            },
            &[component_pattern(DataFinePos::Nr, query)],
            128,
        );
        assert_eq!(decision.outcome, ConstraintOutcome::Supported, "{text}");
    }

    for text in ["3천사", "197명사", "5천톤사"] {
        let graph = EdgeGraph::collect(resolver.resource(), text, 4_096).expect("bounded graph");
        let numeric_end = text.bytes().take_while(u8::is_ascii_digit).count();
        assert!(
            numeral_sequence_spans(text.len(), numeric_end, &graph, true).is_empty(),
            "{text}"
        );
    }
}

#[test]
fn hangul_numeral_sequences_reject_an_ordinary_noun_tail() {
    let resolver = resolver();
    for text in ["백명사전", "일월산맥길"] {
        let graph = EdgeGraph::collect(resolver.resource(), text, 4_096).expect("bounded graph");
        assert!(
            hangul_numeral_spans(text.len(), &graph).is_empty(),
            "{text}"
        );
    }
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
fn predicate_nominalization_keeps_same_syllable_stem_composition() {
    let resolver = resolver_from_entries([
        atomic("봄", "NNG"),
        atomic("으로써", "JKB"),
        atomic("봄으로써", "NNG+JKB"),
    ]);
    let pattern = QueryMorphPattern::new(DataFinePos::Vv, "보").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: true,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let decision = resolver.resolve_candidate(
        BoundedTokenContext::current("봄으로써"),
        CandidateSpans {
            core: 0.."봄".len(),
            anchor: 0.."봄".len(),
            consumed: 0.."봄으로써".len(),
            token: 0.."봄으로써".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(decision.outcome, ConstraintOutcome::Supported);
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
    assert!(resolver.has_whole_modifier("어떤가"));
    assert!(resolver.supports_predicate_ending_path(
        "어떤가",
        "어떤".len(),
        crate::PredicatePos::Adjective,
        128,
    ));
}

#[test]
fn predicate_ending_particle_path_requires_endings_before_particles() {
    let resolver = resolver();

    assert!(resolver.supports_predicate_ending_particle_path(
        "위해서는",
        "위해".len(),
        "위해서".len(),
        crate::PredicatePos::Verb,
        128,
    ));
    assert!(!resolver.supports_predicate_ending_particle_path(
        "위해서",
        "위해".len(),
        "위해서".len(),
        crate::PredicatePos::Verb,
        128,
    ));
    assert!(!resolver.supports_predicate_ending_particle_path(
        "위해서는더니",
        "위해".len(),
        "위해서".len(),
        crate::PredicatePos::Verb,
        128,
    ));
}

#[test]
fn ambiguous_incomplete_particle_suffix_uses_bounded_reachability() {
    let repetitions = 64;
    let mut entries = vec![atomic("가", "VV"), atomic("다", "EF")];
    let mut surface = String::new();
    for _ in 0..repetitions {
        surface.push('나');
        entries.push(atomic(&surface, "JX"));
    }
    let resolver = resolver_from_entries(entries);
    let text = format!("가다{}끝", "나".repeat(repetitions));

    assert!(!resolver.supports_predicate_ending_particle_path(
        &text,
        "가".len(),
        "가다".len(),
        crate::PredicatePos::Verb,
        4_096,
    ));
}

#[test]
fn adnominal_dependent_noun_particle_path_requires_each_typed_segment() {
    let resolver = resolver();

    assert!(resolver.supports_adnominal_dependent_noun_particle_path(
        "온지를",
        "온".len(),
        "온".len(),
        crate::PredicatePos::Verb,
        128,
    ));
    assert!(!resolver.supports_adnominal_dependent_noun_particle_path(
        "온지",
        "온".len(),
        "온".len(),
        crate::PredicatePos::Verb,
        128,
    ));
    assert!(!resolver.supports_adnominal_dependent_noun_particle_path(
        "온를",
        "온".len(),
        "온".len(),
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
fn attached_auxiliary_requires_a_predicate_connective_path() {
    let resolver = resolver();
    let pattern = QueryMorphPattern::new(DataFinePos::Vx, "지").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let supported = resolver.resolve_candidate(
        BoundedTokenContext::current("길어진"),
        CandidateSpans {
            core: "길어".len().."길어진".len(),
            anchor: "길어".len().."길어진".len(),
            consumed: "길어".len().."길어진".len(),
            token: 0.."길어진".len(),
        },
        std::slice::from_ref(&pattern),
        128,
    );
    let rejected = resolver.resolve_candidate(
        BoundedTokenContext::current("사진"),
        CandidateSpans {
            core: "사".len().."사진".len(),
            anchor: "사".len().."사진".len(),
            consumed: "사".len().."사진".len(),
            token: 0.."사진".len(),
        },
        &[pattern],
        128,
    );

    assert_eq!(supported.outcome, ConstraintOutcome::Supported);
    assert_eq!(rejected.outcome, ConstraintOutcome::Contradicted);
}

#[test]
fn attached_auxiliary_accepts_a_contracted_whole_source_analysis() {
    let resolver = resolver_from_entries([
        expression("빨라져", "VA+EC+VX+EC", "빠르/VA/*+아/EC/*+지/VX/*+어/EC/*"),
        expression(
            "알려진",
            "VV+EC+VX+ETM",
            "알리/VV/*+어/EC/*+지/VX/*+ㄴ/ETM/*",
        ),
        expression(
            "뚜렷해졌다",
            "XR+XSA+EC+VX+EP+EF",
            "뚜렷/XR/*+하/XSA/*+어/EC/*+지/VX/*+었/EP/*+다/EF/*",
        ),
        atomic("비춰", "VV+EC"),
        atomic("볼", "VX+ETM"),
        atomic("사진", "NNG"),
    ]);
    let pattern = QueryMorphPattern::new(DataFinePos::Vx, "지").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    for (token, core_start) in [
        ("빨라져", "빨라".len()),
        ("알려진", "알려".len()),
        ("뚜렷해졌다", "뚜렷해".len()),
    ] {
        let supported = resolver.resolve_candidate(
            BoundedTokenContext::current(token),
            CandidateSpans {
                core: core_start..token.len(),
                anchor: core_start..token.len(),
                consumed: core_start..token.len(),
                token: 0..token.len(),
            },
            std::slice::from_ref(&pattern),
            128,
        );
        assert_eq!(supported.outcome, ConstraintOutcome::Supported, "{token}");
    }
    let rejected = resolver.resolve_candidate(
        BoundedTokenContext::current("사진"),
        CandidateSpans {
            core: "사".len().."사진".len(),
            anchor: "사".len().."사진".len(),
            consumed: "사".len().."사진".len(),
            token: 0.."사진".len(),
        },
        &[pattern],
        128,
    );
    let unrelated = QueryMorphPattern::new(DataFinePos::Vx, "빠지").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let unrelated = resolver.resolve_candidate(
        BoundedTokenContext::current("빨라져"),
        CandidateSpans {
            core: "빨".len().."빨라져".len(),
            anchor: "빨".len().."빨라져".len(),
            consumed: "빨".len().."빨라져".len(),
            token: 0.."빨라져".len(),
        },
        &[unrelated],
        128,
    );
    let aligned_auxiliary = QueryMorphPattern::new(DataFinePos::Vx, "보").with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    );
    let aligned_auxiliary = resolver.resolve_candidate(
        BoundedTokenContext::current("비춰볼"),
        CandidateSpans {
            core: "비춰".len().."비춰볼".len(),
            anchor: "비춰".len().."비춰볼".len(),
            consumed: "비춰".len().."비춰볼".len(),
            token: 0.."비춰볼".len(),
        },
        &[aligned_auxiliary],
        128,
    );

    assert_eq!(rejected.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(unrelated.outcome, ConstraintOutcome::Contradicted);
    assert_eq!(aligned_auxiliary.outcome, ConstraintOutcome::Supported);
}

#[test]
fn attached_auxiliary_path_composes_a_split_derivational_source_analysis() {
    let root_end = "뚜렷".len();
    let auxiliary_end = "뚜렷해졌".len();
    let token_end = "뚜렷해졌다".len();
    let edges = [
        Edge {
            span: 0..root_end,
            pos: "XR",
            components: Vec::new(),
        },
        Edge {
            span: root_end..auxiliary_end,
            pos: "XSA+EC+VX+EP",
            components: Vec::new(),
        },
        Edge {
            span: auxiliary_end..token_end,
            pos: "EF",
            components: Vec::new(),
        },
    ];
    let graph = EdgeGraph::from_edges(token_end, edges.into());
    assert!(has_complete_attached_auxiliary_path(token_end, &graph));

    let without_connective = [
        Edge {
            span: 0..root_end,
            pos: "XR",
            components: Vec::new(),
        },
        Edge {
            span: root_end..auxiliary_end,
            pos: "XSA+VX+EP",
            components: Vec::new(),
        },
        Edge {
            span: auxiliary_end..token_end,
            pos: "EF",
            components: Vec::new(),
        },
    ];
    let without_connective = EdgeGraph::from_edges(token_end, without_connective.into());
    assert!(!has_complete_attached_auxiliary_path(
        token_end,
        &without_connective,
    ));
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
        atomic("동안", "NNG"),
        atomic("동안", "MAG"),
        atomic("이", "VCP"),
        atomic("었습니다", "EP+EF"),
        atomic("끝", "NNG"),
        atomic("인", "VCP"),
        atomic("인", "JKS"),
        atomic("가", "JKS"),
        atomic("인가", "VCP+EF"),
        atomic("곳", "NNB"),
        atomic("공학", "NNG"),
        atomic("결과", "NNG"),
        atomic("왕친", "NNP"),
        atomic("고체", "NNG"),
        atomic("이긴", "VCP+ETN+JX"),
        atomic("것", "NNB"),
        atomic("이었다", "VCP+EP+EF"),
        atomic("바튼", "NNP"),
        atomic("반도", "NNG"),
        atomic("는", "VCP+ETM"),
        atomic("홍", "NNP"),
        atomic("맛", "NNG"),
        atomic("있다", "VA+EF"),
        atomic("입", "VCP"),
        atomic("니다", "EF"),
        atomic("에", "NNG"),
        atomic("에서", "JKB"),
        atomic("서", "JKB"),
        atomic("둥그스름", "NNG"),
        atomic("하", "NNG"),
        atomic("게", "JKB"),
        atomic("요코", "NNP"),
        atomic("씨", "NNB"),
        atomic("요코씨", "NNP"),
        atomic("년", "NNBC"),
        atomic("간", "NNB"),
        atomic("수십", "NR"),
        atomic("십", "NR"),
        atomic("일", "NR"),
        atomic("월", "NNBC"),
        atomic("백", "NR"),
        atomic("만", "NR"),
        atomic("천", "NR"),
        atomic("명", "NNBC"),
        atomic("톤", "NNBC"),
        atomic("미터", "NNBC"),
        atomic("의", "JKG"),
        atomic("조", "NNG"),
        atomic("족", "NNG"),
        atomic("산", "NNG"),
        atomic("맥", "NNG"),
        atomic("길", "NNG"),
        atomic("전", "NNG"),
        atomic("사", "NNG"),
        atomic("소", "NNG"),
        atomic("이", "JKS"),
        atomic("날", "NNG"),
        atomic("날씨", "NNG"),
        atomic("자본주", "NNG"),
        expression("자본주의", "NNG", "자본/NNG/*+주의/NNG/*"),
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
        atomic("위해", "VV+EC"),
        atomic("서", "EC"),
        atomic("사람", "NNG"),
        atomic("걸", "VV"),
        expression("걸려", "VV+EC", "걸리/VV/*+어/EC/*"),
        expression("미친다", "VV+EF", "미치/VV/*+ᆫ다/EF/*"),
        atomic("온", "MM"),
        expression("온", "VV+ETM", "오/VV/*+ᆫ/ETM/*"),
        atomic("어떤", "VA"),
        atomic("어떤가", "MM+EC"),
        atomic("가", "EC"),
        atomic("를", "JKO"),
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
        atomic("길", "VA"),
        atomic("어", "EC"),
        atomic("어", "MAG"),
        atomic("진", "VX+ETM"),
        atomic("길어진", "VV+ETM"),
        atomic("사", "NNG"),
        atomic("사진", "NNG"),
    ];
    resolver_from_entries(entries)
}

fn resolver_from_entries(
    entries: impl IntoIterator<Item = MecabSourceMorphologyEntry>,
) -> ConstraintResolver {
    let entries = entries.into_iter().collect::<Vec<_>>();
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

fn predicate_pattern(pos: DataFinePos, lexical_form: &str) -> QueryMorphPattern {
    QueryMorphPattern::new(pos, lexical_form).with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::Predicate {
            state: crate::ContinuationState::Terminal,
            nominal_particles: false,
        },
        ComponentCapability::SourceAndRuntime,
    )
}

fn nominal_pattern(pos: DataFinePos, lexical_form: &str) -> QueryMorphPattern {
    QueryMorphPattern::new(pos, lexical_form).with_candidate_contract(
        CandidateTokenRelation::PrefixWithContinuation,
        MorphContinuation::NominalParticles,
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
