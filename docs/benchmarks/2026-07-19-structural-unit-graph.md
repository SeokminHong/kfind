# 구조 unit 시작 위치 graph

- 측정일: 2026-07-19
- 기준 코드 revision: `81c5fce4a6519624833184c261eb3c160ca37d30`
- 후보 코드 revision: `03f113ff1fd3cb10b404df99c4f5154055504855`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

준비된 `TokenEvidence`의 unit을 평면 `Vec` 대신 시작 위치 인접 graph로 표현했다. Edge와 unit이
공통 `StartGraph<T>`를 사용하며, 후보 판정의 최소 경로 계산은 해당 위치에서 시작하는 unit만
순회한다. 기존 후보별 임시 edge 생성·정렬·중복 제거와 start·end별 전체 unit 재검색은 제거했다.

Node 상한 바로 아래인 4,095-edge graph에서 서로 다른 16개 component 후보를 판정하는 비용은
p50 88.43%, p95 88.49% 줄었다. Graph 준비는 p50 0.38%, p95 0.26% 줄었다. 짧은 일반 판정은
p50 2.28%, p95 2.88% 느려졌다.

Canonical morphology는 embedded 처리량 +4.30%, p95 -4.74%, full-POS 처리량 +1.77%, p95
-1.89%였고 품질 projection은 완전히 같았다. RSS 중앙값은 두 profile 모두 4 KiB 줄었다.

## 구조

기존 `TokenEvidence`는 unit을 시작 위치순으로 정렬했지만 경로 함수는 그 성질을 사용하지 않았다.

```text
후보별 preferred path 판정
  → 전체 unit에서 대상 span을 골라 임시 edge Vec 생성
  → 정렬·중복 제거
  → byte 위치마다 전체 임시 edge를 start 기준 재검색
  → byte 위치마다 전체 임시 edge를 end 기준 재검색
```

변경 뒤 edge 수집과 준비된 unit이 같은 자료구조를 사용한다.

```text
StartGraph<T>
  → 시작 위치순 items
  → byte 위치별 반열림 item 범위

preferred path 판정
  → 시작 위치 범위만 순회해 prefix 최소 비용 계산
  → 같은 시작 위치 범위를 역순으로 순회해 suffix 최소 비용 계산
  → core가 최소 비용 경로에 속하는지 확인
```

Suffix 비용은 끝 위치 역색인을 추가하지 않고 DAG의 시작 위치를 역순으로 순회해 계산한다. 최소
unit 경로도 span의 절대 끝 위치가 아니라 span 길이에 비례한 배열을 사용한다. 후보별 임시 graph와
전역 cache는 없다.

준비된 token graph가 보존하는 추가 메모리는 64-bit 환경에서 `(token bytes + 2) × 8 + 16`
bytes다. 189-byte 밀집 token은 1,544 bytes가 늘어난다. 이 값은 `PreparedTokenGraph::memory_usage`에
포함되므로 matcher의 bounded memory 예약과 fallback 정책을 우회하지 않는다. Edge 준비용 index는
준비가 끝나면 폐기한다.

## Criterion과 표본 편중

양쪽 revision에 동일한 benchmark source를 적용했다. Benchmark source SHA-256은
`0ec95092ad1ff9e6df5fb27448be24cad2303e4d9d0f706aa323b956dfed5dfe`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을
정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 16개 밀집 preferred-path 후보 | 1.8225 / 1.8904 ms | 0.2108 / 0.2176 ms | -88.43% / -88.49% |
| 4,032-edge graph 준비 | 0.5526 / 0.5730 ms | 0.5505 / 0.5715 ms | -0.38% / -0.26% |
| 짧은 구조 문맥 판정 | 4.1943 / 4.3113 µs | 4.2899 / 4.4355 µs | +2.28% / +2.88% |

밀집 preferred-path workload는 `가` 63개의 모든 접두 surface에 `NNG`, `VV+EC`를 등록하고
단일 음절 `JX`를 추가해 4,095개 형태 edge와 다수의 동일 비용 명사 경로를 만든다. 준비된 graph
하나에서 길이가 서로 다른 16개 component span을 순환한다. Candidate 결과 cache나 mutable
memoization은 없으며 모든 후보가 실제 최소 경로 계산을 수행한다. Graph 준비 비용은 별도 workload가
매 iteration 새 graph를 만들어 측정한다. 따라서 반복 후보의 높은 cache hit율을 개선 근거로 쓰지
않는다. Token SHA-256은
`177942e0f9378bea35e9e362da0c26fdbceb68fb3732419f6d23c42cc1e945ae`다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 16개 밀집 preferred-path 후보 | `3ea7bd7ab263498396895e7722e32acd3db868af90b24b5792b8ef4f0744b696` | `d8986263b80a226b76144a3594f0e7c9710fef9d386b2d4aaf5fad6d09476e89` |
| 4,032-edge graph 준비 | `7061c905a336a0eb9af8abe9891a51fd048a162f91f9fe57535fc7090cf70e77` | `be627d1156f453cd21856603b9ae8de5902f86c575dff9ce05411f7a57a2b447` |
| 짧은 구조 문맥 판정 | `59b50623ab11182208c666c787dc3dd2dbc84c50f68b257a21eb77f15d37573c` | `06c70754b0d1e49dcc60d5b17093a8549472f1e886ce61d5327a577a822e365f` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.043794 [0.042263, 0.044153] s | 0.041036 [0.040042, 0.044941] s | -6.30% |
| embedded | cases/s | 31,441.4 [29,992.4, 32,606.0] | 32,792.0 [29,979.1, 34,254.9] | +4.30% |
| embedded | p95 | 0.0633 [0.0617, 0.0669] ms | 0.0603 [0.0593, 0.0659] ms | -4.74% |
| embedded | RSS | 42,276 [42,264, 42,280] KiB | 42,272 [42,260, 42,288] KiB | -0.01% |
| full-POS | initialization | 0.076742 [0.073246, 0.079246] s | 0.074501 [0.073895, 0.077610] s | -2.92% |
| full-POS | cases/s | 19,268.0 [17,402.2, 20,385.2] | 19,609.4 [18,230.0, 20,432.5] | +1.77% |
| full-POS | p95 | 0.1374 [0.1298, 0.1505] ms | 0.1348 [0.1291, 0.1454] ms | -1.89% |
| full-POS | RSS | 57,852 [57,840, 57,912] KiB | 57,848 [57,848, 57,912] KiB | -0.01% |

Canonical, development, hard-negative, query matrix, Robust, shadow와 Agent/Human workflow에서
성능·환경 필드를 제외한 품질 projection은 기준과 후보가 byte 단위로 같았다. Projection
SHA-256은 양쪽 모두
`7d5cc7dd21bb79bd3d01f55ad882792988b10fe95bead2917717b0dcee753543`다. 기준/후보 morphology
report SHA-256은 각각
`614f67cd8758db6a285556af70e19c448125fa83d6bdd94cc3112b7529ad2624`,
`a4ce8cf5226e6e2d2405ded0eae5f662d35404d6a593f9340fd1d0a5e46b18c1`다.

짧은 microbenchmark의 불리한 변화와 morphology의 유리한 변화 모두 실행 범위가 겹친다. 밀집
경로의 큰 개선을 모든 제품 입력에 같은 폭으로 일반화하지 않는다.

## 정확성과 안전성

최대 64-byte span과 256개 임의 unit으로 graph를 만들고, index 기반 최소 경로 길이와
preferred-path 포함 판정을 기존 선형 reference와 비교하는 property test를 추가했다. 중복 span,
source/runtime 비용, nominal/predicate 필터와 경로가 없는 입력을 함께 생성한다. 범위를 벗어난
`usize::MAX` 시작 위치는 panic 없이 빈 결과를 낸다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 총 4,555,786개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다. `slowest_unit_time_sec`는 모든 target에서 0이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 1,024,762 | 569 MiB |
| `matcher_bytes` | 30,590 | 446 MiB |
| `matcher_plan` | 222,874 | 524 MiB |
| `user_lexicon` | 765,707 | 647 MiB |
| `json_output` | 341,687 | 554 MiB |
| `binary_detection` | 56,778 | 485 MiB |
| `pos_resource` | 1,659,487 | 595 MiB |
| `component_resource` | 120,038 | 405 MiB |
| `search_executor` | 93,846 | 474 MiB |
| `structural_preparation` | 240,017 | 573 MiB |

## 재현

기준 worktree에는 benchmark와 문서 계약만 적용하고 제품 코드는 최신 `origin/main` 그대로
측정했다.

```console
git switch --detach 81c5fce4a6519624833184c261eb3c160ca37d30
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-unit-graph-baseline

git switch --detach 03f113ff1fd3cb10b404df99c4f5154055504855
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-unit-graph-candidate
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
