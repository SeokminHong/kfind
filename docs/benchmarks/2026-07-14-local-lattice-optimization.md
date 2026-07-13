# 국소 lattice 제품 경로 최적화

- 측정일: 2026-07-14
- Criterion 기준 revision: `fbbf742`
- 후보 revision: `10dd69a`
- morphology 기준 보고서 revision: `b2d3c93`
- Criterion 환경: macOS 26.4, Apple M1 Max, Rust 1.97.0
- morphology 환경: Linux/aarch64, 10 logical CPUs, Python 3.12.13, Docker 29.6.1
- explicit-POS fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- untagged fixture: `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592`
- 후보 report SHA-256: `2581ad2168875a0a0136a18808ff34192ca632a1bdbd3a15bd779050329a1f43`

## 결론

제품 matcher는 local lattice의 N-best 경로를 만들지 않고 query 포함·제외별 최저 비용만
계산한다. compact component resource의 unknown model은 evaluator에서 한 번 파싱해 matcher
사이에 공유한다. 진단 보고서는 기존 N-best 경로와 비용 provenance를 유지한다.

세 입력을 묶은 제품 판정 Criterion p95는 10.596 µs에서 4.662 µs로 56.00% 줄었다. 진단
보고서 p95는 10.704 µs에서 10.492 µs로 1.98% 줄어 제품·진단 경로가 분리됐음을 확인했다.

1,000-case morphology 품질은 모든 profile에서 기준선과 같았다. full-POS `smart` 처리량은
8,809.4에서 13,474.8 cases/s로 52.96% 늘었고 p95는 0.3192 ms에서 0.1878 ms로 41.17%
줄었다. component 판정과 진단 projection의 불일치는 0건이었다.

## 국소 판정 microbenchmark

고정 compact component fixture에서 accept, reject, ambiguous 입력을 순환한다. resource 생성은
측정 전에 끝내며 `component_decision`은 제품 판정, `component_report`는 N-best 진단 보고서를
측정한다. 기본 Criterion 설정의 100개 sample에서 `times[i] / iters[i]`를 구하고 nearest-rank
p95를 사용했다.

| workload | 기준 p95 | 후보 p95 | 증감 |
| --- | ---: | ---: | ---: |
| `local_lattice/component_decision` | 10.596 µs | 4.662 µs | -56.00% |
| `local_lattice/component_report` | 10.704 µs | 10.492 µs | -1.98% |

## 형태소 품질·성능

Docker에서 고정 사전과 resource를 다시 생성하고 각 profile을 fresh process로 1회 warm-up 뒤
5회 측정했다. 기준선은 같은 fixture와 환경의 직전 승인 보고서다.

| workload | 기준 TP / FP / FN | 후보 TP / FP / FN | 기준 cases/s | 후보 cases/s | 기준 p95 | 후보 p95 | RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| explicit-POS, full-POS `smart` | 413 / 0 / 87 | 413 / 0 / 87 | 8,809.4 | 13,474.8 | 0.3192 ms | 0.1878 ms | 92.0 MiB |
| 사람용 무품사, full-POS `smart` | 410 / 0 / 90 | 410 / 0 / 90 | 7,439.0 | 11,908.2 | 0.3843 ms | 0.2072 ms | 92.0 MiB |
| User persona, full-POS `smart` | 410 / 0 / 90 | 410 / 0 / 90 | 7,197.3 | 11,893.6 | 0.4323 ms | 0.2068 ms | 92.0 MiB |

`any`는 component lattice를 실행하지 않는다. Agent의 처리량 변화는 이 최적화의 효과로
해석하지 않는다. hard-negative 결과와 dev의 TP/FP/FN도 기준선과 같았다.

![경계 정책별 성능](assets/2026-07-14-local-lattice-optimization-boundary-performance.svg)

![제품 persona와 외부 분석기](assets/2026-07-14-local-lattice-optimization-product-external-comparison.svg)

## 실제 CLI 사용 케이스

고정 100 MiB·1,000파일 corpus의 SHA-256은
`7692072cb7bff9261c1fa5933bde41b27e558170818eeac6d07cabdd673815ff`다. 사람용 `학교`
query는 component 후보가 한 줄뿐이어서 국소 판정 microbenchmark보다 개선 폭이 작다.

| workflow | 기준 wall | 후보 wall | 기준 처리량 | 후보 처리량 | 후보 RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Agent: embedded + `any` + explicit POS | 17.3 ms | 16.1 ms | 5,793.3 MiB/s | 6,193.6 MiB/s | 7.0 MiB |
| Human: full-POS + `smart` + untagged | 313.1 ms | 301.5 ms | 319.3 MiB/s | 331.6 MiB/s | 91.5 MiB |

Agent는 component를 사용하지 않는 대조 workload다. 두 CLI 행의 작은 차이는 전체 process와
filesystem 측정 변동을 포함한다.

![제품 workflow별 품질과 CLI 비용](assets/2026-07-14-local-lattice-optimization-product-workflows.svg)

## 재현

```console
cargo bench -p kfind-testkit --bench query_matcher -- local_lattice
scripts/benchmark-morphology.sh target/morph-benchmark-local-lattice
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark-local-lattice/report.json docs/benchmarks/assets \
  --prefix 2026-07-14-local-lattice-optimization-
```

Criterion 기준선은 제품 판정이 진단 보고서를 생성하던 구현에서 같은 harness로 측정했다.
외부 분석기 snapshot은 fixture, schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
