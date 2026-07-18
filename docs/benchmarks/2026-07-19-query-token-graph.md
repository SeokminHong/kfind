# Query token graph 분리

- 측정일: 2026-07-19
- 기준 revision: `d5d5173ea9c80725d11d0f4728c3ccf998b78dd7`
- 후보 코드 revision: `d38182b4a5ff11ccf73337c6bdfe50718c03d36c`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

구조 제약 판정을 현재 token의 query 불변 graph 생성과 인접 token 선택으로 분리했다. 작은
query plan의 exact whole-token anchor만 matcher에 등록하고, graph는 처음 실제로 필요한
candidate에서 한 번 생성한다. 인접 token과 raw·NFC span은 candidate마다 선택하므로 동일한
현재 token의 서로 다른 문맥에서 graph만 공유한다.

기존 전체 문맥 cache가 모두 빗나가는 고유 인접 token workload는 p50 41.55%, p95 41.65%
개선됐다. 반복·고정 문맥 workload는 3.24~4.25% 느려졌고, 현재 token까지 모두 다른 거부
workload는 0.80~0.86%, matcher 생성과 첫 검색은 7.44~7.61% 느려졌다. 반복 표본의 높은 cache
hit율을 개선 근거로 사용하지 않았으며, 약 3% 이상이 같은 exact token의 고유 문맥이면 측정된
절대 시간 기준으로 추가 준비 비용을 상쇄한다.

Canonical morphology는 embedded 처리량 +2.00%, p95 -2.41%, full-POS 처리량 -0.62%, p95
+0.23%였다. 품질 projection은 완전히 같았다. 전역 cache나 일반 token memoization은 추가하지
않았고, 큰 plan·미등록 token·메모리 상한 초과는 기존 bounded 직접 판정으로 돌아간다.

## 구조

기존 경로는 구조 candidate마다 현재 token graph와 인접 token 선택을 한 번에 수행했다.
변경 뒤 흐름은 다음과 같다.

```text
matcher build
  → 작은 plan의 exact structural anchor를 정렬 등록

candidate
  → 등록된 exact whole-token인지 확인
  → 최초 접근이면 query 불변 PreparedTokenGraph를 생성·공개
  → candidate별 인접 token과 raw·NFC span을 선택
  → graph와 선택 결과를 결합해 구조 제약 판정
```

등록 목록은 plan 전체 program 수가 8개 이하일 때만 만들고 normalized exact anchor는 64개로
제한한다. 조회는 정렬된 `Vec`의 binary search를 사용한다. Graph는 `OnceLock<Option<Arc<_>>>`에
lazy publish하고 실제 크기만 matcher 메모리 예산에 원자적으로 예약한다. 동시 최초 접근도 하나의
결과만 공개한다. One-shot API는 graph를 inline 소유해 기존 직접 판정에 heap 할당을 추가하지 않는다.

전체 structural program을 eager 준비하는 방식은 사용하지 않는 program까지 full-POS plan에서
순회·생성하므로 채택하지 않았다. Query 전체를 위한 범용 token graph나 전역 LRU도 두지 않는다.
이 경계는 graph의 query 불변 부분만 공유하면서 큰 query와 공격적인 고유 token 입력의 비용을
기존 상한 안에 둔다.

## Criterion과 cache 표본 편중

양쪽 revision에 동일한 benchmark source를 적용했다. Source SHA-256은
`9b0b6170b73652873a517fccb1d1070e5d5c15dd29ddfd6558939ce3c0ddd376`이다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을
정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 반복 문맥 | 12.722 / 13.136 ms | 13.249 / 13.676 ms | +4.14% / +4.11% |
| 고정 인접 token | 12.436 / 12.837 ms | 12.838 / 13.382 ms | +3.24% / +4.25% |
| 고유 인접 token | 39.603 / 41.027 ms | 23.148 / 23.938 ms | -41.55% / -41.65% |
| 고유 현재 token 거부 | 97.206 / 102.752 ms | 97.986 / 103.640 ms | +0.80% / +0.86% |
| matcher build + 첫 구조 검색 | 3.0847 / 3.1774 µs | 3.3193 / 3.4138 µs | +7.61% / +7.44% |
| 직접 구조 판정 | 4.0804 / 4.1994 µs | 4.0825 / 4.2107 µs | +0.05% / +0.27% |

입력은 다음 네 축을 분리한다.

| 입력 | 의미 | SHA-256 |
| --- | --- | --- |
| 반복 문맥 | 같은 raw context를 반복해 기존 cache warm hit를 강조 | `9e5c3adc0037cca693d6b36db94cb759ebabac548e4b5c9103e31875ebb26035` |
| 고정 인접 token | 같은 `가 매일 나` 문맥 반복 | `dcbee0adff204c19234c69d6f49518e85c6552cbf8f11bda8a4aeeaa2ae8846c` |
| 고유 인접 token | 현재 exact token은 같고 앞뒤 token 쌍은 모두 달라 기존 cache miss | `e5e74147ab26571ca929d4da880a9e80202a5c6054290a71797679bd60cebcf3` |
| 고유 현재 token | anchor 뒤 suffix가 모두 다르고 구조 후보는 모두 거부 | `d78c2495ee5c272f32a664ce4a6c676a88f0e9abb5ebf7a6380de1c64f98606b` |

Build workload의 `매일 보고` 입력 SHA-256은
`d7d25fbc0eaf972814ccdb26ea7a7a930f74b83776993289b34a0380625820d1`이다. 고유 인접 token의
절대 개선은 p50 약 16.45 ms이고 반복 문맥 회귀는 약 0.53 ms다. 두 입력을 단순 혼합하면 고유
문맥 비율 약 3%에서 손익분기한다. 실제 workload 분포를 이 비율로 가정하지 않으며, 네 결과를
각각 보존한다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 반복 문맥 | `10b097c09c68a97017d73c8273b9daee74345f4854b2872dae2d39a7f987bad3` | `1222cdaec7ebf893bd73ae1e7aa88abbda3a0163ff3199847abbafd57c358f67` |
| 고정 인접 token | `de4e586e416fb17ac36e0a1346bf8f853575cf1a1e7c1ee50061db575c915b16` | `02e53dede0c72b421c6874343daf08838f8c7481cd0253e73f67c37ab1c63c7c` |
| 고유 인접 token | `36f8d08c214482083ae619e451633f5a2ab521daad8c83e77a3d16897c288c33` | `34dc613b35fb56f2a2976f02072b6a8eb1ac014942b847aefd64e4c527b209ac` |
| 고유 현재 token | `b67e360f023e1a54a5f1fa3da72b8dfff3ba297b5badb94cce52e834a12affb8` | `60ad137d3574e9aff4b1c1976bd17405bd4f917799b446277357c401e53ae8dc` |
| build + 첫 검색 | `61044ec5c621375b6fe48dbd28bb261ee634caddeea076bc8695b08e07aef54d` | `92b029ab190f1843db3070fc45cda9a095e493aa314bc1869d438aac2a628fe9` |
| 직접 구조 판정 | `c0cb09595c5b9655d38e5666ed2644d777a88b02c491890b700a3eb05615ea00` | `b9c73d4b6c4bc48b7113788193b2664505d2e746357ea4fcb7a898dc3429ca03` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | cases/s | 31,612.0 [30,459.9, 33,779.4] | 32,243.5 [29,775.9, 33,474.8] | +2.00% |
| embedded | p95 | 0.0622 [0.0596, 0.0653] ms | 0.0607 [0.0587, 0.0683] ms | -2.41% |
| embedded | RSS | 42,220 [42,200, 42,224] KiB | 42,276 [42,256, 42,284] KiB | +0.13% |
| full-POS | cases/s | 19,858.1 [18,278.0, 20,378.1] | 19,734.5 [18,829.9, 20,199.8] | -0.62% |
| full-POS | p95 | 0.1324 [0.1292, 0.1411] ms | 0.1327 [0.1303, 0.1384] ms | +0.23% |
| full-POS | RSS | 58,044 [57,960, 58,044] KiB | 58,056 [57,856, 58,060] KiB | +0.02% |

초기화 중앙값은 embedded 0.044218초에서 0.039918초(-9.72%), full-POS 0.075694초에서
0.072406초(-4.34%)였지만 변경 경로가 초기화가 아니므로 개선으로 일반화하지 않는다.

Canonical, development, hard-negative, query matrix, Robust, shadow와 Agent/Human workflow에서
성능·환경 필드를 제외한 품질 projection은 기준과 후보가 byte 단위로 같았다. Projection
SHA-256은 양쪽 모두
`aa31ba742ddde51cb7ccfead83243d619fbcef145098637157609f34cbe565af`다. 기준/후보 morphology
report SHA-256은 각각
`231ffd07fee3e83653f432c28307ec066e5a5ed8fee3d095e15406a7dfda088d`,
`7a572e7583dbf7d10951d4c08e17a5ff591999cd7a2ffafaafa4e13a9ccc9f83`다.

## 정확성과 안전성

분리된 graph 경로와 기존 일괄 준비 경로가 같은 구조 판정을 내리는지 직접 비교하는
`structural_preparation` fuzz target을 추가했다. 유효한 bounded token context와 pattern을 만들고
두 판정, 잘못 연결한 graph 거부, 임의 byte 입력의 실제 structural matcher 실행을 함께 검사한다.
동시 최초 접근, 작은 plan 등록 상한과 fallback은 단위 테스트로 고정했다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 총 4,502,932개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 990,996 | 585 MiB |
| `matcher_bytes` | 31,320 | 447 MiB |
| `matcher_plan` | 218,823 | 535 MiB |
| `user_lexicon` | 756,459 | 650 MiB |
| `json_output` | 341,552 | 559 MiB |
| `binary_detection` | 67,824 | 479 MiB |
| `pos_resource` | 1,644,518 | 577 MiB |
| `component_resource` | 107,877 | 403 MiB |
| `search_executor` | 93,290 | 473 MiB |
| `structural_preparation` | 250,273 | 579 MiB |

## 재현

기준 worktree에는 후보의 benchmark source만 적용해 양쪽 workload와 입력을 같게 했다. 제품
코드는 각 revision 그대로 측정했다.

```console
git switch --detach d5d5173ea9c80725d11d0f4728c3ccf998b78dd7
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-criterion.sh 'matcher/build_and_find_structural_exact'
scripts/benchmark-morphology.sh target/morph-token-graph-baseline

git switch --detach d38182b4a5ff11ccf73337c6bdfe50718c03d36c
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-criterion.sh 'matcher/build_and_find_structural_exact'
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-token-graph-tiny-plan
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
