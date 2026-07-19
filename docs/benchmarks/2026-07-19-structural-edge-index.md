# 구조 edge 시작 위치 index

- 측정일: 2026-07-19
- 기준 코드 revision: `f2525b564d973ae95918b7732549b9f1bb463650`
- 후보 코드 revision: `2c2b15af3872d2a33915799fc2ec1c42daeaa937`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

구조 분석에서 최대 4,096개의 형태 edge를 여러 상태 머신이 byte 위치마다 처음부터 다시
검색하던 흐름을 하나의 시작 위치 인접 index로 바꿨다. 형태소 resource에서 edge를 수집할 때
한 번만 index를 만들고 모든 구조 판정이 공유한다. 준비가 끝나면 index도 폐기되므로 matcher나
전역 상태에 cache를 추가하지 않는다.

node 상한에 가까운 4,032-edge 입력의 준비 시간은 p50 83.16%, p95 83.06% 줄었다. 짧은 일반
문맥도 p50 2.86%, p95 3.21% 줄었다. Canonical morphology의 full-POS는 처리량 2.68%, p95
1.69% 개선됐지만 embedded는 처리량 3.91%, p95 1.31% 나빠졌다. 각 측정 범위는 겹치며 품질
projection은 완전히 같았다.

밀집 benchmark는 매 iteration마다 graph와 index를 새로 준비한다. 따라서 반복 표본의 cache
hit나 warm graph 재사용은 측정값에 들어가지 않는다. 개선 근거는 cache hit율이 아니라 반복
전체 탐색을 시작 위치 인접 조회로 바꾼 알고리즘 복잡도 감소다.

## 구조

기존 edge는 시작 위치순 `Vec`에 저장됐지만 소비자는 그 정렬 성질을 공통 자료구조로 표현하지
않았다. 상태 머신마다 다음 패턴을 반복해 edge 수를 `E`, token byte 길이를 `B`, 상태 머신 수를
`S`라고 할 때 시작 위치 조회만 대략 `O(S × B × E)`였다.

```text
각 구조 상태 머신
  → byte 위치를 순회
  → 전체 edge Vec에서 같은 시작 위치를 filter
```

변경 뒤 `EdgeGraph`가 edge와 `start_offsets`를 함께 소유한다.

```text
형태 edge 수집
  → 시작 위치순 edge Vec
  → byte 위치별 반열림 edge 범위 생성

각 구조 상태 머신
  → start_offsets[position]..start_offsets[position + 1]
  → 해당 위치의 edge만 순회
```

Index 생성은 `O(B + E)`, 각 위치 조회는 `O(1)`이다. Edge의 순서와 판정 함수는 유지했다.
Resource가 돌려준 span이 token을 벗어나거나 node 상한을 넘는 기존 실패도 그대로 보존한다.

## Criterion 측정

양쪽 revision에 동일한 benchmark source를 적용했다. Benchmark source SHA-256은
`5ee846e969a35debc3afec5fba69f1f06347d2e9a9f9ffcd23972a4fff52829f`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을
정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 4,032-edge graph 준비 | 3.1449 / 3.2461 ms | 0.5297 / 0.5500 ms | -83.16% / -83.06% |
| 짧은 구조 문맥 판정 | 4.1604 / 4.3081 µs | 4.0415 / 4.1699 µs | -2.86% / -3.21% |

밀집 입력은 `가` 63개로 된 189-byte token과 모든 접두 surface의 `NNG`, `VV+EC` 분석으로
구성한다. Node 상한 4,096 바로 아래인 4,032개 edge를 만든다. Token SHA-256은
`177942e0f9378bea35e9e362da0c26fdbceb68fb3732419f6d23c42cc1e945ae`다. 짧은 문맥은 고정된
`매일 보고`를 사용한다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 4,032-edge graph 준비 | `1b5e7b87bce88a27e6746ea9642339163c56a8dd29043ba412fc1648ca4103e8` | `cb45df4be323da7b850899c9554de761a4556697600826722e51d6199a542717` |
| 짧은 구조 문맥 판정 | `de727621569eaf758577ab83a86c5a818f4535e19c12386e82b28924cad45fb7` | `e32b10d79813e2b07a0ee9562ab3e5a921d0ad3ae3cf7db97c5a2ff803c5134` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.041325 [0.039969, 0.042245] s | 0.041674 [0.040393, 0.044852] s | +0.84% |
| embedded | cases/s | 32,908.0 [29,029.9, 33,642.8] | 31,621.0 [30,835.1, 32,949.1] | -3.91% |
| embedded | p95 | 0.0609 [0.0592, 0.0686] ms | 0.0617 [0.0595, 0.0646] ms | +1.31% |
| embedded | RSS | 42,276 [42,272, 42,284] KiB | 42,276 [42,272, 42,284] KiB | 0.00% |
| full-POS | initialization | 0.077334 [0.074385, 0.079258] s | 0.073833 [0.073213, 0.075357] s | -4.53% |
| full-POS | cases/s | 20,191.1 [18,185.0, 20,385.2] | 20,732.2 [19,621.8, 20,818.1] | +2.68% |
| full-POS | p95 | 0.1300 [0.1280, 0.1422] ms | 0.1278 [0.1265, 0.1325] ms | -1.69% |
| full-POS | RSS | 57,976 [57,792, 58,040] KiB | 57,852 [57,852, 57,936] KiB | -0.21% |

Canonical, development, hard-negative, query matrix, Robust, shadow와 Agent/Human workflow에서
성능·환경 필드를 제외한 품질 projection은 기준과 후보가 byte 단위로 같았다. Projection
SHA-256은 양쪽 모두
`d9f93ad0d81f787e0da2ef04e9093b30c8de5025756f58714e8ee6371267ee12`다. 기준/후보 morphology
report SHA-256은 각각
`a61cae687bd60314ae42e8068aa5725884dff8c594eb6bda6da5eab7b179e1e0`,
`21cebd5f0287b1282ca78fe0001194a4dcade3fd2c9cc74d15b0aa56aec4dd02`다.

Embedded의 불리한 변화와 full-POS의 유리한 변화 모두 실행 간 범위가 겹친다. 밀집 graph의
큰 개선을 일반 morphology 처리량의 같은 폭 개선으로 해석하지 않는다.

## 정확성과 안전성

임의의 정렬된 edge graph에 대해 모든 byte 위치의 index 결과를 기존 선형 filter 결과와
비교하는 property test를 추가했다. 최대 256-byte token과 512개 edge를 생성하며 범위를 벗어난
조회가 빈 결과를 내는지도 고정한다. 실제 component resource로 만든 graph의 모든 위치도 같은
reference와 비교한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 총 4,300,537개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다. `slowest_unit_time_sec`는 모든 target에서 0이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 997,577 | 568 MiB |
| `matcher_bytes` | 29,724 | 451 MiB |
| `matcher_plan` | 214,956 | 530 MiB |
| `user_lexicon` | 747,889 | 645 MiB |
| `json_output` | 327,080 | 583 MiB |
| `binary_detection` | 59,432 | 487 MiB |
| `pos_resource` | 1,485,162 | 571 MiB |
| `component_resource` | 105,688 | 406 MiB |
| `search_executor` | 88,535 | 473 MiB |
| `structural_preparation` | 244,494 | 582 MiB |

## 재현

기준 worktree에는 benchmark와 문서 계약만 적용하고 제품 코드는 최신 `origin/main` 그대로
측정했다.

```console
git switch --detach f2525b564d973ae95918b7732549b9f1bb463650
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-edge-index-baseline

git switch --detach 2c2b15af3872d2a33915799fc2ec1c42daeaa937
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-edge-index-candidate
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
