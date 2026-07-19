# Matcher 입력 view

- 측정일: 2026-07-19
- 기준 revision: `d97daf40d7c2503063df39e6d6660f2a10443965`
- 후보 code revision: `5b853fd70a5e2e2274d2e7b30dd1882aa5aeb57c`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0

## 결론

제품 matcher의 구조 분석 입력을 소유 `String`에서 haystack을 빌리는 view로 바꿨다. 일반적인 NFC
입력은 현재 token과 앞뒤 token을 복사하지 않는다. NFD 등 정규화가 필요한 입력만 NFC 문자열과
offset mapping을 소유한다. Cache miss에서는 앞·현재·뒤를 포함한 bounded context의 UTF-8, NFC
상태와 normalized scalar 수를 한 번에 확인하고 같은 결과로 세 token view를 만든다. 공개
`AnalysisWindow` API가 소유 값을 요구할 때만 compatibility 경계에서 `String`을 만든다.

새 cache는 추가하지 않았다. 반복 context뿐 아니라 교대 context, 고유 이웃, 고유 현재 token을
같이 측정했다. 후보 p50은 각각 4.462%, 3.649%, 15.269%, 9.897% 단축됐다. Cache hit가 낮은 두
workload의 개선이 더 커서 반복 표본 편중으로 만든 결과가 아니다.

기존 256-entry content cache 제거도 별도 실험했다. Context를 한 번만 정규화하는 구조까지 적용한
상태에서 cache를 제거하면 반복·교대·고정 이웃 p50이 7.323%, 6.907%, 13.953% 느려졌고, 고유
이웃·고유 현재 token은 20.095%, 10.735% 빨라졌다. 제품 workload에서도 embedded와 Human
처리량이 각각 1.314%, 1.190% 불리했다. 따라서 cache를 구조 개선 대신 덧대지 않았지만, 실제
반복 입력의 재분석을 막는 기존 cache도 무조건 제거하지 않았다. 제거 실험 코드는 제품에 남기지
않았다.

## 구조

기존 경로는 구조 candidate마다 현재 token의 소유 window를 먼저 만든 뒤 context cache miss에서
앞뒤 token을 다시 정규화했다.

```text
haystack
  -> current token UTF-8/NFC 검사 -> owned String
  -> context cache lookup
  -> miss: previous/current/next를 각각 NFC 정규화
  -> structural preparation
```

변경 뒤에는 span과 정규화 필요 여부가 소유권을 결정한다.

```text
haystack
  -> bounded current/context span
  -> context cache lookup
  -> hit: current만 borrowed-or-owned view로 복원
  -> miss: context UTF-8/NFC/scalar 검사 1회
       -> NFC: previous/current/next가 haystack을 borrow
       -> non-NFC: 필요한 normalized text와 mapping만 own
  -> structural preparation
  -> public diagnostic API에서만 owned AnalysisWindow materialization
```

`StructuralCache`와 streaming matcher의 lifetime은 입력 haystack에 묶여 view가 입력보다 오래
살 수 없다. Cache key와 eviction 정책, 공개 API와 사용자 동작은 바꾸지 않았다.

## Criterion과 표본 편중

양쪽 revision에 같은 benchmark source와 runner를 사용했다. Source SHA-256은
`15d3674bb9e22b2533ecf2770ac3b6138129f693b5fa0044cf38660460d010df`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 sample별 1회 시간을 정렬한 p50 midpoint와
nearest-rank p95다. `constant neighbors`의 첫 기준 p95에 severe outlier가 몰려 양쪽을 다시
측정한 값을 사용했다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 반복 context 16,384개 | 13.4893 / 13.7324 ms | 12.8875 / 13.3035 ms | -4.462% / -3.123% |
| 두 spacing 교대 | 13.8844 / 14.1179 ms | 13.3777 / 13.8507 ms | -3.649% / -1.892% |
| 동일 앞뒤 token | 13.2103 / 13.5795 ms | 12.5952 / 12.7681 ms | -4.656% / -5.975% |
| 매번 고유한 앞뒤 token | 18.4626 / 18.9404 ms | 15.6435 / 16.1110 ms | -15.269% / -14.939% |
| 매번 고유한 현재 token, 전부 거부 | 63.5596 / 65.4786 ms | 57.2692 / 59.9602 ms | -9.897% / -8.428% |

입력과 SHA-256:

| workload | bytes | SHA-256 |
| --- | ---: | --- |
| 반복 context | 114,688 | `9e5c3adc0037cca693d6b36db94cb759ebabac548e4b5c9103e31875ebb26035` |
| spacing 교대 | 122,880 | `b141deb68516b9f9a8c6b1dbb8bdba9225900b9b301b6239355e03616cc7e355` |
| 동일 앞뒤 token | 245,760 | `dcbee0adff204c19234c69d6f49518e85c6552cbf8f11bda8a4aeeaa2ae8846c` |
| 고유 앞뒤 token | 245,760 | `e5e74147ab26571ca929d4da880a9e80202a5c6054290a71797679bd60cebcf3` |
| 고유 현재 token | 212,992 | `d78c2495ee5c272f32a664ce4a6c676a88f0e9abb5ebf7a6380de1c64f98606b` |

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 반복 context | `cfdeba97579e73e70b1b5782ec1c1c866618c3b0a007c463cc7c124aa57ea9db` | `1ba387ee10b1456ad574caa85b81463b87ec9577b3affc32432caa62eb8a607c` |
| spacing 교대 | `10d04f7b6a15e90b5f503e529a055b1a4f08ea9a81f20f23f0c7daaea68d093e` | `70ce5a8ef7f91c29c47eff83a5f7665209f56dba1ddf12ba3656b865f88c6ed8` |
| 동일 앞뒤 token | `fb48fa0461819c2f3ec2928de817d7e64498218d3ec5c885ed7b8ca8110057de` | `9e7a8c83cebddc2f43de7dd2d727399bbea99524db7a368ca8a48bf016960067` |
| 고유 앞뒤 token | `e857025f8952fe3059e7eccab74f4130209b93ea9aa4813ae168a73d642ad19e` | `f3faba5cc42dfb13b406a36cfff23164315bc022af19bd720daa8aaa2615feba` |
| 고유 현재 token | `27cc66141a7b7c7ccd0e633ba9a0dfbe9da0d343d902e4bac4c65c28936372bc` | `a672ef31c424f3dc1db2ca7d56e36d4b7a220112656fe131537bd20d19db2ff3` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 후보 A, 기준, 후보 B를
연속 실행해 시간에 따른 시스템 변동을 확인했다.

| profile / metric | 후보 A | 기준 | 후보 B | 후보 B 변화 |
| --- | ---: | ---: | ---: | ---: |
| embedded cases/s | 40,125.1 | 40,662.3 | 40,212.7 | -1.106% |
| embedded p50 | 0.0202 ms | 0.0185 ms | 0.0198 ms | +7.027% |
| embedded p95 | 0.0562 ms | 0.0540 ms | 0.0540 ms | 0.000% |
| embedded peak RSS | 42,232 KiB | 42,224 KiB | 42,224 KiB | 0 KiB |
| full-POS cases/s | 25,056.3 | 23,683.9 | 25,413.1 | +7.301% |
| full-POS p50 | 0.0214 ms | 0.0242 ms | 0.0208 ms | -14.050% |
| full-POS p95 | 0.1104 ms | 0.1162 ms | 0.1090 ms | -6.196% |
| full-POS peak RSS | 57,924 KiB | 57,948 KiB | 57,988 KiB | +40 KiB |

후보 A/B는 서로 가깝지만 가운데 기준은 embedded에서는 더 빠르고 full-POS에서는 더 느렸다.
따라서 full-POS의 큰 개선을 변경 효과로 일반화하지 않는다. 불리한 embedded 처리량과 p50도
누락하지 않는다. 변경 경로를 직접 반복하는 Criterion의 모든 표본군 개선을 채택 근거로 삼고,
제품 p95와 RSS에 반복되는 실질 회귀가 없는지를 guardrail로 사용했다.

성능 scalar와 환경·버전 필드만 재귀적으로 제외한 canonical, development, hard-negative,
Human, query matrix, Robust, shadow와 workflow 결과는 기준과 두 후보에서 같다. Projection
SHA-256은 `675a7d3846587864b32e128814d49912afa1f663828c10395bb24feab9f34858`다. Fixture
SHA-256은 `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`, morphology
resource는 `50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23`, component
resource는 `d3a7eb486eef5faa92e006dd72e5ff72b63befb4bb102013f2d19e5fc32ff00a`다.

Morphology report SHA-256:

- 후보 A: `ba772e7fb9f616659a346717121e23d1c1fee9bb21130783ca6eae518a821b77`
- 기준: `16b04f6b1e39b95babf00fdc56c9a1a7ff4078fe3071541ac28d754be150d65b`
- 후보 B: `d5fd4499b1337e8897b8979e8d588074e8365a1b4526524559ec35d8a764d3c8`

## 안전성

Borrowed view는 Rust lifetime으로 haystack보다 오래 살 수 없다. Invalid target, bounded raw bytes,
UTF-8, normalized scalar limit와 NFD offset mapping 검사는 기존과 같은 fail-closed 경계를 유지한다.
임의 Unicode scalar sequence에 대해 quick NFC 판정과 scalar count가 `is_nfc` 및 실제 NFC 결과와
같은지 property test로 검증한다. NFC는 borrow하고 NFD는 필요한 값만 own하는지도 별도 회귀 검사로
고정했다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 10개 target을 각각 15초, 입력 timeout 5초,
RSS 상한 2 GiB로 실행했다. 모든 target이 완료됐고 crash, panic, timeout, RSS 초과와 failure
artifact는 0건이다. 변경 경로인 `structural_preparation`은 257,784개 입력, peak RSS 594 MiB였고
`slowest_unit_time_sec=0`이었다.

Workspace 전체 Rust test, Clippy `-D warnings`, Rust/fuzz formatter, morphology Python 77개와 README
guard도 통과했다. 공개 동작과 문서 계약은 바뀌지 않아 README, CLI help와 man page는 수정하지
않았다.

## 재현

Morphology runner SHA-256은
`35eb318302ba4e16f36df735eb4a42086b0d124de19e52bbef65c0a204391fd0`, fuzz runner는
`3bba3af9906451c92e421b91cbe0c3c45092bf400e5483d7333a1ae64c1a4968`다.

```console
git switch --detach d97daf40d7c2503063df39e6d6660f2a10443965
scripts/benchmark-criterion.sh context_
scripts/benchmark-morphology.sh

git switch --detach 5b853fd70a5e2e2274d2e7b30dd1882aa5aeb57c
scripts/benchmark-criterion.sh context_
scripts/benchmark-morphology.sh
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
