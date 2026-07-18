# Benchmarks

이 문서는 benchmark의 현재 실행 방법과 측정 계약을 설명한다. 개별 보고서는 해당 측정의
입력, 환경과 결과를 보존하며, 현재 제품 계약은 [`specs/kfind.md`](../../specs/kfind.md)를
따른다.

## Global execution lock

공식 benchmark script는 build와 resource 준비 전에 Git common directory의 global lock을
획득한다. 같은 저장소의 다른 worktree에서 이미 benchmark가 실행 중이면 종료될 때까지
대기하며, 이 대기 시간은 workload 측정에 포함하지 않는다.

```console
scripts/benchmark-run.sh status
scripts/benchmark-run.sh status --json
scripts/benchmark-run.sh doctor
```

`status`는 현재 owner의 benchmark 이름, worktree, revision, command, PID, 경과 시간과
supervisor·자식 process의 생존 상태를 표시한다. 상태 확인 실패만으로 운영체제가 보유한 lock을
강제로 해제하지 않는다. 기본 대기와 실행 timeout은 모두 제한 없음이다. 초 단위 제한은 다음
환경 변수로 설정한다.

```console
KFIND_BENCHMARK_WAIT_TIMEOUT=3600 scripts/benchmark-morphology.sh
KFIND_BENCHMARK_RUN_TIMEOUT=7200 scripts/benchmark-morphology.sh
```

대기 timeout은 exit code 75, 실행 timeout은 exit code 124를 반환한다. raw `cargo`, `docker`나
임의의 재현 명령도 직렬화하려면 공통 runner로 감싼다.

```console
scripts/benchmark-run.sh run --name custom-check -- command arg
```

`scripts/benchmark-1gib.sh`는 고정 seed로 1 GiB mixed corpus를 생성하고 `kfind --literal --quiet --no-ignore`와 `rg -F --quiet --no-ignore`의 warm-cache 전체 scan을 비교한다.

```console
scripts/benchmark-1gib.sh
```

corpus는 기본적으로 `target/benchmark/1gib-mixed`에 생성되고 보고서 작성 뒤 삭제된다. 반복 측정을 위해 보존하려면 다음 환경 변수를 사용한다.

```console
KFIND_BENCH_KEEP_CORPUS=1 scripts/benchmark-1gib.sh
KFIND_BENCH_KEEP_CORPUS=1 KFIND_BENCH_REUSE_CORPUS=1 scripts/benchmark-1gib.sh
```

`KFIND_BENCH_RUNS`, `KFIND_BENCH_SCAN_REPETITIONS`, `KFIND_BENCH_REPORT`와 `KFIND_BENCH_*` corpus 설정을 환경 변수로 덮어쓸 수 있다. 공식 인수 보고서는 기본값을 사용한다.

이미 빌드한 release binary를 측정할 때는 `KFIND_BENCH_SKIP_BUILD=1`, `KFIND_BENCH_KFIND_BIN`, `KFIND_BENCH_GENERATOR_BIN`, `KFIND_BENCH_REVISION`을 함께 지정한다. 보고서의 revision과 실제 binary가 일치하도록 호출자가 보장해야 한다.

## Path-sorted output

`--sort path`의 결과 메모리, 정렬 조정 비용과 입력 편중을 분리해 측정한다.

```console
scripts/benchmark-sorted-output.sh
```

고정 생성기는 동일한 match 행을 반복한 high-hit, 모든 행의 내용이 다른 high-hit,
많은 작은 파일에 match가 없는 low-hit corpus를 만든다. 반복과 고유 high-hit을 함께
보아 반복 샘플에만 유리한 캐시·분기 효과를 분리하고, low-hit으로 경로 수집
비용을 감시한다. 각 corpus에서 sorted와 unsorted를 fresh process로 교대 실행하며,
warm-up 1회 후 wall time과 peak RSS를 5회 기록한다. 결과 TSV와 input·binary checksum,
revision, 도구 버전은 `target/benchmark/sorted-output/<revision>`에 저장한다.

이미 build한 binary는 `KFIND_BENCH_SKIP_BUILD=1`, `KFIND_BENCH_KFIND_BIN`,
`KFIND_BENCH_REVISION`으로 지정한다. 기준과 후보는 같은 `KFIND_BENCH_THREADS`, fixture 크기와
실행 횟수를 사용해야 한다.

## Query compile

단일 atom과 8 atom phrase compile benchmark는 다음 명령으로 실행한다.

```console
scripts/benchmark-criterion.sh query_compile
```

빠른 smoke 측정에는 마지막에 `--quick`을 추가한다. 목표 판정에는 기본 sample 설정과
`target/criterion/query_compile/*/new/sample.json`의 1회당 시간 p95를 사용한다.

## Phrase matcher

일반적인 다중 match corpus와 반복 span·큰 gap의 병적 입력을 각각 측정한다.

```console
cargo bench -p kfind-testkit --bench query_matcher -- matcher/phrase_find_all
cargo bench -p kfind-testkit --bench query_matcher -- matcher/phrase_find_all_repeated
cargo bench -p kfind-testkit --bench query_matcher -- matcher/phrase_input_searcher_repeated_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/phrase_input_searcher_repeated_line_exists
cargo bench -p kfind-testkit --bench query_matcher -- matcher/phrase_input_searcher_missing_atom_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/phrase_input_searcher_sparse_tail_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/context_repeated_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/context_alternating_spacing_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/context_constant_neighbors_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/context_unique_neighbors_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/context_unique_current_long_line
cargo bench -p kfind-testkit --bench query_matcher -- matcher/build_and_find_structural_exact
```

앞의 phrase benchmark는 입력의 anchor·atom span 수집과 leftmost-longest non-overlapping 결과 선택을
포함한다. `phrase_find_all_repeated`는 가능한 atom 조합을 모두 만들어 메모리에 쌓지 않는지 감시한다.
`phrase_input_searcher_repeated_line`은 줄바꿈 없는 한 줄의 여러 결과를 실제 metadata 출력 경로로
수집할 때 남은 입력을 반복해서 다시 스캔하지 않는지 감시한다.
`phrase_input_searcher_repeated_line_exists`는 같은 입력의 metadata 없는 존재 판정 경로를 분리해
측정한다.
`phrase_input_searcher_missing_atom_long_line`은 둘째 atom의 raw anchor가 없는 1 MiB 단일 줄에서
verifier와 atom span 적재를 건너뛰는지 감시한다. 모든 atom이 있는 병적 줄의 메모리 상한과는
분리해 해석한다.
`phrase_input_searcher_sparse_tail_long_line`은 둘째 atom을 줄 끝에 한 번 넣어 prefilter를
통과시키고, max-gap 밖의 첫 atom candidate를 active state에서 제거하는지 감시한다.
`context_repeated_long_line`은 문맥 candidate마다 전체 줄의 UTF-8을 반복 검증하지 않는지 감시한다.
`context_alternating_spacing_long_line`은 두 문맥 형태가 교대할 때의 비용을 감시한다. 반복·교대
workload는 모두 warm cache hit에 편중될 수 있다. Byte 수와 match 수가 같은
`context_constant_neighbors_long_line`과 `context_unique_neighbors_long_line`을 함께 측정해
반복 context hit와 고유 context miss를 분리한다.
`context_unique_current_long_line`은 현재 token까지 매번 바꾼 거부 입력으로 query 불변 token
graph 재사용이 불가능한 miss 비용을 감시한다. `build_and_find_structural_exact`는 matcher 생성과
첫 구조 검색을 함께 측정해 lazy graph 준비가 steady-state 수치에 가려지지 않게 한다.

## TUI pager held-key scroll

고정 2,000행 fixture에서 `j`를 50 Hz로 반복 입력하며 PTY를 계속 소비해, viewport 크기별
scroll frame 수와 출력 bytes, 최종 offset 도달 시간을 측정한다.

```console
python3 tools/tui-scroll-benchmark/benchmark.py \
  --binary target/release/kfind \
  --revision "$(git rev-parse HEAD)" \
  --label candidate
```

측정 결과는 실행 환경과 입력 checksum을 포함한 개별 보고서에 기록한다.

## TUI pager index memory

내장 pager가 임시 파일과 별도로 유지하는 source-line index와 layout-row index의 메모리·시간을
plain 결과와 match별 전개 결과에서 측정한다.

```console
cargo run --release --locked -p kfind-cli \
  --features pager-memory-benchmark \
  --bin kfind-pager-memory-benchmark -- \
  SOURCE_LINES MATCHES_PER_LINE TERMINAL_WIDTH
```

## Full POS startup

native CLI와 Node WASM이 full POS resource를 초기화하는 시간과 RSS를 literal scan과 분리해
측정한다.

## Optional component startup

resource 없는 Rust/WASM engine과 생성 후 compact component resource를 수동 초기화한 engine의
시간과 RSS를 분리해 측정한다. native 결과는 morphology report의 `component_startup`, WASM
결과는 별도 JSON에 기록한다.

```console
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
```

## Structural constraint

제품용 `ConstraintResolver`의 구조 판정을 고정 문장 배치에서 측정한다. full morphology
cost-lattice 진단은 이 제품 benchmark에 포함하지 않는다.

```console
scripts/benchmark-criterion.sh structural_constraint
```

`structural_constraint/resolve_candidate`는 고정된 짧은 제품 문맥을 측정한다.
`structural_constraint/prepare_dense_token_graph`는 node 상한에 가까운 합성 token graph를 매번
새로 준비해 시작 위치별 edge 탐색의 최악 복잡도를 감시한다. Nominal prefix, ending suffix와
predicate-connective 경계처럼 여러 구조 기능이 소비하는 도달성 사실은 token 준비당 한 번만
계산한다.
`structural_constraint/resolve_dense_preferred_paths`는 준비된 밀집 graph에서 서로 다른 component
후보의 최소 unit 경로 판정을 순환한다.
`structural_constraint/reject_ambiguous_particle_suffix_*`는 완성 경로가 없는 다분기 suffix의
12개·20개 반복 입력을 각각 거부해 중복 재귀와 stack 성장 회귀를 감시한다. 모든 workload를 함께
비교한다.
`structural_constraint/select_dense_nominal_particle_facts`는 준비된 token graph를 공유한 선택 비용,
`structural_constraint/prepare_dense_nominal_particle_context`는 같은 입력의 graph 생성부터 선택까지를
측정한다. 두 workload를 함께 비교해 graph 공유 표본의 편중을 확인한다.

## Morphology comparison

수동 검토를 통과한 UD Korean-Kaist test 문장에서 `kfind` embedded/full-POS를 실행하고
Kiwi·Lindera·MeCab-ko·KOMORAN의 고정 품질·성능 스냅샷과 lemma/POS/span task를 비교한다.
full-POS 프로필은 제품 기본 CLI와 같은 enriched 용언 metadata를 함께 읽고, embedded
프로필은 두 외부 어휘 resource를 모두 읽지 않는다.
기본 실행은 kfind만 다시 측정하고 외부 결과는 저장된 스냅샷에서 읽는다. dev의 VCP/VCN
분석 slice는 성능 측정에서 제외한다.
Canonical fixture에는 샘플링 후보 전체를 수동 검토해 표준 맞춤법을 확인한 Korean-Kaist
문장만 넣는다. 검토에서 제외한 Korean-Kaist 문장은 query-level gold가 없는 sentence
registry로 보존하며 canonical 품질 합계에 넣지 않는다.
Korean-KSL은 source-signal과 quota 보충 후보 전체를 수동 검토하고, 실제 오류로 확정한 문장만
별도 scored Robust 500-case로 만든다. 명시적 품사는 kfind와 외부 분석기 4종의
precision·recall·F1, 오류 class·scope별 품질과 성능을 비교한다. 무품사 kfind Human 결과는
입력 계약이 다르므로 별도 workflow로 보고한다. 현재 기준선은 `robustness=off`이며 fresh
process warm-up 1회 뒤 5회 측정한 초기화, cases/s, p50/p95, RSS의 median/min/max를 기록한다.
Robust 점수와 canonical 점수는 합치지 않는다.
별도 human fixture는 품사 옵션과 atom 태그를 생략하고, query 표제어가 어떤 지원 품사로도
없는 문장을 negative로 사용한다. embedded/full-POS의 smart/any 품질·성능과 auto plan
사용성을 같은 보고서의 `human_untagged` 절에 기록한다. 무품사 결과를 개선하기 위해 fixture,
gold 또는 negative 선택을 바꾸지 않는다.
고정 1,000-case fixture와 별도로 `query_matrix` 절은 canonical positive가 있는 고유 문장마다
정렬된 존재 질의를 최대 3개로 늘리고, 각 positive와 같은 품사의 부재 질의를 같은 문장에
대응시킨다. 명시적 품사 matrix는 kfind와 외부 분석기 4종, 별도 무품사 matrix는 kfind의
사람용 profile을 측정한다. 질의별 품질·성능과 문장별 모든 질의 회수율을 함께 기록하며,
strict와 contract-adjusted 기대값을 각각 적용한 confusion matrix와 문장 회수율을 병렬로
보존한다. 두 recall의 95% 구간은 문장 group 단위 cluster bootstrap으로 계산한다. 이 결과는
canonical 회귀선과 합치거나 대체하지 않는다.
명시적 품사 `smart` 변경은 고정 development에서 FN을 우선 줄이고 precision 99.00% 하한과
hard-negative 신규 contract FP 0을 지킨다. FN이 같은 후보끼리만 FP를 비교한다.
보고서는 기존 corpus-gold TP·FP·TN·FN과 precision·recall·F1을 그대로 보존하고, 수동 검토한
`contract_expected`가 있는 fixture에는 TPᶜ·FPᶜ·TNᶜ·FNᶜ와 contract precision·recall·F1을
추가한다. Contract registry는 문법 구조로 구분할 수 없는 동형이의와 source component를
contract positive로, gold 정렬 오류를 contract negative로 교정한다. 현재 비문·비표준 입력만
분모에서 제외하며 비용, 현재 profile과 미구현 표준 문법은 제외하지 않는다. kfind 표·차트는
raw와 contract를 나란히 표시하고 두 지표를 합치거나 strict 오류를 숨기지 않는다.
Canonical·hard-negative의 contract-positive 분모는 `PNᶜ = TPᶜ + FNᶜ`로 표기한다.
Recall 개선 보고서는 `PNᶜ`, `FNᶜ`와 `recallᶜ = TPᶜ / PNᶜ`를 함께 기록한다.
보고서의 `product_workflows`는 에이전트용 `embedded + any + 명시적 품사`와 사람용
`full-POS + smart + 무품사`를 먼저 제시하고, 전체 profile 행렬은 진단 자료로 둔다.
`product_use_cases`는 같은 두 profile을 100 MiB·1,000파일 고정 코퍼스의 독립 CLI
process로 실행하여 wall time, 처리량, peak RSS를 기록한다. 라이브러리 resource 조합의
초기화 시간과 peak RSS는 CLI workload와 분리한다.
`product-workflows.svg`는 profile별 precision·recall·F1·FP 후보와 실제 CLI 비용을 함께
표시하고 두 측정 단위가 다름을 명시한다.
`product-external-comparison.svg`는 같은 explicit-POS fixture와 gold에서 Agent, User와 외부
분석기 4종의 precision·recall·F1, 초기화, 처리량, p95와 peak RSS를 표시한다. Agent와 외부
분석기는 품사를 명시하고 User는 같은 query에서 품사를 제거한다. 따라서 동일 입력의 backend
순위가 아니라 실제 persona 입력을 반영한 제품 비교로 해석한다.
`robustness-quality.svg`는 같은 자연 오류 explicit-POS fixture에서 kfind Agent와 외부 분석기
기본 설정의 precision·recall·F1, target-span과 context-only recall을 비교한다.
`robustness-performance.svg`는 같은 행의 초기화, 처리량, p95와 peak RSS를 표시한다. 두 chart는
250 positive·250 negative, target-span positive 100건, context-only positive 150건,
`robustness=off`와 canonical 표준문 점수에 합산하지 않는다는 조건을 내부 subtitle에 보존한다.

외부 스냅샷은 test fixture, 성능 schema나 고정한 도구·어댑터 설정이 바뀔 때만 갱신한다.
기본 명령은 fixture·schema 불일치에서 자동 실행하거나 오래된 결과를 쓰지 않고 실패한다.
기본 benchmark 이미지는 `kfind` runner만 빌드하고, 외부 분석기와 전용 runner는 별도 refresh
이미지에서만 빌드한다.

```console
scripts/benchmark-morphology.sh
scripts/refresh-morph-baselines.sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets
python3 tools/morph-compare/export_site_snapshot.py \
  target/morph-benchmark/report.json docs/benchmarks/site-morphology.json \
  --revision "$(git rev-parse --short=12 HEAD)"
```

`benchmark-morphology.sh`는 기본적으로 현재 단계와 최종 보고서 경로만 stdout에 출력하고,
실패 진단은 stderr에 출력한다. Docker 빌드 과정과 생성된 Markdown 보고서 전문이 필요하면
`KFIND_MORPH_VERBOSE=1 scripts/benchmark-morphology.sh`를 사용한다.

`site-morphology.json`은 공개 site 차트에 필요한 승인 보고서 필드와 원본 report의 revision,
SHA-256을 보존한다. 승인 보고서와 site 차트를 갱신할 때 같은 변경에서 다시 생성한다.

관련 현재 계약은 다음 문서에 있다.

- [구조 기반 국소 형태 판정 계약](selective-morphology.md)
- [형태소 검색 품질 검증 계약](morphology-quality.md)
- [명사 smart-boundary 계약](nominal-boundary.md)
- [copula smart-boundary 계약](copula-boundary.md)
- [비표준·오타·띄어쓰기 입력 robustness 후속 설계](noisy-text-robustness-plan.md)
- [비표준·오타·띄어쓰기 입력 평가 계약](noisy-text-robustness-evaluation.md)

## Morphology prefix index

고정 MeCab snapshot의 표면형·품사·연결 ID·비용을 보존한 morphology index에서 packed
Double-Array trie와 FST의 크기, 초기화, exact lookup, common-prefix 열거와 RSS를 비교한다.

```console
scripts/benchmark-morph-index.sh
```
