# matcher 줄 평가 파이프라인 재설계

- 측정일: 2026-07-18
- 기준 revision: `71f65148a0b7f8f91a352d97b1551cda790d51e6`
- 후보 코드 revision: `86dd7ecf1253accc0c6b5c69a8f33a4a606cbc76`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

매칭된 줄의 span 확인과 metadata 출력을 별도 검색으로 실행하던 흐름을 한 번의 줄 평가와
one-shot sink 전달로 합쳤다. Phrase 결합은 successor 비교 중 원문 endpoint를 반복 탐색하지
않도록 모든 candidate endpoint의 byte·Unicode scalar·line-break 누적값을 한 번 계산한다.

반복 phrase의 결합 p50은 49.25%, 실제 metadata 줄 검색은 69.34% 줄었다. Metadata가 없는
존재 판정도 60.73% 줄어 전달 상태가 summary 경로를 악화시키지 않았다. 일반 phrase 입력은
p50 3.45% 개선됐다.

전체 줄을 token IR로 바꾸는 설계는 한 줄 크기에 비례하는 추가 메모리와 공격 표면 때문에
채택하지 않았다. Query plan을 완전히 평탄화하는 변경도 현재 profile에서 근거가 없었다. 현재
구조는 raw anchor 후보를 유지하면서 줄 안의 검증 결과만 일괄 계산하므로 큰 입력의 streaming
계약과 matcher의 bounded window를 보존한다.

## Profile과 구조

기준 `scan_deterministic_corpus`의 sampled leaf time 97.1%는 Aho-Corasick overlapping scan이었다.
희소 단일 atom 입력에서 anchor scan을 대체할 구조적 근거는 없었다.

반복 phrase의 기준 profile에서는 atom span 수집 53%, phrase 선택 38%였고, 조밀한 실제 줄 검색은
phrase 선택 81.6%, compatible successor 계산 67.4%, nested endpoint lookup의 partition point가
61.6%였다. `grep-searcher`는 후보 줄 확인에 span-only matcher를 실행한 뒤 sink에서 같은 줄의
metadata를 다시 계산했다.

변경 뒤 데이터 흐름은 다음과 같다.

```text
raw anchor가 있는 줄
  → atom span과 provenance를 한 번 수집
  → candidate endpoint 누적값을 한 번 계산
  → bounded suffix DP로 non-overlapping match 선택
  → 첫 span은 grep-searcher에, 전체 결과는 sink에 one-shot 전달
```

One-shot 상태는 재진입 가능한 cache가 아니다. 줄 길이와 pending 결과가 호출 순서와 맞지 않으면
`RefCell` panic 대신 관측 가능한 I/O 오류를 반환한다. Multi-line query는 줄 단위 handoff를 쓰지
않고 기존 buffer 안전 경로를 유지한다. 줄당 metadata 결과는 기존 65,536개 상한을 그대로 적용한다.

## Criterion 측정

양쪽 revision을 같은 기기와 기본 Criterion 설정으로 실행했다. 각 workload는 3초 warm-up 뒤
100 sample을 측정했다. 표는 sample별 1회 시간을 정렬한 nearest-rank p50/p95다.

- 반복 phrase 입력: 384 byte,
  `d52a8bc70bef97ac9c43f989776b3288d2b117525d129e9e9d23ace578efd7c1`
- 줄 검색 입력: 24,577 byte,
  `2ee6a67f3b3ba05ba0c4ff04a03cf53d7b13bf694ec17176a6abe7153b82fe87`

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `phrase_find_all` | 814.42 / 845.27 µs | 786.35 / 817.62 µs | -3.45% / -3.27% |
| `phrase_find_all_repeated` | 245.03 / 253.70 µs | 124.34 / 128.54 µs | -49.25% / -49.33% |
| `phrase_input_searcher_repeated_line` | 5.4709 / 5.7015 ms | 1.6776 / 1.7397 ms | -69.34% / -69.49% |
| `phrase_input_searcher_repeated_line_exists` | 2.3605 / 2.4455 ms | 0.9269 / 0.9605 ms | -60.73% / -60.72% |

Sample JSON SHA-256은 일반 phrase 기준/후보가
`a984747e6bc9dba8c7a2865a7af82a46bace1d0935335856001824eda86d443d`,
`d4521cbce25f86eb6c53e53b5b58805b1b5996bd4fc2cbdd40b727f07bcc7eeb`,
반복 phrase 기준/후보가
`baa40d38ad22d10d83b4d6727d0d2f96ce0cfa3063566a55dc6865308b7359cb`,
`94f2238873186195962e65b36d8c9553366be5e0b05aa6eb046672be0265a804`다.
줄 metadata 기준/후보는
`6fd7d4452d297a381209139ed3edb9a9311ffa825d698a655082f03684e6d3a4`,
`79748be9a45d3391e1b7f3226826e5fd05cdeffbf006bf77fcc1b0ebc2564bf2`,
존재 판정 기준/후보는
`fd4770fedfa3154039b54931e675787baa5fa6be9aff12c4a1dd5d4f27da2614`,
`8116974da6c872b946582e007bb2565d1fe958d0367d52ae52a46f5607a854cc`다.

## Cache 표본 편중

기존 반복·교대 문맥 benchmark는 각각 한두 종류의 raw context만 반복하므로 warm cache hit에
편중된다. Byte 수와 match 수가 같은 대조군을 추가했다.

- constant: 245,760 byte, 16,384 match,
  `dcbee0adff204c19234c69d6f49518e85c6552cbf8f11bda8a4aeeaa2ae8846c`
- unique: 245,760 byte, 16,384 match,
  `e5e74147ab26571ca929d4da880a9e80202a5c6054290a71797679bd60cebcf3`

| workload | p50 / p95 | constant 대비 |
| --- | ---: | ---: |
| `context_constant_neighbors_long_line` | 12.4478 / 12.5830 ms | 1.00x / 1.00x |
| `context_unique_neighbors_long_line` | 39.7618 / 41.3116 ms | 3.19x / 3.28x |

Sample JSON SHA-256은 각각
`0565d50f40c6b3f6d6d0143789ddc62617cc1457e941515b14791df85402fa53`,
`11c1ebae3d58bd2c94e6a1d944fdeeb43f33521314039332b886235ba7d8e7e4`다.
따라서 반복 입력의 수치만으로 cache 이득을 일반화하지 않는다.

내용 기반 준비 context cache를 제거하는 prototype도 측정했다. Criterion estimate는 constant가
약 12.5 ms에서 39.1 ms로 악화됐고 unique는 약 40.6 ms에서 40.0 ms로 실질 개선이 없었다.
전체 morphology 수치에서도 안정적인 miss-path 이득이 없었다. 반복 경로만 약 3배 악화시키므로
제거하지 않았다. Cache는 256개 상한과 원본 key 충돌 확인을 유지하되, 매칭 파이프라인 설계의
대체물로 사용하지 않는다.

## 제품 workload와 품질

Fresh process warm-up 1회 뒤 5회 측정했다. 후보를 먼저 측정하고 기준 직후 후보를 다시 확인했다.
표는 마지막 기준/후보의 `median [min, max]`다.

| workload | cases/s 기준 → 후보 | p95 기준 → 후보 |
| --- | ---: | ---: |
| canonical embedded | 33,774.5 [29,822.6, 33,933.0] → 31,118.4 [28,713.7, 33,078.4] | 0.0598 [0.0586, 0.0659] → 0.0641 [0.0608, 0.0680] ms |
| canonical full-POS | 20,432.1 [19,810.9, 20,474.1] → 20,170.9 [19,459.8, 20,584.3] | 0.1301 [0.1294, 0.1337] → 0.1304 [0.1293, 0.1349] ms |
| Agent | 53,116.0 [50,156.8, 53,550.4] → 53,288.8 [52,269.7, 53,495.8] | 0.0501 [0.0496, 0.0536] → 0.0503 [0.0495, 0.0509] ms |
| Human | 18,600.1 [18,018.6, 18,644.7] → 18,421.1 [18,038.2, 18,630.5] | 0.1380 [0.1352, 0.1392] → 0.1386 [0.1365, 0.1397] ms |

Canonical embedded 중앙값은 처리량 -7.86%, p95 +7.19%로 불리했으나 범위가 겹쳤다. 이
workload는 single atom만 실행해 변경된 phrase 선택과 line handoff를 호출하지 않는다. 첫 후보
측정도 30,724.9 cases/s였지만 Agent는 50,535.3에서 재측정 53,288.8로 회복했다. 다른 single-atom
profile과 전체 범위도 겹치므로 일반 morphology 회귀로 판정하지 않고 불리한 수치를 보존한다.

100 MiB CLI의 마지막 Agent wall은 0.017134초에서 0.018736초(+9.35%), Human은
0.075956초에서 0.080006초(+5.33%)였다. 앞선 후보 측정은 각각 0.017602초(+2.73%),
0.074603초(-1.78%)였고 실행 범위와 방향이 일치하지 않았다. 한 matching line의 개선보다
100 MiB scan 변동이 큰 workload이므로 전체 CLI 개선은 주장하지 않는다.

기준·후보의 canonical, development, hard-negative, query matrix, Robust, Agent/Human workflow의
모든 품질 projection은 같다. 주요 confusion matrix는 다음과 같다.

| workload | profile | TP / FP / TN / FN |
| --- | --- | ---: |
| canonical | embedded | 461 / 1 / 499 / 39 |
| canonical | full-POS | 498 / 2 / 498 / 2 |
| Agent | embedded + any + explicit POS | 486 / 7 / 493 / 14 |
| Human | full-POS + smart + untagged | 495 / 3 / 497 / 5 |

기준과 마지막 후보 morphology report SHA-256은 각각
`5ee0ed0f283e1102f7efe80d3290574b60d07aa126d282223ea83c9bb76e5033`,
`ce999e4d191481cae428bdf3c248c21d5725649e1af534667dff38f1d5ec25e0`다.

## 안전성 검증

Line handoff는 정상 소비 뒤 두 번째 소비와 상태 불일치를 오류로 반환하는 단위 테스트를 추가했다.
`binary_detection` fuzz target은 single atom과 phrase matcher, metadata와 summary 경로를 같은 입력에서
실행하고 matching line 수와 match span 범위를 대조한다. NUL은 UTF-8 경계에 삽입한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 8개 target을 실행했다. 총 4,338,666개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 1,051,333 | 564 MiB |
| `matcher_bytes` | 38,948 | 447 MiB |
| `matcher_plan` | 211,516 | 532 MiB |
| `user_lexicon` | 763,555 | 662 MiB |
| `json_output` | 348,540 | 552 MiB |
| `binary_detection` | 57,864 | 499 MiB |
| `pos_resource` | 1,745,176 | 610 MiB |
| `component_resource` | 121,734 | 404 MiB |

## 재현

```console
git switch --detach 71f65148a0b7f8f91a352d97b1551cda790d51e6
scripts/benchmark-criterion.sh 'matcher/phrase_'
scripts/benchmark-morphology.sh target/morph-pipeline-baseline-final

git switch --detach 86dd7ecf1253accc0c6b5c69a8f33a4a606cbc76
scripts/benchmark-criterion.sh 'matcher/phrase_'
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-morphology.sh target/morph-pipeline-final-confirm
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
