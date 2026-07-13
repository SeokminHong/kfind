# Benchmarks

날짜가 붙은 보고서는 해당 측정의 입력, 환경과 결과를 보존한다. 현재 제품 계약은
[`specs/kfind.md`](../../specs/kfind.md), 이어갈 작업은
[형태소 검색 개선 핸드오프](morphology-handoff.md)를 기준으로 한다. 활성 계약 문서는
완료 이력을 누적하지 않고 현재 기술 계약과 남은 검증만 유지한다.

`scripts/benchmark-1gib.sh`는 고정 seed로 1 GiB mixed corpus를 생성하고 `kfind --literal --quiet --no-ignore`와 `rg -F --quiet --no-ignore`의 warm-cache 전체 scan을 비교한다.

최신 기준 결과는 [2026-07-12 1 GiB 보고서](2026-07-12-1gib-mixed.md)에 기록한다.

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

## Full POS startup

native CLI와 Node WASM이 full POS resource를 초기화하는 시간과 RSS를 literal scan과 분리해
측정한다. 최신 비교는 [2026-07-13 full POS 지연 조회 보고서](2026-07-13-full-pos-startup.md)에
기록한다.

## Optional component startup

resource 없는 Rust/WASM engine과 생성 후 compact component resource를 수동 초기화한 engine의
시간과 RSS를 분리해 측정한다. native 결과는 morphology report의 `component_startup`, WASM
결과는 별도 JSON에 기록한다.

```console
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
```

최신 수치는 [smart component 검색 근거](2026-07-13-smart-component-evidence.md)에 기록한다.

## Morphology comparison

독립된 UD Korean-Kaist·KSL test split에서 `kfind` embedded/full-POS, Kiwi, Lindera의
lemma/POS/span 품질과
end-to-end 비용을 비교한다. dev의 VCP/VCN 지정사 판별 slice는 성능 측정에서 제외하고
source·raw tag별 confusion matrix와 local-context shadow 대상 수를 함께 기록한다.
별도 human fixture는 품사 옵션과 atom 태그를 생략하고, query 표제어가 어떤 지원 품사로도
없는 문장을 negative로 사용한다. embedded/full-POS의 smart/any 품질·성능과 auto plan
사용성을 같은 보고서의 `human_untagged` 절에 기록한다.

```console
scripts/benchmark-morphology.sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets
```

- [2026-07-12 비교 기준선](2026-07-12-morphology-comparison.md)
- [현재 smart component 품질·성능](2026-07-13-smart-component-evidence.md)
- [형태소 검색 개선 핸드오프](morphology-handoff.md)
- [선택적 국소 형태 추론 계약](selective-morphology.md)
- [형태소 검색 품질 검증 계약](morphology-quality.md)
- [명사 smart-boundary 계약](nominal-boundary.md)
- [VCP 지정사 smart-boundary 계약](copula-boundary.md)
- [지정사 lattice dev gold 진단](2026-07-13-copula-dev-diagnosis.md)
- [지정사 lattice 독립 평가](2026-07-13-copula-blind-evaluation.md)
- [local lattice 비용 분석](2026-07-12-lattice-cost-analysis.md)

## Morphology prefix index

고정 MeCab snapshot의 표면형·품사·연결 ID·비용을 보존한 morphology index에서 packed
Double-Array trie와 FST의 크기, 초기화, exact lookup, common-prefix 열거와 RSS를 비교한다.

```console
scripts/benchmark-morph-index.sh
```

- [2026-07-12 prefix index 비교 결과](2026-07-12-morph-index-comparison.md)
