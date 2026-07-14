use std::hint::black_box;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use kfind_data::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, DataFinePos, MecabSourceMorphologyEntry,
    decode_component_resource, encode_component_resource, parse_mecab_connection_matrix,
};
use kfind_matcher::MorphMatcher;
use kfind_morph::{
    DEFAULT_LATTICE_NODE_LIMIT, LocalComponentEvaluator, evaluate_local_component_paths,
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
const PHRASE_8_ATOMS_QUERY: &str =
    "n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다";

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
    let corpus = deterministic_corpus(MATCH_EVERY_LINES);
    assert_eq!(
        matcher.find_all_with_meta(&corpus).len(),
        CORPUS_LINES / MATCH_EVERY_LINES
    );
    let mut group = criterion.benchmark_group("matcher");
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.bench_function("scan_deterministic_corpus", |bencher| {
        bencher.iter(|| matcher.find_all_with_meta(black_box(&corpus)));
    });

    let phrase_plan = compile_query(PHRASE_QUERY, &CompileOptions::default(), &analyzer)
        .expect("phrase benchmark query must compile");
    let phrase_matcher =
        MorphMatcher::with_component_resource(Arc::new(phrase_plan), component_resource)
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
    group.finish();
}

fn local_lattice(criterion: &mut Criterion) {
    let resource = Arc::new(component_resource());
    let evaluator = LocalComponentEvaluator::new(Arc::clone(&resource));
    let cases = [
        ("사용자권한", "사용자".len(), "사용자권한".len()),
        ("대학교", "대".len(), "대학교".len()),
        ("공공", 0, "공".len()),
    ];
    let decisions = cases.map(|(text, start, end)| {
        evaluator
            .evaluate_decision(
                text,
                start..end,
                DataFinePos::Nng,
                DEFAULT_LATTICE_NODE_LIMIT,
            )
            .expect("benchmark lattice must have a complete path")
    });
    assert_eq!(
        decisions,
        [
            kfind_morph::LocalLatticeDecision::Accept,
            kfind_morph::LocalLatticeDecision::Reject,
            kfind_morph::LocalLatticeDecision::Ambiguous,
        ]
    );

    let mut group = criterion.benchmark_group("local_lattice");
    group.throughput(Throughput::Elements(cases.len() as u64));
    group.bench_function("component_decision", |bencher| {
        bencher.iter(|| {
            for (text, start, end) in cases {
                let decision = black_box(&evaluator)
                    .evaluate_decision(
                        black_box(text),
                        start..end,
                        DataFinePos::Nng,
                        DEFAULT_LATTICE_NODE_LIMIT,
                    )
                    .expect("benchmark lattice must have a complete path");
                black_box(decision);
            }
        });
    });
    group.bench_function("component_report", |bencher| {
        bencher.iter(|| {
            for (text, start, end) in cases {
                black_box(
                    evaluate_local_component_paths(
                        black_box(resource.as_ref()),
                        black_box(text),
                        start..end,
                        DataFinePos::Nng,
                        DEFAULT_LATTICE_NODE_LIMIT,
                    )
                    .expect("benchmark lattice must have a complete path"),
                );
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
    ];
    let matrix = parse_mecab_connection_matrix(
        "matrix.def",
        Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
    )
    .expect("benchmark matrix must be valid");
    let bytes = encode_component_resource(
        COMPONENT_RESOURCE_SOURCE_DIGEST,
        &entries,
        &matrix,
        b"DEFAULT 0 1 0\nHANGUL 0 1 2\n0xAC00..0xD7A3 HANGUL\n",
        b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
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

criterion_group!(benches, query_compile, matcher_scan, local_lattice);
criterion_main!(benches);
