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
cargo bench -p kfind-testkit --bench query_matcher -- matcher/context_repeated_long_line
```

세 benchmark 모두 입력의 anchor·atom span 수집과 leftmost-longest non-overlapping 결과 선택을
포함한다. `phrase_find_all_repeated`는 가능한 atom 조합을 모두 만들어 메모리에 쌓지 않는지 감시한다.
`phrase_input_searcher_repeated_line`은 줄바꿈 없는 한 줄의 여러 결과를 실제 metadata 출력 경로로
수집할 때 남은 입력을 반복해서 다시 스캔하지 않는지 감시한다.
`context_repeated_long_line`은 문맥 candidate마다 전체 줄의 UTF-8을 반복 검증하지 않는지 감시한다.

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

## Morphology comparison

독립된 UD Korean-Kaist·KSL test split에서 `kfind` embedded/full-POS를 실행하고
Kiwi·Lindera·MeCab-ko·KOMORAN의 고정 품질·성능 스냅샷과 lemma/POS/span task를 비교한다.
full-POS 프로필은 제품 기본 CLI와 같은 enriched 용언 metadata를 함께 읽고, embedded
프로필은 두 외부 어휘 resource를 모두 읽지 않는다.
기본 실행은 kfind만 다시 측정하고 외부 결과는 저장된 스냅샷에서 읽는다. dev의 VCP/VCN
분석 slice는 성능 측정에서 제외한다.
별도 human fixture는 품사 옵션과 atom 태그를 생략하고, query 표제어가 어떤 지원 품사로도
없는 문장을 negative로 사용한다. embedded/full-POS의 smart/any 품질·성능과 auto plan
사용성을 같은 보고서의 `human_untagged` 절에 기록한다. 무품사 결과를 개선하기 위해 fixture,
gold 또는 negative 선택을 바꾸지 않는다.
명시적 품사 `smart` 변경은 고정 development에서 FN을 우선 줄이고 precision 99.00% 하한과
hard-negative 신규 FP 0을 지킨다. FN이 같은 후보끼리만 FP를 비교한다.
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
