# 토큰 경계 hot path 최적화

- 측정일: 2026-07-18
- 최신 `origin/main` 및 기준 revision:
  `f9cc1a075fc3eeb8701960ea336b5bd1d82cb346`
- 후보 코드 revision: `b91c1bfdaac4c43db89c72ee0da12b0687b2fc12`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1
- canonical fixture:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- explicit-POS matrix:
  `e862d8af010c23462ba3a9ebf4f1134275b68de5004bc60035565734f5f19999`
- Robust explicit-POS fixture:
  `6bfa1c00d1d4469742d100099eab3a4d6d0d679d2ed147cda1ca2e980a64e282`
- 100 MiB corpus:
  `7692072cb7bff9261c1fa5933bde41b27e558170818eeac6d07cabdd673815ff`
- fresh 기준 morphology report SHA-256:
  `8df8ad9b0225e4bc6a5984d0cc6be80d5c0cd4c04e55ab700790562a2a2d40e4`
- 후보 확인 morphology report SHA-256:
  `3b4706d54b32e27969c6c101b49da29bef0739332be9b867c93560cd544cd716`
- paired 후보 morphology report SHA-256:
  `14594fa06b65f3c9f99f865409dacf450b3406d0cfbba73f1f9f5af7fa37ae48`

## 결론

한 entry의 최근 준비 문맥을 재사용하는 cache 후보는 채택하지 않았다. 동일 token과 공백만
반복하는 입력에서 p50이 46.71% 줄었지만, 연속 cache miss를 만드는 교대 공백 입력에서는
p50 1.29%, p95 2.88%가 악화됐다. 실제 morphology fixture의 hit는 대부분 0건이었고 관측된
최고 hit율도 0.30% 미만이었다. 반복 입력의 사실상 연속 hit가 제품 workload를 대표하지
않는 것으로 판정했다.

대신 토큰 문자 분류와 인접 UTF-8 scalar 해독의 불필요한 일반 경로를 줄였다. Cache를 쓰지
않는 최종 후보는 같은 반복 입력에서 p50 26.76%, p95 26.24%, 교대 입력에서도 p50 26.87%,
p95 25.89%를 줄였다. Unicode와 잘못된 UTF-8의 기존 동작은 property test와 전체 품질
대조로 유지했다.

## Profile과 변경

기준 Time Profiler에서 `PreparedStructuralContextAnalysis::extract`의 inclusive sample은
43.7%, `execute_program_without_decision`은 33.4%, `ParticleVerifier::verify_prefix`는
13.1%였다. Unicode alphabetic 판정과 UTF-8 slice 검증의 exclusive sampled time은 각각
4,014 ms, 3,004 ms였다.

최종 변경은 다음 경로만 줄인다.

- `_`, ASCII 영숫자와 문자로 모두 할당된 한글 완성형 음절은 일반 Unicode 범주 조회 전에
  판정한다.
- 나머지 문자는 기존 Unicode 영숫자·mark 범주 판정으로 fallback한다.
- 이전 scalar는 continuation byte를 한 번 역주행하고, 다음 scalar는 선두 byte의 폭을 계산해
  정확히 한 scalar인지 한 번 검증한다.
- `unsafe`와 전역 상태를 추가하지 않는다.

임의의 `char`에서 기존 토큰 분류기와 결과가 같은지, 최대 511 byte의 임의 byte열과 범위 밖
offset에서 이전·다음 scalar 해독 결과가 기존 구현과 같은지 property test로 검증했다.
잘못된 UTF-8은 계속 토큰 경계로 취급한다.

## Cache 샘플 편중 평가

Cache prototype은 exact raw context byte가 직전 entry와 같을 때만 준비된 구조 분석을
재사용했다. `매일 ` 16,384회 반복 입력은 첫 lookup 뒤 같은 문맥이 연속해 사실상 hit 전용
샘플이다. 비교군은 공백을 1 byte와 2 byte로 교대해 직전 entry가 매번 빗나가게 했다.

| workload | 기준 p50 / p95 | cache prototype p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 반복 문맥 | 18.1906 / 18.3960 ms | 9.6938 / 10.0074 ms | -46.71% / -45.60% |
| 교대 문맥 | 18.7801 / 18.9896 ms | 19.0229 / 19.5371 ms | +1.29% / +2.88% |

실제 runner에 hit/miss counter를 임시로 넣어 측정한 뒤 instrumentation과 prototype을 모두
제거했다. 분모는 준비 문맥 cache lookup 수다.

| fixture / profile | hit / lookup | hit율 |
| --- | ---: | ---: |
| canonical embedded | 0 / 323 | 0% |
| canonical full-POS | 0 / 537 | 0% |
| development embedded | 0 / 345 | 0% |
| development full-POS | 0 / 561 | 0% |
| hard-negative embedded / full-POS | 0 / 24, 0 / 32 | 0% |
| test matrix `find_all`, embedded | 2 / 862 | 0.23% |
| test matrix `find_all`, full-POS | 3 / 1,399 | 0.21% |
| development matrix `find_all`, full-POS | 4 / 1,367 | 0.29% |

Matrix의 `find_at`과 Robust workload에서는 hit가 없었다. 반복 문맥만으로 cache 이득을
일반화할 수 없고 miss 비용도 증가하므로 제품 변경에서 제외했다.

## Criterion 측정

두 workload는 2초 warm-up, 20초 측정, 20 sample로 실행했다. 표는 sample별 1회 시간을
정규화한 p50/p95다. 기준의 교대 benchmark는 측정 함수만 임시로 더한 동일 `f9cc1a0`
제품 코드에서 실행했다.

- 반복 입력: 114,688 byte,
  `9e5c3adc0037cca693d6b36db94cb759ebabac548e4b5c9103e31875ebb26035`
- 교대 입력: 122,880 byte,
  `b141deb68516b9f9a8c6b1dbb8bdba9225900b9b301b6239355e03616cc7e355`

| workload | 기준 p50 / p95 | 최종 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `context_repeated_long_line` | 18.1906 / 18.3960 ms | 13.3225 / 13.5693 ms | -26.76% / -26.24% |
| `context_alternating_spacing_long_line` | 18.7801 / 18.9896 ms | 13.7348 / 14.0741 ms | -26.87% / -25.89% |

Sample JSON SHA-256은 반복 기준/후보가 각각
`80b6dc213a7494d3bf612b20890747253255afaf2a050b5546041ea4d1981244`,
`3a5173bf6f08a0e1fed4bef0ead95ffe5f3c6102dd5aa6b577d6aeea9d5eb393`,
교대 기준/후보가 각각
`c0f1e577fd72a00889dd15ea0a9e57b99f49cd812d64d9686cd3088b5b91c140`,
`b03057f4243665910fd0568633fb37aa864275bebf1beb55b069ef87ea3d8e28`다.

## 제품 workload

Fresh process warm-up 1회 뒤 5회 측정했다. 같은 시간대에 후보와 기준을 교차 재측정하고,
기준 worktree가 exact revision인지 다시 확인한 뒤 clean 기준을 한 번 더 측정했다. 표는 clean
기준과 마지막 paired 후보의 `median [min, max]`다.

| workload | cases/s 기준 → 후보 | p95 기준 → 후보 |
| --- | ---: | ---: |
| canonical embedded | 33,121.2 [30,205.7, 33,424.4] → 32,130.4 [23,799.2, 33,714.9] (-2.99%) | 0.0602 [0.0595, 0.0653] → 0.0628 [0.0602, 0.0840] ms (+4.32%) |
| canonical full-POS | 20,061.7 [19,595.6, 20,336.5] → 19,497.9 [11,631.6, 20,398.9] (-2.81%) | 0.1306 [0.1296, 0.1325] → 0.1349 [0.1285, 0.2359] ms (+3.29%) |
| Agent | 53,373.9 [53,123.7, 53,583.5] → 53,237.3 [51,302.6, 53,688.3] (-0.26%) | 0.0498 [0.0496, 0.0503] → 0.0504 [0.0500, 0.0522] ms (+1.20%) |
| Human | 18,396.3 [17,861.0, 18,445.6] → 18,580.0 [16,571.6, 18,605.0] (+1.00%) | 0.1385 [0.1371, 0.1430] → 0.1374 [0.1346, 0.1536] ms (-0.79%) |
| matrix Agent | 52,591.8 [52,355.7, 52,731.9] → 52,537.8 [51,276.0, 52,885.4] (-0.10%) | 0.0499 [0.0498, 0.0505] → 0.0499 [0.0494, 0.0515] ms (0.00%) |
| matrix Human | 17,811.1 [17,378.8, 18,178.2] → 17,020.7 [16,705.2, 17,842.9] (-4.44%) | 0.1462 [0.1435, 0.1505] → 0.1505 [0.1448, 0.1561] ms (+2.94%) |
| Robust Agent | 52,748.2 [50,545.3, 54,001.3] → 54,595.1 [53,107.2, 54,695.9] (+3.50%) | 0.0504 [0.0489, 0.0533] → 0.0482 [0.0476, 0.0498] ms (-4.37%) |
| Robust Human | 15,137.1 [14,993.3, 15,586.7] → 16,563.1 [15,286.7, 16,808.7] (+9.42%) | 0.1604 [0.1550, 0.1633] → 0.1467 [0.1429, 0.1630] ms (-8.54%) |

마지막 후보의 canonical full-POS 한 process에서 initialization과 p95가 함께 길어진 outlier가
있었다. 앞선 후보 확인 report의 중앙값은 20,102.4 cases/s, 0.1299 ms였고 fresh 기준과의
차이는 각각 +0.20%, -0.54%다. 제품 성능 향상은 Criterion의 큰 반복 입력에만 주장하며,
일반 morphology 경로는 회귀 없음으로 판정한다.

100 MiB CLI Agent wall은 0.018125초에서 0.018499초(+2.06%), Human wall은
0.075347초에서 0.076860초(+2.01%)였다. 처리량은 각각 -2.02%, -1.97%이고 모든 범위가
겹쳤다. Canonical embedded/full-POS RSS는 각각 0.00%, -0.11%였다.

## 품질과 fuzzing

기준·후보의 canonical, development, hard-negative, query matrix, Robust, Agent/Human
workflow에서 모든 confusion matrix, contract-adjusted 품질, case-level failure와 shadow
진단이 같다.

| workload | profile | TP / FP / TN / FN |
| --- | --- | ---: |
| canonical | embedded | 461 / 1 / 499 / 39 |
| canonical | full-POS | 498 / 2 / 498 / 2 |
| Agent | embedded + any + explicit POS | 486 / 7 / 493 / 14 |
| Human | full-POS + smart + untagged | 495 / 3 / 497 / 5 |
| test matrix Agent | embedded + any + explicit POS | 1,268 / 24 / 1,272 / 28 |
| test matrix Human | full-POS + smart + untagged | 1,286 / 6 / 1,290 / 10 |

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초,
RSS 상한 2 GiB로 8개 target을 실행했다. 총 4,396,997개 입력에서 crash, panic, timeout과
RSS 초과는 0건이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 1,037,224 | 552 MiB |
| `matcher_bytes` | 25,172 | 432 MiB |
| `matcher_plan` | 227,808 | 531 MiB |
| `user_lexicon` | 781,782 | 659 MiB |
| `json_output` | 346,661 | 562 MiB |
| `binary_detection` | 442,172 | 422 MiB |
| `pos_resource` | 1,419,836 | 592 MiB |
| `component_resource` | 116,342 | 406 MiB |

Stable formatter, workspace Clippy `-D warnings`, workspace test, fuzz manifest build, Rust 1.97
all-target build, WASM target build, morphology Python 77 tests와 README guard도 통과했다.

## 재현

```console
git switch --detach f9cc1a075fc3eeb8701960ea336b5bd1d82cb346
scripts/benchmark-criterion.sh context_repeated_long_line \
  --warm-up-time 2 --measurement-time 20 --sample-size 20
git restore --source b91c1bfdaac4c43db89c72ee0da12b0687b2fc12 -- \
  crates/kfind-testkit/benches/query_matcher.rs
scripts/benchmark-criterion.sh context_alternating_spacing_long_line \
  --warm-up-time 2 --measurement-time 20 --sample-size 20
git restore crates/kfind-testkit/benches/query_matcher.rs
KFIND_MORPH_RUNS=5 \
KFIND_MORPH_IMAGE=kfind-morph-benchmark:boundary-hotpath-clean-baseline \
scripts/benchmark-morphology.sh target/morph-boundary-hotpath-clean-baseline

git switch --detach b91c1bfdaac4c43db89c72ee0da12b0687b2fc12
scripts/benchmark-criterion.sh context_repeated_long_line \
  --warm-up-time 2 --measurement-time 20 --sample-size 20
scripts/benchmark-criterion.sh context_alternating_spacing_long_line \
  --warm-up-time 2 --measurement-time 20 --sample-size 20
KFIND_MORPH_RUNS=5 \
KFIND_MORPH_IMAGE=kfind-morph-benchmark:boundary-hotpath-paired-candidate \
scripts/benchmark-morphology.sh target/morph-boundary-hotpath-paired-candidate

KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
