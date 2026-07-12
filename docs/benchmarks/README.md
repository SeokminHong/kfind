# Benchmarks

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
cargo bench -p kfind-testkit --bench query_matcher -- query_compile
```

빠른 smoke 측정에는 마지막에 `--quick`을 추가한다. 목표 판정에는 기본 sample 설정과
`target/criterion/query_compile/*/new/sample.json`의 1회당 시간 p95를 사용한다.

기준 결과는 [2026-07-11 query compile 보고서](2026-07-11-query-compile.md)에 기록한다.

## Morphology comparison

독립된 UD Korean-Kaist·KSL test split에서 `kfind` embedded/full-POS, Kiwi, Lindera의
lemma/POS/span 품질과
end-to-end 비용을 비교한다.

```console
scripts/benchmark-morphology.sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets
```

- [2026-07-12 비교 분석](2026-07-12-morphology-comparison.md)
- [후속 개선 핸드오프](2026-07-12-morphology-handoff.md)
- [선택적 국소 형태 추론 작업 계획](2026-07-12-selective-morphology-plan.md)

## Morphology prefix index

고정 MeCab snapshot의 표면형·품사·연결 ID·비용을 보존한 morphology index에서 packed
Double-Array trie와 FST의 크기, 초기화, exact lookup, common-prefix 열거와 RSS를 비교한다.

```console
scripts/benchmark-morph-index.sh
```

- [2026-07-12 prefix index 비교 결과](2026-07-12-morph-index-comparison.md)
