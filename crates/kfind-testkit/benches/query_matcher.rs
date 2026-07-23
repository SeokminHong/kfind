use std::hint::black_box;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, DataFinePos, MecabSourceMorphologyEntry,
    decode_component_resource, encode_component_resource,
};
use kfind_matcher::MorphMatcher;
use kfind_morph::{
    BoundedTokenContext, CandidateSpans, CandidateTokenRelation, ComponentCapability,
    ConstraintOutcome, ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, MorphContinuation,
    PredicatePos, QueryMorphPattern,
};
use kfind_query::{
    BoundaryPolicy, CompileOptions, LexiconQueryAnalyzer, Lexicons, PhrasePolicy, compile_query,
};
use kfind_search::{InputOptions, InputSearcher};

const MATCHING_LINE: &str = "길을 걸어 갔다. 권한을 검증했습니다.\n";
const NON_MATCHING_LINE: &str = "사용자는 새 문서를 읽고 접근 정책을 확인했습니다.\n";
const CORPUS_LINES: usize = 1_024;
const MATCH_EVERY_LINES: usize = 64;
const PHRASE_MATCH_EVERY_LINES: usize = 4;
const SINGLE_ATOM_QUERY: &str = "걷다";
const PHRASE_QUERY: &str = "n:길 v:걷다";
const REPEATED_PHRASE_QUERY: &str = "lit:가 lit:가 lit:가 lit:가 lit:가 lit:가 lit:가 lit:가";
const REPEATED_PHRASE_SPANS: usize = 128;
const INPUT_SEARCHER_PHRASE_QUERY: &str = "lit:가 lit:나";
const INPUT_SEARCHER_PHRASE_REPETITIONS: usize = 4_096;
const MISSING_ATOM_LINE_REPETITIONS: usize = 262_144;
const CONTEXT_REPETITIONS: usize = 16_384;
const UNIQUE_CONTEXT_REPETITIONS: usize = CONTEXT_REPETITIONS;
const PHRASE_8_ATOMS_QUERY: &str =
    "n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다";
const DISJUNCTION_8_ATOMS_QUERY: &str =
    "n:사용자|n:권한|v:검증하다|adj:예쁘다|det:새|adv:빨리|n:기술|v:걷다";
const DISJUNCTION_SCAN_QUERY: &str = "lit:걸어|lit:사용자는";
const SHORT_MATCHING_TEXT: &[u8] = "길을 걸었다.".as_bytes();

fn query_compile(criterion: &mut Criterion) {
    let analyzer = analyzer();
    let options = CompileOptions::default();

    let single_atom = compile_query(SINGLE_ATOM_QUERY, &options, &analyzer)
        .expect("single-atom benchmark query must compile");
    assert_eq!(single_atom.atoms.len(), 1);
    let phrase = compile_query(PHRASE_8_ATOMS_QUERY, &options, &analyzer)
        .expect("phrase benchmark query must compile");
    assert_eq!(phrase.atoms.len(), 8);
    let disjunction = compile_query(DISJUNCTION_8_ATOMS_QUERY, &options, &analyzer)
        .expect("disjunction benchmark query must compile");
    assert_eq!(disjunction.atoms.len(), 1);

    let mut group = criterion.benchmark_group("query_compile");
    group.bench_function("single_atom", |bencher| {
        bencher.iter(|| {
            compile_query(
                black_box(SINGLE_ATOM_QUERY),
                black_box(&options),
                black_box(&analyzer),
            )
            .expect("single-atom benchmark query must compile")
        });
    });
    group.bench_function("phrase_8_atoms", |bencher| {
        bencher.iter(|| {
            compile_query(
                black_box(PHRASE_8_ATOMS_QUERY),
                black_box(&options),
                black_box(&analyzer),
            )
            .expect("phrase benchmark query must compile")
        });
    });
    group.bench_function("disjunction_8_atoms", |bencher| {
        bencher.iter(|| {
            compile_query(
                black_box(DISJUNCTION_8_ATOMS_QUERY),
                black_box(&options),
                black_box(&analyzer),
            )
            .expect("disjunction benchmark query must compile")
        });
    });
    group.finish();
}

fn matcher_scan(criterion: &mut Criterion) {
    let analyzer = analyzer();
    let component_resource = Arc::new(component_resource());
    let plan = compile_query("걷다", &CompileOptions::default(), &analyzer)
        .expect("benchmark query must compile");
    let matcher = MorphMatcher::new(Arc::new(plan)).expect("benchmark matcher must build");
    let short_plan = Arc::clone(matcher.plan());
    assert_eq!(matcher.find_all_with_meta(SHORT_MATCHING_TEXT).len(), 1);
    let corpus = deterministic_corpus(MATCH_EVERY_LINES);
    assert_eq!(
        matcher.find_all_with_meta(&corpus).len(),
        CORPUS_LINES / MATCH_EVERY_LINES
    );
    let mut group = criterion.benchmark_group("matcher");
    group.bench_function("build_and_find_short", |bencher| {
        bencher.iter(|| {
            MorphMatcher::new(Arc::clone(black_box(&short_plan)))
                .expect("benchmark matcher must build")
                .find_all_with_meta(black_box(SHORT_MATCHING_TEXT))
        });
    });
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.bench_function("scan_deterministic_corpus", |bencher| {
        bencher.iter(|| matcher.find_all_with_meta(black_box(&corpus)));
    });

    let disjunction_plan = compile_query(
        DISJUNCTION_SCAN_QUERY,
        &CompileOptions::default(),
        &analyzer,
    )
    .expect("disjunction scan benchmark query must compile");
    let disjunction_matcher = MorphMatcher::new(Arc::new(disjunction_plan))
        .expect("disjunction scan benchmark matcher must build");
    assert_eq!(
        disjunction_matcher.find_all_with_meta(&corpus).len(),
        CORPUS_LINES
    );
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.bench_function("disjunction_find_all", |bencher| {
        bencher.iter(|| disjunction_matcher.find_all_with_meta(black_box(&corpus)));
    });

    let phrase_plan = compile_query(PHRASE_QUERY, &CompileOptions::default(), &analyzer)
        .expect("phrase benchmark query must compile");
    let phrase_matcher = MorphMatcher::with_component_resource(
        Arc::new(phrase_plan),
        Arc::clone(&component_resource),
    )
    .expect("phrase benchmark matcher must build");
    let phrase_corpus = deterministic_corpus(PHRASE_MATCH_EVERY_LINES);
    assert_eq!(
        phrase_matcher.find_all_with_meta(&phrase_corpus).len(),
        CORPUS_LINES / PHRASE_MATCH_EVERY_LINES
    );
    group.throughput(Throughput::Bytes(phrase_corpus.len() as u64));
    group.bench_function("phrase_find_all", |bencher| {
        bencher.iter(|| phrase_matcher.find_all_with_meta(black_box(&phrase_corpus)));
    });

    let repeated_options = CompileOptions {
        boundary: BoundaryPolicy::Any,
        phrase: PhrasePolicy {
            max_gap: REPEATED_PHRASE_SPANS,
        },
        ..CompileOptions::default()
    };
    let repeated_plan = compile_query(REPEATED_PHRASE_QUERY, &repeated_options, &analyzer)
        .expect("repeated phrase benchmark query must compile");
    let repeated_matcher =
        MorphMatcher::new(Arc::new(repeated_plan)).expect("repeated phrase matcher must build");
    let repeated_corpus = "가".repeat(REPEATED_PHRASE_SPANS).into_bytes();
    let repeated_matches = repeated_matcher.find_all_with_meta(&repeated_corpus);
    assert_eq!(repeated_matches.len(), 1);
    assert_eq!(repeated_matches[0].span, 0..repeated_corpus.len());
    group.throughput(Throughput::Bytes(repeated_corpus.len() as u64));
    group.bench_function("phrase_find_all_repeated", |bencher| {
        bencher.iter(|| repeated_matcher.find_all_with_meta(black_box(&repeated_corpus)));
    });

    let input_searcher_options = CompileOptions {
        boundary: BoundaryPolicy::Any,
        phrase: PhrasePolicy { max_gap: 0 },
        ..CompileOptions::default()
    };
    let input_searcher_plan = compile_query(
        INPUT_SEARCHER_PHRASE_QUERY,
        &input_searcher_options,
        &analyzer,
    )
    .expect("input searcher phrase benchmark query must compile");
    let input_searcher_matcher = MorphMatcher::new(Arc::new(input_searcher_plan))
        .expect("input searcher phrase benchmark matcher must build");
    let input_searcher_line =
        format!("{}\n", "가나".repeat(INPUT_SEARCHER_PHRASE_REPETITIONS)).into_bytes();
    let mut input_searcher =
        InputSearcher::new(InputOptions::default()).expect("input searcher must build");
    let result = input_searcher
        .search_reader(
            &input_searcher_matcher,
            PathBuf::from("repeated-line.txt"),
            Cursor::new(&input_searcher_line),
        )
        .expect("input searcher benchmark corpus must be searchable");
    assert_eq!(
        result.matched_spans,
        Some(INPUT_SEARCHER_PHRASE_REPETITIONS as u64)
    );
    group.throughput(Throughput::Bytes(input_searcher_line.len() as u64));
    group.bench_function("phrase_input_searcher_repeated_line", |bencher| {
        bencher.iter(|| {
            input_searcher
                .search_reader(
                    black_box(&input_searcher_matcher),
                    PathBuf::from("repeated-line.txt"),
                    Cursor::new(black_box(&input_searcher_line)),
                )
                .expect("input searcher benchmark corpus must be searchable")
        });
    });

    let mut summary_input_searcher = InputSearcher::new(InputOptions {
        capture_records: false,
        ..InputOptions::default()
    })
    .expect("summary input searcher must build");
    let summary = summary_input_searcher
        .search_reader(
            &input_searcher_matcher,
            PathBuf::from("repeated-line.txt"),
            Cursor::new(&input_searcher_line),
        )
        .expect("summary benchmark corpus must be searchable");
    assert_eq!(summary.matching_lines, 1);
    assert_eq!(summary.matched_spans, None);
    group.throughput(Throughput::Bytes(input_searcher_line.len() as u64));
    group.bench_function("phrase_input_searcher_repeated_line_exists", |bencher| {
        bencher.iter(|| {
            summary_input_searcher
                .search_reader(
                    black_box(&input_searcher_matcher),
                    PathBuf::from("repeated-line.txt"),
                    Cursor::new(black_box(&input_searcher_line)),
                )
                .expect("summary benchmark corpus must be searchable")
        });
    });

    let missing_atom_line = "가 ".repeat(MISSING_ATOM_LINE_REPETITIONS).into_bytes();
    assert_eq!(missing_atom_line.len(), 1024 * 1024);
    let missing_atom_result = summary_input_searcher
        .search_reader(
            &input_searcher_matcher,
            PathBuf::from("missing-atom-line.txt"),
            Cursor::new(&missing_atom_line),
        )
        .expect("missing-atom benchmark corpus must be searchable");
    assert!(!missing_atom_result.has_match());
    group.throughput(Throughput::Bytes(missing_atom_line.len() as u64));
    group.bench_function("phrase_input_searcher_missing_atom_long_line", |bencher| {
        bencher.iter(|| {
            summary_input_searcher
                .search_reader(
                    black_box(&input_searcher_matcher),
                    PathBuf::from("missing-atom-line.txt"),
                    Cursor::new(black_box(&missing_atom_line)),
                )
                .expect("missing-atom benchmark corpus must be searchable")
        });
    });

    let mut sparse_tail_line = "가 ".repeat(MISSING_ATOM_LINE_REPETITIONS - 1);
    assert_eq!(sparse_tail_line.pop(), Some(' '));
    sparse_tail_line.push_str("나  ");
    let sparse_tail_line = sparse_tail_line.into_bytes();
    assert_eq!(sparse_tail_line.len(), 1024 * 1024);
    let sparse_tail_result = input_searcher
        .search_reader(
            &input_searcher_matcher,
            PathBuf::from("sparse-tail-line.txt"),
            Cursor::new(&sparse_tail_line),
        )
        .expect("sparse-tail benchmark corpus must be searchable");
    assert_eq!(sparse_tail_result.matched_spans, Some(1));
    group.throughput(Throughput::Bytes(sparse_tail_line.len() as u64));
    group.bench_function("phrase_input_searcher_sparse_tail_long_line", |bencher| {
        bencher.iter(|| {
            input_searcher
                .search_reader(
                    black_box(&input_searcher_matcher),
                    PathBuf::from("sparse-tail-line.txt"),
                    Cursor::new(black_box(&sparse_tail_line)),
                )
                .expect("sparse-tail benchmark corpus must be searchable")
        });
    });

    let context_plan = Arc::new(
        compile_query("adv:매일", &CompileOptions::default(), &analyzer)
            .expect("context benchmark query must compile"),
    );
    let context_first_text = "매일 보고".as_bytes();
    group.throughput(Throughput::Elements(1));
    group.bench_function("build_and_find_structural_exact", |bencher| {
        bencher.iter(|| {
            MorphMatcher::with_component_resource(
                Arc::clone(black_box(&context_plan)),
                Arc::clone(black_box(&component_resource)),
            )
            .expect("context benchmark matcher must build")
            .find_all_with_meta(black_box(context_first_text))
        });
    });
    let context_matcher = MorphMatcher::with_component_resource(context_plan, component_resource)
        .expect("context benchmark matcher must build");
    let context_line = "매일 ".repeat(CONTEXT_REPETITIONS).into_bytes();
    assert_eq!(
        context_matcher.find_all_with_meta(&context_line).len(),
        CONTEXT_REPETITIONS
    );
    group.throughput(Throughput::Bytes(context_line.len() as u64));
    group.bench_function("context_repeated_long_line", |bencher| {
        bencher.iter(|| context_matcher.find_all_with_meta(black_box(&context_line)));
    });

    let mut alternating_context = String::with_capacity(context_line.len() + CONTEXT_REPETITIONS);
    for index in 0..CONTEXT_REPETITIONS {
        alternating_context.push_str("매일");
        alternating_context.push_str(if index % 2 == 0 { " " } else { "  " });
    }
    let alternating_context = alternating_context.into_bytes();
    assert_eq!(
        context_matcher
            .find_all_with_meta(&alternating_context)
            .len(),
        CONTEXT_REPETITIONS
    );
    group.throughput(Throughput::Bytes(alternating_context.len() as u64));
    group.bench_function("context_alternating_spacing_long_line", |bencher| {
        bencher.iter(|| context_matcher.find_all_with_meta(black_box(&alternating_context)));
    });

    let constant_context = "가 매일 나 "
        .repeat(UNIQUE_CONTEXT_REPETITIONS)
        .into_bytes();
    assert_eq!(
        context_matcher.find_all_with_meta(&constant_context).len(),
        UNIQUE_CONTEXT_REPETITIONS
    );
    group.throughput(Throughput::Bytes(constant_context.len() as u64));
    group.bench_function("context_constant_neighbors_long_line", |bencher| {
        bencher.iter(|| context_matcher.find_all_with_meta(black_box(&constant_context)));
    });

    let mut unique_context = String::with_capacity(constant_context.len());
    for index in 0..UNIQUE_CONTEXT_REPETITIONS {
        use std::fmt::Write;
        let previous = char::from_u32(0xac00 + (index / 128) as u32)
            .expect("benchmark previous token must be valid Hangul");
        let next = char::from_u32(0xac00 + (index % 128) as u32)
            .expect("benchmark next token must be valid Hangul");
        write!(unique_context, "{previous} 매일 {next} ")
            .expect("writing benchmark context must succeed");
    }
    let unique_context = unique_context.into_bytes();
    assert_eq!(
        context_matcher.find_all_with_meta(&unique_context).len(),
        UNIQUE_CONTEXT_REPETITIONS
    );
    group.throughput(Throughput::Bytes(unique_context.len() as u64));
    group.bench_function("context_unique_neighbors_long_line", |bencher| {
        bencher.iter(|| context_matcher.find_all_with_meta(black_box(&unique_context)));
    });

    let mut unique_current = String::with_capacity(constant_context.len());
    for index in 0..UNIQUE_CONTEXT_REPETITIONS {
        use std::fmt::Write;
        let first = char::from_u32(0xac00 + (index / 128) as u32)
            .expect("benchmark token suffix must be valid Hangul");
        let second = char::from_u32(0xac00 + (index % 128) as u32)
            .expect("benchmark token suffix must be valid Hangul");
        write!(unique_current, "매일{first}{second} ")
            .expect("writing benchmark current token must succeed");
    }
    let unique_current = unique_current.into_bytes();
    assert!(
        context_matcher
            .find_all_with_meta(&unique_current)
            .is_empty()
    );
    group.throughput(Throughput::Bytes(unique_current.len() as u64));
    group.bench_function("context_unique_current_long_line", |bencher| {
        bencher.iter(|| context_matcher.find_all_with_meta(black_box(&unique_current)));
    });
    group.finish();
}

fn structural_constraint(criterion: &mut Criterion) {
    let resource = Arc::new(component_resource());
    let resolver = ConstraintResolver::new(resource);
    let cases = vec![
        (
            BoundedTokenContext {
                previous: None,
                current: "매일",
                next: Some("보고"),
            },
            CandidateSpans {
                core: 0.."매일".len(),
                anchor: 0.."매일".len(),
                consumed: 0.."매일".len(),
                token: 0.."매일".len(),
            },
            QueryMorphPattern::new(DataFinePos::Mag, "매일"),
        ),
        (
            BoundedTokenContext {
                previous: Some("아니라"),
                current: "매일",
                next: Some("수도"),
            },
            CandidateSpans {
                core: 0.."매".len(),
                anchor: 0.."매".len(),
                consumed: 0.."매일".len(),
                token: 0.."매일".len(),
            },
            QueryMorphPattern::new(DataFinePos::Nng, "매").with_candidate_contract(
                CandidateTokenRelation::Whole,
                MorphContinuation::Exact,
                ComponentCapability::SourceAndRuntime,
            ),
        ),
    ];
    assert!(cases.iter().all(|(context, spans, pattern)| {
        resolver
            .resolve_candidate(
                *context,
                spans.clone(),
                std::slice::from_ref(pattern),
                DEFAULT_LATTICE_NODE_LIMIT,
            )
            .outcome
            == ConstraintOutcome::Supported
    }));

    let mut group = criterion.benchmark_group("structural_constraint");
    group.throughput(Throughput::Elements(cases.len() as u64));
    group.bench_function("resolve_candidate", |bencher| {
        bencher.iter(|| {
            for (context, spans, pattern) in &cases {
                black_box(black_box(&resolver).resolve_candidate(
                    black_box(*context),
                    black_box(spans.clone()),
                    std::slice::from_ref(black_box(pattern)),
                    DEFAULT_LATTICE_NODE_LIMIT,
                ));
            }
        });
    });

    let dense_resource = Arc::new(dense_component_resource());
    let dense_resolver = ConstraintResolver::new(dense_resource).with_attached_auxiliary(true);
    let dense_token = "가".repeat(63);
    let dense_graph = dense_resolver
        .prepare_token_graph_for_candidate(&dense_token, DEFAULT_LATTICE_NODE_LIMIT, true, true)
        .expect("dense token graph must stay inside the node limit");
    assert!(dense_graph.memory_usage() > dense_token.len());
    group.throughput(Throughput::Elements(4_032));
    group.bench_function("prepare_dense_token_graph", |bencher| {
        bencher.iter(|| {
            black_box(&dense_resolver)
                .prepare_token_graph_for_candidate(
                    black_box(&dense_token),
                    DEFAULT_LATTICE_NODE_LIMIT,
                    true,
                    true,
                )
                .expect("dense token graph must stay inside the node limit")
        });
    });

    let (dense_unique_pos_resource, dense_unique_pos_token) = dense_unique_pos_component_resource();
    let dense_unique_pos_resource = Arc::new(dense_unique_pos_resource);
    let dense_unique_pos_resolver = ConstraintResolver::new(dense_unique_pos_resource);
    dense_unique_pos_resolver
        .prepare_token_graph_for_candidate(
            &dense_unique_pos_token,
            DEFAULT_LATTICE_NODE_LIMIT,
            true,
            true,
        )
        .expect("dense unique POS token graph must stay inside the node limit");
    group.throughput(Throughput::Elements(4_032));
    group.bench_function("prepare_dense_unique_pos_token_graph", |bencher| {
        bencher.iter(|| {
            black_box(&dense_unique_pos_resolver)
                .prepare_token_graph_for_candidate(
                    black_box(&dense_unique_pos_token),
                    DEFAULT_LATTICE_NODE_LIMIT,
                    true,
                    true,
                )
                .expect("dense unique POS token graph must stay inside the node limit")
        });
    });

    let dense_component_resource = Arc::new(dense_component_span_resource());
    let dense_component_resolver = ConstraintResolver::new(dense_component_resource);
    dense_component_resolver
        .prepare_token_graph_for_candidate(&dense_token, DEFAULT_LATTICE_NODE_LIMIT, true, true)
        .expect("dense component token graph must stay inside the node limit");
    group.throughput(Throughput::Elements(4_032));
    group.bench_function("prepare_dense_component_token_graph", |bencher| {
        bencher.iter(|| {
            black_box(&dense_component_resolver)
                .prepare_token_graph_for_candidate(
                    black_box(&dense_token),
                    DEFAULT_LATTICE_NODE_LIMIT,
                    true,
                    true,
                )
                .expect("dense component token graph must stay inside the node limit")
        });
    });

    let dense_path_resource = Arc::new(dense_unit_path_component_resource());
    let dense_path_resolver = ConstraintResolver::new(dense_path_resource);
    let dense_path_token = "가".repeat(63);
    let dense_path_graph = Arc::new(
        dense_path_resolver
            .prepare_token_graph_for_candidate(
                &dense_path_token,
                DEFAULT_LATTICE_NODE_LIMIT,
                false,
                false,
            )
            .expect("dense path graph must stay inside the node limit"),
    );
    let dense_path_context = dense_path_resolver
        .prepare_context_with_token_graph(
            BoundedTokenContext::current(&dense_path_token),
            dense_path_graph,
        )
        .expect("dense path context must use the prepared token graph");
    let dense_path_cases = (8_usize..24)
        .map(|syllables| {
            let core_end = syllables * "가".len();
            (
                CandidateSpans {
                    core: 0..core_end,
                    anchor: 0..core_end,
                    consumed: 0..core_end,
                    token: 0..dense_path_token.len(),
                },
                QueryMorphPattern::new(DataFinePos::Nng, &dense_path_token[..core_end])
                    .with_candidate_contract(
                        CandidateTokenRelation::Whole,
                        MorphContinuation::Exact,
                        ComponentCapability::SourceAndRuntime,
                    ),
            )
        })
        .collect::<Vec<_>>();
    assert!(dense_path_cases.iter().all(|(spans, pattern)| {
        dense_path_context
            .resolve_candidate(spans.clone(), std::slice::from_ref(pattern))
            .outcome
            == ConstraintOutcome::Supported
    }));
    group.throughput(Throughput::Elements(dense_path_cases.len() as u64));
    group.bench_function("resolve_dense_preferred_paths", |bencher| {
        bencher.iter(|| {
            for (spans, pattern) in &dense_path_cases {
                black_box(black_box(&dense_path_context).resolve_candidate(
                    black_box(spans.clone()),
                    std::slice::from_ref(black_box(pattern)),
                ));
            }
        });
    });

    let ambiguous_suffix_resolver =
        ConstraintResolver::new(Arc::new(ambiguous_particle_suffix_resource(20)));
    for repetitions in [12_usize, 20] {
        let text = format!("가다{}끝", "나".repeat(repetitions));
        assert!(
            !ambiguous_suffix_resolver.supports_predicate_ending_particle_path(
                &text,
                "가".len(),
                "가다".len(),
                PredicatePos::Verb,
                DEFAULT_LATTICE_NODE_LIMIT,
            )
        );
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            format!("reject_ambiguous_particle_suffix_{repetitions}"),
            |bencher| {
                bencher.iter(|| {
                    black_box(&ambiguous_suffix_resolver).supports_predicate_ending_particle_path(
                        black_box(&text),
                        "가".len(),
                        "가다".len(),
                        PredicatePos::Verb,
                        DEFAULT_LATTICE_NODE_LIMIT,
                    )
                });
            },
        );
    }

    let dense_selection_text = "나".repeat(20);
    let dense_selection_resolver =
        ConstraintResolver::new(Arc::new(dense_nominal_particle_resource(20)));
    let dense_selection_graph = Arc::new(
        dense_selection_resolver
            .prepare_token_graph_for_candidate(
                &dense_selection_text,
                DEFAULT_LATTICE_NODE_LIMIT,
                false,
                false,
            )
            .expect("dense selection token graph must prepare"),
    );
    dense_selection_resolver
        .prepare_context_with_token_graph(
            BoundedTokenContext {
                previous: None,
                current: &dense_selection_text,
                next: None,
            },
            Arc::clone(&dense_selection_graph),
        )
        .expect("dense selection context must prepare");
    group.bench_function("select_dense_nominal_particle_facts", |bencher| {
        bencher.iter(|| {
            dense_selection_resolver
                .prepare_context_with_token_graph(
                    BoundedTokenContext {
                        previous: None,
                        current: black_box(&dense_selection_text),
                        next: None,
                    },
                    Arc::clone(black_box(&dense_selection_graph)),
                )
                .expect("dense selection context must prepare")
        });
    });
    group.bench_function("prepare_dense_nominal_particle_context", |bencher| {
        bencher.iter(|| {
            dense_selection_resolver
                .prepare_context_for_candidate(
                    BoundedTokenContext {
                        previous: None,
                        current: black_box(&dense_selection_text),
                        next: None,
                    },
                    DEFAULT_LATTICE_NODE_LIMIT,
                    false,
                    false,
                )
                .expect("dense selection context must prepare")
        });
    });
    group.finish();
}

fn deterministic_corpus(match_every_lines: usize) -> Vec<u8> {
    let mut corpus = String::with_capacity(NON_MATCHING_LINE.len() * CORPUS_LINES);
    for line_index in 0..CORPUS_LINES {
        let line = if line_index % match_every_lines == 0 {
            MATCHING_LINE
        } else {
            NON_MATCHING_LINE
        };
        corpus.push_str(line);
    }
    corpus.into_bytes()
}

fn analyzer() -> LexiconQueryAnalyzer {
    let lexicons = Lexicons::embedded().expect("embedded lexicons must be valid");
    LexiconQueryAnalyzer::new(Arc::new(lexicons))
}

fn component_resource() -> kfind_data::ComponentResource {
    let entries = [
        component_entry("길", "NNG", -5_000),
        component_entry("사용자", "NNG", -5_000),
        component_entry("권한", "NNG", -5_000),
        component_entry("사용자권한", "NNG", 5_000),
        component_entry("대학교", "NNG", -5_000),
        component_entry("대", "XPN", 5_000),
        component_entry("학교", "NNG", 5_000),
        component_entry("공", "NNG", 0),
        component_entry("공공", "NNG", 0),
        component_entry("매일", "MAG", 0),
        component_entry("매일", "NNG", 0),
        component_entry("매", "NNG", 0),
        component_entry("일", "VCP+ETM", 0),
        component_entry("보고", "VV+EC", 0),
        component_entry("아니", "VCN", 0),
        component_entry("라", "EC", 0),
        component_entry("수도", "NNB+JX", 0),
    ];
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("benchmark component resource must encode");
    decode_component_resource("benchmark", bytes, &COMPONENT_RESOURCE_SOURCE_DIGEST)
        .expect("benchmark component resource must decode")
}

fn dense_component_resource() -> kfind_data::ComponentResource {
    let mut entries = Vec::with_capacity(126);
    let mut surface = String::new();
    for _ in 0..63 {
        surface.push('가');
        entries.push(component_entry(&surface, "NNG", 0));
        entries.push(component_entry(&surface, "VV+EC", 0));
    }
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("dense benchmark component resource must encode");
    decode_component_resource("dense benchmark", bytes, &COMPONENT_RESOURCE_SOURCE_DIGEST)
        .expect("dense benchmark component resource must decode")
}

fn dense_unique_pos_component_resource() -> (kfind_data::ComponentResource, String) {
    let token = (0..63)
        .map(|offset| char::from_u32('가' as u32 + offset).expect("benchmark scalar must be valid"))
        .collect::<String>();
    let mut boundaries = token
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(token.len());
    let mut entries = Vec::with_capacity(4_032);
    let mut pos_index = 0_u16;
    for start in 0..63 {
        for end in start + 1..=63 {
            let surface = &token[boundaries[start]..boundaries[end]];
            for _ in 0..2 {
                let pos = format!("U{pos_index:04X}");
                pos_index += 1;
                entries.push(component_entry(surface, &pos, 0));
            }
        }
    }
    assert_eq!(entries.len(), 4_032);
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("dense unique POS benchmark component resource must encode");
    let resource = decode_component_resource(
        "dense unique POS benchmark",
        bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .expect("dense unique POS benchmark component resource must decode");
    (resource, token)
}

fn dense_component_span_resource() -> kfind_data::ComponentResource {
    let mut entries = Vec::with_capacity(126);
    let mut surface = String::new();
    for _ in 0..63 {
        surface.push('가');
        if surface.len() == "가".len() {
            entries.push(component_entry(&surface, "NNG", 0));
            entries.push(component_entry(&surface, "NNP", 0));
            continue;
        }
        entries.push(component_span_entry(&surface, "NNG+JX", "NNG"));
        entries.push(component_span_entry(&surface, "NNP+JX", "NNP"));
    }
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("dense component benchmark resource must encode");
    decode_component_resource(
        "dense component benchmark",
        bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .expect("dense component benchmark resource must decode")
}

fn component_span_entry(
    surface: &str,
    pos: &'static str,
    first_component_pos: &'static str,
) -> MecabSourceMorphologyEntry {
    let first_end = surface
        .char_indices()
        .nth(1)
        .map_or(surface.len(), |(offset, _)| offset);
    let expression = format!(
        "{}/{first_component_pos}/*+{}/JX/*",
        &surface[..first_end],
        &surface[first_end..]
    );
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost: 0,
        analysis_type: "Compound".to_owned(),
        start_pos: first_component_pos.to_owned(),
        end_pos: "JX".to_owned(),
        expression,
    }
}

fn dense_unit_path_component_resource() -> kfind_data::ComponentResource {
    let mut entries = Vec::with_capacity(127);
    entries.push(component_entry("가", "JX", 0));
    let mut surface = String::new();
    for _ in 0..63 {
        surface.push('가');
        entries.push(component_entry(&surface, "NNG", 0));
        entries.push(component_entry(&surface, "VV+EC", 0));
    }
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("dense path benchmark component resource must encode");
    decode_component_resource(
        "dense path benchmark",
        bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .expect("dense path benchmark component resource must decode")
}

fn ambiguous_particle_suffix_resource(repetitions: usize) -> kfind_data::ComponentResource {
    let mut entries = Vec::with_capacity(repetitions + 2);
    entries.push(component_entry("가", "VV", 0));
    entries.push(component_entry("다", "EF", 0));
    let mut surface = String::new();
    for _ in 0..repetitions {
        surface.push('나');
        entries.push(component_entry(&surface, "JX", 0));
    }
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("ambiguous suffix benchmark component resource must encode");
    decode_component_resource(
        "ambiguous suffix benchmark",
        bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .expect("ambiguous suffix benchmark component resource must decode")
}

fn dense_nominal_particle_resource(repetitions: usize) -> kfind_data::ComponentResource {
    let mut entries = Vec::with_capacity(repetitions * 2);
    let mut surface = String::new();
    for _ in 0..repetitions {
        surface.push('나');
        entries.push(component_entry(&surface, "NNG", 0));
        entries.push(component_entry(&surface, "JX", 0));
    }
    let bytes = encode_component_resource(COMPONENT_RESOURCE_SOURCE_DIGEST, &entries)
        .expect("dense selection benchmark component resource must encode");
    decode_component_resource(
        "dense selection benchmark",
        bytes,
        &COMPONENT_RESOURCE_SOURCE_DIGEST,
    )
    .expect("dense selection benchmark component resource must decode")
}

fn component_entry(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: "*".to_owned(),
    }
}

criterion_group!(benches, query_compile, matcher_scan, structural_constraint);
criterion_main!(benches);
