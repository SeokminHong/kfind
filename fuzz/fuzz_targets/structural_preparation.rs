#![no_main]

use std::sync::{Arc, OnceLock};

use kfind_data::{
    ComponentResource, DataFinePos, MecabSourceMorphologyEntry, decode_component_resource,
    encode_component_resource,
};
use kfind_matcher::MorphMatcher;
use kfind_morph::{
    BoundedTokenContext, CandidateSpans, CandidateTokenRelation, ComponentCapability,
    ConstraintResolver, ContinuationState, MorphContinuation, QueryMorphPattern,
};
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};
use libfuzzer_sys::fuzz_target;

mod seed;

const SOURCE_DIGEST: [u8; 32] = [0x4b; 32];
const TOKEN_SCALARS: [char; 12] = [
    '가', '나', '다', '라', '마', '보', '사', '아', '일', '하', '못', '진',
];

fuzz_target!(|data: &[u8]| {
    let decoded = seed::decode_hex(data);
    exercise(decoded.as_ref());
});

fn exercise(data: &[u8]) {
    let _ = structural_matcher().find_all_with_meta(data);

    let current = token(data, 0, "매일");
    let previous = token(data, 8, "가");
    let next = token(data, 16, "보고");
    let control = data.first().copied().unwrap_or(0);
    let context = BoundedTokenContext {
        previous: (control & 1 != 0).then_some(previous.as_str()),
        current: &current,
        next: (control & 2 != 0).then_some(next.as_str()),
    };
    let node_limit = usize::from(data.get(1).copied().unwrap_or(127) % 64) + 1;
    let include_nominal_copula = control & 4 != 0;
    let include_nominal_derivation_predicate = control & 8 != 0;
    let resolver = resolver();

    let direct = resolver.prepare_context_for_candidate(
        context,
        node_limit,
        include_nominal_copula,
        include_nominal_derivation_predicate,
    );
    let graph = resolver.prepare_token_graph_for_candidate(
        &current,
        node_limit,
        include_nominal_copula,
        include_nominal_derivation_predicate,
    );

    match (direct, graph) {
        (Err(direct_error), Err(graph_error)) => assert_eq!(direct_error, graph_error),
        (Ok(direct), Ok(graph)) => {
            let graph = Arc::new(graph);
            let split = resolver
                .prepare_context_with_token_graph(context, Arc::clone(&graph))
                .expect("a graph prepared for the same token must remain usable");
            for (spans, patterns) in candidate_cases(&current) {
                assert_eq!(
                    direct.resolve_candidate(spans.clone(), &patterns),
                    split.resolve_candidate(spans, &patterns),
                );
            }

            if next != current {
                let mismatched = BoundedTokenContext {
                    current: &next,
                    ..context
                };
                assert!(
                    resolver
                        .prepare_context_with_token_graph(mismatched, graph)
                        .is_err()
                );
            }
        }
        (direct, graph) => panic!(
            "split graph preparation diverged from direct preparation: direct={direct:?}, graph={graph:?}"
        ),
    }
}

fn candidate_cases(current: &str) -> Vec<(CandidateSpans, Vec<QueryMorphPattern>)> {
    let token = 0..current.len();
    let whole_spans = CandidateSpans {
        core: token.clone(),
        anchor: token.clone(),
        consumed: token.clone(),
        token: token.clone(),
    };
    let whole_patterns = [DataFinePos::Mag, DataFinePos::Nng, DataFinePos::Va]
        .into_iter()
        .map(|pos| QueryMorphPattern::new(pos, current))
        .collect();

    let prefix_end = current
        .char_indices()
        .nth(1)
        .map_or(current.len(), |(offset, _)| offset);
    let prefix = &current[..prefix_end];
    let prefix_spans = CandidateSpans {
        core: 0..prefix_end,
        anchor: 0..prefix_end,
        consumed: token.clone(),
        token,
    };
    let prefix_patterns = vec![
        QueryMorphPattern::new(DataFinePos::Nng, prefix).with_candidate_contract(
            CandidateTokenRelation::PrefixWithContinuation,
            MorphContinuation::NominalParticles,
            ComponentCapability::SourceAndRuntime,
        ),
        QueryMorphPattern::new(DataFinePos::Vv, prefix).with_candidate_contract(
            CandidateTokenRelation::PrefixWithContinuation,
            MorphContinuation::Predicate {
                state: ContinuationState::Terminal,
                nominal_particles: false,
            },
            ComponentCapability::SourceAndRuntime,
        ),
    ];

    vec![
        (whole_spans, whole_patterns),
        (prefix_spans, prefix_patterns),
    ]
}

fn token(data: &[u8], offset: usize, fallback: &str) -> String {
    let Some(length) = data.get(offset).map(|byte| usize::from(byte % 6) + 1) else {
        return fallback.to_owned();
    };
    (0..length)
        .map(|index| {
            let byte = data.get(offset + index + 1).copied().unwrap_or(0);
            TOKEN_SCALARS[usize::from(byte) % TOKEN_SCALARS.len()]
        })
        .collect()
}

fn resolver() -> &'static ConstraintResolver {
    static RESOLVER: OnceLock<ConstraintResolver> = OnceLock::new();
    RESOLVER.get_or_init(|| {
        ConstraintResolver::new(Arc::clone(component_resource())).with_attached_auxiliary(true)
    })
}

fn structural_matcher() -> &'static MorphMatcher {
    static MATCHER: OnceLock<MorphMatcher> = OnceLock::new();
    MATCHER.get_or_init(|| {
        let lexicons = Arc::new(Lexicons::embedded().expect("embedded lexicons must be valid"));
        let analyzer = LexiconQueryAnalyzer::new(lexicons);
        let plan = compile_query("adv:매일", &CompileOptions::default(), &analyzer)
            .expect("fixed structural fuzz query must compile");
        MorphMatcher::with_component_resource(Arc::new(plan), Arc::clone(component_resource()))
            .expect("fixed structural fuzz matcher must build")
    })
}

fn component_resource() -> &'static Arc<ComponentResource> {
    static RESOURCE: OnceLock<Arc<ComponentResource>> = OnceLock::new();
    RESOURCE.get_or_init(|| {
        let entries = [
            atomic("가", "NNG"),
            atomic("가", "MAG"),
            atomic("매일", "MAG"),
            atomic("보고", "MAG"),
            atomic("못", "MAG"),
            atomic("못하다", "VA+EF"),
            atomic("사진", "NNG"),
        ];
        let bytes = encode_component_resource(SOURCE_DIGEST, &entries)
            .expect("fixed structural fuzz resource must encode");
        let resource = decode_component_resource("<structural-fuzz>", bytes, &SOURCE_DIGEST)
            .expect("fixed structural fuzz resource must decode");
        Arc::new(resource)
    })
}

fn atomic(surface: &str, pos: &str) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 0,
        right_id: 0,
        word_cost: 0,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: "*".to_owned(),
    }
}
