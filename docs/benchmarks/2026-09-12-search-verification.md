# 구조 판정 진단과 근거 수집 분리의 CLI 비용

- 측정일: 2026-09-12
- 기준 revision: `bae0a005`; 후보 revision: `0768f87`
- 환경: macOS 26.6.2, Apple M1 Max, 10 logical CPUs, 32 GiB, Rust 1.97.0, Python 3.14.7
- 양쪽 모두 같은 환경에서 release locked build와 같은 full-POS·component resource를 사용했다.
- 성능 corpus: 14,192,640 bytes, 458,752 lines, SHA-256 `cfd252958729588624625d58e942781f2d1326b36d1690d7c17488fb60862d40`
- 측정 단위: 독립 CLI 7개 질의의 묶음. Warm-up 2회 뒤 10회 측정.
- 대표값은 중앙값이며 p95는 nearest rank다. 원본 JSON은 모든 표본, 정확한 argv, 도구 버전과 binary·resource·fixture checksum을 포함한다.

## 성능

| workload        | 기준 median / p95 (ms) | 후보 median / p95 (ms) | median 변화 | p95 변화 |
| --------------- | ---------------------: | ---------------------: | ----------: | -------: |
| kfind_any       |      506.961 / 512.943 |      505.785 / 545.140 |      -0.23% |   +6.28% |
| kfind_smart     |    2397.165 / 2451.871 |    2380.333 / 2482.426 |      -0.70% |   +1.25% |
| rg_enumerated   |        74.421 / 75.120 |        74.002 / 75.504 |      -0.56% |   +0.51% |
| grep_enumerated |    5772.887 / 5847.700 |    5800.058 / 5890.568 |      +0.47% |   +0.73% |
| rg_stem         |        75.157 / 76.310 |        74.987 / 77.013 |      -0.23% |   +0.92% |
| grep_stem       |    1939.514 / 1960.130 |    1935.486 / 2001.413 |      -0.21% |   +2.11% |

kfind any/smart 중앙값은 각각 0.23%·0.70% 감소했다. p95는 각각 6.28%·1.25% 증가했다.
중앙값에서 처리량 회귀는 보이지 않지만 tail latency의 불리한 변화는 남아 있다.
이 표는 파일 검색 전체 비용이며 국소 형태 분석기의 처리량이나 RSS 측정을 대신하지 않는다.
외부 명령은 같은 환경의 참고 행이며 입력 정규식의 의미가 달라 kfind와 속도만으로 순위를 매기지 않는다.

## 고정 fixture 품질

112개 constructed case의 기준·후보 raw와 contract-adjusted 결과는 모두 같다.
아래는 공통 값이며 기존 contract review를 유지했다. 정답 계약을 성능 수치에 적용하지 않는다.

| method           | raw TP / FP / TN / FN |     raw P / R / F1 (%) | adjusted TP / FP / TN / FN |  adjusted P / R / F1 (%) |
| ---------------- | --------------------: | ---------------------: | -------------------------: | -----------------------: |
| kfind_any        |       56 / 9 / 47 / 0 | 86.15 / 100.00 / 92.56 |            62 / 3 / 47 / 0 |   95.38 / 100.00 / 97.64 |
| kfind_smart      |       56 / 6 / 50 / 0 | 90.32 / 100.00 / 94.92 |            62 / 0 / 50 / 0 | 100.00 / 100.00 / 100.00 |
| regex_enumerated |       50 / 8 / 48 / 6 |  86.21 / 89.29 / 87.72 |            55 / 3 / 47 / 7 |    94.83 / 88.71 / 91.67 |
| regex_stem       |     46 / 34 / 22 / 10 |  57.50 / 82.14 / 67.65 |          52 / 28 / 22 / 10 |    65.00 / 83.87 / 73.24 |

배포용 resource를 사용한 별도 `verify-gold`는 자동 품사 coverage를 포함한 590개 case와 걷다·걸다 stress를 통과했다.

## 재현

각 revision의 binary를 같은 설정으로 빌드한다. 기준 binary를 별도 경로에 보존하고 순차 실행한다.

```sh
cargo build --release --locked -p kfind-cli --bin kfind
KFIND_SEARCH_BASELINE_SKIP_BUILD=1 \
KFIND_SEARCH_BASELINE_KFIND_BIN="$PWD/target/review-baseline/kfind" \
KFIND_SEARCH_BASELINE_REVISION=bae0a005 \
KFIND_SEARCH_BASELINE_RUNS=10 \
  scripts/benchmark-search-baseline.sh target/benchmark/search-trust/baseline
KFIND_SEARCH_BASELINE_SKIP_BUILD=1 \
KFIND_SEARCH_BASELINE_KFIND_BIN="$PWD/target/release/kfind" \
KFIND_SEARCH_BASELINE_REVISION=0768f87 \
KFIND_SEARCH_BASELINE_RUNS=10 \
  scripts/benchmark-search-baseline.sh target/benchmark/search-trust/candidate
cargo run --release --locked -p kfind-testkit --bin verify-gold -- \
  target/full-pos/lexicon.bin target/component-resource/morphology-component-compact.kfc
```
