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
    QueryMorphPattern,
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
const CONTEXT_REPETITIONS: usize = 16_384;
const PHRASE_8_ATOMS_QUERY: &str =
    "n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다";
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

    let context_plan = compile_query("adv:매일", &CompileOptions::default(), &analyzer)
        .expect("context benchmark query must compile");
    let context_matcher =
        MorphMatcher::with_component_resource(Arc::new(context_plan), component_resource)
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
