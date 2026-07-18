# 구조 선택 사실 계산

- 측정일: 2026-07-19
- 기준 코드 revision: `fe80f9aab0d9ddf26bde9fdd5d76b7b209f054d5`
- 후보 코드 revision: `fe9a06a36683b055c24ae9be600f062e05bd4d71`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

현재 token의 구조 선택이 이미 만든 형태 edge graph를 사용하지 않고 nominal host와 particle
suffix를 resource에서 다시 분해하던 경로를 없앴다. Graph를 만드는 동안 particle suffix 도달성,
nominal prefix 도달성, exact nominal 끝 위치를 계산해 `TokenEvidence`에 보존하고 선택과 후보
승인이 같은 사실을 사용한다.

20음절의 모든 접두 surface가 nominal과 particle 분석을 함께 갖는 입력에서 준비된 graph의 선택
비용은 p50/p95 99.560%/99.553%, graph 생성부터 선택까지는 64.469%/64.690% 단축됐다. Matcher
생성과 첫 구조 검색도 3.254%/2.951% 단축됐다. 전체 morphology는 embedded 처리량 +7.56%, p95
-3.08%, full-POS 처리량 +4.79%, p95 -3.03%였다. 품질 projection은 완전히 같았다.

전역 cache나 candidate 결과 memoization은 추가하지 않았다. 반복·고정 문맥은 p50
2.63~3.16% 느렸으며 그대로 기록한다. 같은 측정 구간의 변경되지 않은 suffix와 prepared-path
대조군도 후보 쪽으로 1.45~3.76% 이동했으므로 이 작은 폭은 변경의 회귀나 개선으로 일반화하지
않는다. 고유 현재 token과 고유 인접 token은 각각 17.36%, 22.76% 개선돼 반복 표본의 높은
cache hit율을 개선 근거로 사용하지 않았다.

## 구조

기존 선택 경로는 graph 준비 뒤에도 다음 작업을 독립적으로 수행했다.

```text
현재 token graph 생성
  → 모든 split에서 exact nominal host 재조회
  → 각 split의 particle suffix를 resource에서 다시 도달성 판정
  → fallback에서 각 nominal prefix를 다시 완성 경로 판정
  → 선택 결과에 particle host 목록을 복사해 후보 승인에서 사용
```

변경 뒤에는 graph의 시작 위치 index를 세 번 순회한다.

```text
EdgeGraph
  → 뒤에서 앞으로 particle suffix 도달성
  → 앞에서 뒤로 nominal prefix 도달성(has_nominal 상태 포함)
  → 시작 위치의 exact nominal 끝 위치
  → NominalPathFacts를 TokenEvidence에 보존
  → 선택과 후보 승인이 같은 사실을 참조
```

임시 도달성 배열은 token byte 길이에 비례하고 계산 뒤 폐기한다. 영구 추가 메모리는 가능한
nominal-particle host의 `Range<usize>` 목록뿐이며 `TokenEvidence::memory_usage`에 포함된다. Particle
edge가 없으면 배열을 만들지 않고 빈 사실을 반환한다. 기존 직접 resource 탐색 함수는 graph가 없는
인접 token과 공개 독립 판정 경로를 위해 남긴다.

## Criterion과 표본 편중

양쪽 revision에 동일한 benchmark source를 적용했다. Benchmark source SHA-256은
`bfb4d72841b3acff6f18121749104610c353ad71f4032094e79ac95fadaa4819`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을
정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 준비된 graph의 nominal-particle 선택 | 88.7306 / 90.0548 µs | 0.3902 / 0.4027 µs | -99.560% / -99.553% |
| dense graph 생성 + 선택 | 138.7784 / 144.0286 µs | 49.3099 / 50.8569 µs | -64.469% / -64.690% |
| matcher build + 첫 구조 검색 | 3.4698 / 3.5840 µs | 3.3569 / 3.4782 µs | -3.254% / -2.951% |
| 짧은 구조 문맥 판정 | 4.2614 / 4.3979 µs | 4.0044 / 4.0502 µs | -6.029% / -7.907% |
| 4,032-edge graph 준비 | 0.5500 / 0.5706 ms | 0.5568 / 0.5798 ms | +1.226% / +1.619% |
| 16개 prepared preferred-path 후보 | 0.2100 / 0.2122 ms | 0.2144 / 0.2153 ms | +2.050% / +1.452% |
| particle suffix 12회 거부 | 4.3149 / 4.4502 µs | 4.4595 / 4.5970 µs | +3.350% / +3.297% |
| particle suffix 20회 거부 | 10.9316 / 11.2864 µs | 11.3431 / 11.7005 µs | +3.764% / +3.669% |

Dense 선택 입력은 `나` 20개이며 모든 접두 surface에 `NNG`와 `JX` 분석을 등록한다. 선택-only
workload는 immutable graph를 공유하지만 결과 cache와 mutable memoization 없이 매 iteration
사실 선택을 수행한다. 생성+선택 workload는 매 iteration graph와 사실을 새로 만든다. 두 결과를
함께 사용해 graph 공유 표본의 편중을 피한다. 입력 SHA-256은
`cd73dcd03891f218e0c807e7918fc4d290fa048b036f5a04170a7e3fdf632bb4`다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 준비된 graph의 nominal-particle 선택 | `994280dab686a2277b2a0906b38ed044febea9061e7f4813d7589b75448b077e` | `63e8342e8196a8d95fbe4d42a5184860d8c1aa7458e426eaa625a0b7c6d1ccc0` |
| dense graph 생성 + 선택 | `a02c8b7c9615b2d68e9e44565a5a6377901d2b807ccf337c0e5e09cef0bb9d49` | `c08d9c3317972f2c008533e2b5993defcc2dfd0f2995c94e105ed757b82d6d4a` |
| matcher build + 첫 구조 검색 | `26b447a06589e9ea90ee2c524b826528a0f74021ba05e00934e7438501dbc6b4` | `223af21dc683a4af4ae5a97c8d2991f98ce690b522ddc81c42370e88ca7cd5e9` |
| 짧은 구조 문맥 판정 | `32f7c3c49516e53c7279450e9bdf6c1189e016fd0dc143b3bd95b96a83927d48` | `1797c218f4b66af4b76f87c68d4a2a052faeb0b1aea4800a33983fca4dadd27a` |
| 4,032-edge graph 준비 | `455a5fc3204c66efa9e223e30f58d31083d26e2210cbf447168e129e393c92ed` | `a4bb890ba76bf43959a3fc823cff485fd8ef87ea2fb31a7260a5dfdf6eadd908` |
| 16개 prepared preferred-path 후보 | `19e96768881184d66de2123075bb8c5349f04760433ea447b809cffa3e4f8ed7` | `343c422f07189bdb30ba05ae299cf82ebbf41c872c641014f916c315b25c53c6` |
| particle suffix 12회 거부 | `69ad375d8f87d0240a8d4f55234317d299b592c04ee612a52ba5fa28cc2f2d7b` | `c538330d7a58be9366d08b7de4eb035d42623c7b884f1f696ad2b5e4076e5c12` |
| particle suffix 20회 거부 | `72d0f39c9d30fe1256fea4482c7ec1dfe255c6970e8196bbe48d077ff4acbe80` | `570d89f079fc324b77181a4c5697b8219bf0473f2531a91e0bc8ac02d2744d4b` |

### 문맥 분포

기존 cache와 현재 token 재사용이 유리한 입력, 재사용이 불가능한 입력을 같은 benchmark source에서
분리했다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 반복 문맥 | 13.2104 / 13.6877 ms | 13.6274 / 13.8577 ms | +3.156% / +1.242% |
| 고정 인접 token | 12.9653 / 13.4336 ms | 13.3404 / 13.6893 ms | +2.893% / +1.903% |
| 교대 공백 문맥 | 13.6517 / 14.1216 ms | 14.0103 / 14.4654 ms | +2.626% / +2.435% |
| 고유 현재 token 거부 | 106.1478 / 111.8592 ms | 87.7162 / 92.7024 ms | -17.364% / -17.126% |
| 고유 인접 token | 24.1237 / 25.0536 ms | 18.6338 / 19.2999 ms | -22.757% / -22.966% |

입력과 Sample JSON SHA-256:

| workload | 입력 | 기준 sample | 후보 sample |
| --- | --- | --- | --- |
| 반복 문맥 | `9e5c3adc0037cca693d6b36db94cb759ebabac548e4b5c9103e31875ebb26035` | `b1ad40c09d0fff1ca4f43829551aa1719e3f6ea055b55178af4ad2c6fe5cd65e` | `84c3f63a032e0834d7a0b4b28f64470ee682edd716a80ab844c4808349ca773e` |
| 고정 인접 token | `dcbee0adff204c19234c69d6f49518e85c6552cbf8f11bda8a4aeeaa2ae8846c` | `dc547092768d27a1cf2c5602822233b0900fdea4ffbadd99b97e1499d77c746b` | `5e09a2a179f3f58a333e6808068adeaade559cc2c31b8c9714530ff54466a6c9` |
| 교대 공백 문맥 | `b141deb68516b9f9a8c6b1dbb8bdba9225900b9b301b6239355e03616cc7e355` | `03fc72635a96aa3b893672bb03adb066c95bc3a962822a388e2e9cd3756dbfee` | `8c4e790140216ff3599ff94e8b81eca4d21a627337ea3ac2d37ba02ae7f988f4` |
| 고유 현재 token | `d78c2495ee5c272f32a664ce4a6c676a88f0e9abb5ebf7a6380de1c64f98606b` | `2cfbba8196a1e6e9e9a6ed6d98632b927ee8935f5e0dd785ac97823599f159b6` | `fa4bca8ba399dbbeebae179b2ce92ebabc5f540b5ad0e5bb086bafc3c94dbf99` |
| 고유 인접 token | `e5e74147ab26571ca929d4da880a9e80202a5c6054290a71797679bd60cebcf3` | `361b1f59fce7037d34d22f241b979242d0aef5605240bdd5b82b00040cdc5095` | `fdc096745bd698f7a5dc6d79abcffc71ee81e4985ceea344fa6237f59b53dafc` |

반복·고정·교대 입력은 소수의 동일 token 또는 문맥을 반복해 cache와 준비 graph 공유에 유리하다.
고유 입력의 개선 폭이 더 크므로 이번 결과는 cache hit 표본 편중으로 만들어진 개선이 아니다.
반복 계열의 불리한 절대 결과도 채택 판단에 포함했다.

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.042001 [0.041700, 0.043115] s | 0.040744 [0.039991, 0.041162] s | -2.99% |
| embedded | cases/s | 31,691.7 [31,417.7, 32,753.2] | 34,086.6 [30,636.4, 34,904.8] | +7.56% |
| embedded | p95 | 0.0617 [0.0610, 0.0625] ms | 0.0598 [0.0574, 0.0663] ms | -3.08% |
| embedded | RSS | 42,276 [42,260, 42,284] KiB | 42,288 [42,280, 42,296] KiB | +0.03% |
| full-POS | initialization | 0.077496 [0.075321, 0.080881] s | 0.077850 [0.076451, 0.079569] s | +0.46% |
| full-POS | cases/s | 18,725.5 [17,876.6, 19,286.3] | 19,623.0 [18,755.9, 20,866.8] | +4.79% |
| full-POS | p95 | 0.1385 [0.1328, 0.1478] ms | 0.1343 [0.1296, 0.1376] ms | -3.03% |
| full-POS | RSS | 57,984 [57,972, 58,048] KiB | 57,796 [57,792, 57,924] KiB | -0.32% |

Canonical, development, hard-negative, query matrix, Robust, shadow와 Agent/Human workflow에서
성능·환경·version 필드를 제외한 품질 projection은 기준과 후보가 byte 단위로 같았다. Projection
SHA-256은 양쪽 모두
`a0b6b6cece8282a82570c7860a15c1cf94d050f9bb724822bc82fc557bc777cb`다. 기준/후보 morphology
report SHA-256은 각각
`1ad81da06c34d74b2133d6e330e2f55e20a185c7417682870ca681f3309bfe23`,
`5c6fb34d98948781e8a5d08b66d05264014cc52e377e42a1e6237ae2dfec2a80`다.

## 정확성과 안전성

대표 resource와 256-case property test에서 새 graph 사실을 기존 직접 resource 순회와 비교한다.
임의 1~12음절 token, 최대 63개 edge, nominal·particle·접사·어근·용언·복합 POS를 생성해 exact
particle host 목록과 완성 nominal-particle host가 같은지 확인한다. Graph node 상한과 token byte
범위를 기존 준비 경로가 계속 적용한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 총 4,648,117개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다. `slowest_unit_time_sec`는 모든 target에서 0이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 999,090 | 557 MiB |
| `matcher_bytes` | 34,526 | 451 MiB |
| `matcher_plan` | 217,693 | 530 MiB |
| `user_lexicon` | 732,205 | 679 MiB |
| `json_output` | 334,705 | 556 MiB |
| `binary_detection` | 54,915 | 489 MiB |
| `pos_resource` | 1,832,167 | 608 MiB |
| `component_resource` | 109,773 | 407 MiB |
| `search_executor` | 88,240 | 472 MiB |
| `structural_preparation` | 244,803 | 579 MiB |

## 재현

기준 worktree에는 후보의 benchmark와 spec 계약만 적용해 입력과 runner를 같게 했다. 제품 코드는
각 revision 그대로 측정했다. 최종 test-only 보강 뒤 Criterion과 morphology를 다시 실행했다.

```console
git switch --detach fe80f9aab0d9ddf26bde9fdd5d76b7b209f054d5
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-criterion.sh 'matcher/build_and_find_structural_exact'
scripts/benchmark-morphology.sh target/morph-selection-facts-baseline-final

git switch --detach fe9a06a36683b055c24ae9be600f062e05bd4d71
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-criterion.sh 'matcher/build_and_find_structural_exact'
scripts/benchmark-morphology.sh target/morph-selection-facts-candidate-final
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
