use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use kfind_matcher::MorphMatcher;
use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};

const MATCHING_LINE: &str = "길을 걸어 갔다. 권한을 검증했습니다.\n";
const NON_MATCHING_LINE: &str = "사용자는 새 문서를 읽고 접근 정책을 확인했습니다.\n";
const CORPUS_LINES: usize = 1_024;
const MATCH_EVERY_LINES: usize = 64;
const SINGLE_ATOM_QUERY: &str = "걷다";
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
    let plan = compile_query("걷다", &CompileOptions::default(), &analyzer)
        .expect("benchmark query must compile");
    let matcher = MorphMatcher::new(Arc::new(plan)).expect("benchmark matcher must build");
    let corpus = deterministic_corpus();
    assert_eq!(
        matcher.find_all_with_meta(&corpus).len(),
        CORPUS_LINES / MATCH_EVERY_LINES
    );
    let mut group = criterion.benchmark_group("matcher");
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.bench_function("scan_deterministic_corpus", |bencher| {
        bencher.iter(|| matcher.find_all_with_meta(black_box(&corpus)));
    });
    group.finish();
}

fn deterministic_corpus() -> Vec<u8> {
    let mut corpus = String::with_capacity(NON_MATCHING_LINE.len() * CORPUS_LINES);
    for line_index in 0..CORPUS_LINES {
        let line = if line_index % MATCH_EVERY_LINES == 0 {
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

criterion_group!(benches, query_compile, matcher_scan);
criterion_main!(benches);
