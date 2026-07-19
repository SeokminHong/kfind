# 리소스 소유 구조 POS 표현

- 측정일: 2026-07-19
- 기준 코드 revision: `dc5efaccf9a6c3899bcb4602dc6f0366005b8450`
- 후보 코드 revision: `436b77756edd9672130d0a13bdc353e3c2598db8`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

구조 판정이 component resource의 POS 문자열을 token과 기능마다 `+`로 분리하고 품사 접두사를
다시 해석하던 경로를 제거했다. Resource loader가 검증된 string ID마다 1-byte typed POS sequence를
한 번 만들고, token graph와 POS-only prefix 순회는 resource가 소유한 immutable slice를 빌려 쓴다.
Component span이 필요한 경로만 전체 analysis를 materialize한다. Token cache, POS interning과
token-local arena는 추가하지 않았다.

4,032-edge 반복 POS graph 준비는 p50/p95 52.493%/51.270%, 모든 분석의 POS가 고유한 같은 크기
graph는 40.078%/39.898% 단축됐다. 고유 현재 token은 28.445%/23.850%, 고유 인접 token은
1.780%/1.633% 단축됐다. 따라서 반복 표본의 높은 cache·intern hit율을 채택 근거로 사용하지 않았다.

두 실행 순서의 공식 morphology 측정에서 smart 처리량은 embedded 10.153~11.399%, full-POS
12.106~12.163%, Human query matrix는 15.562~18.240% 개선됐다. Component-free 대조군은 같은
실행에서 느려졌으므로 시스템 변동이 후보를 유리하게 만든 결과로 해석하지 않았다. 품질 projection은
기준 두 번과 후보가 byte 단위로 같았다. Component load 시간의 지속적인 회귀는 없었고 peak RSS는
최대 64 KiB 늘었다.

## 구조

기존 경로는 resource lookup 뒤 구조 기능마다 문자열 표현을 다시 해석했다.

```text
validated component resource
  -> ComponentAnalysis { pos: &str, components: Vec<_> }
  -> token graph가 pos 문자열 보존
  -> 각 상태기계가 split('+')와 starts_with(E/J/N/V) 반복
```

변경 뒤에는 검증과 실행 표현의 소유권이 resource에 모인다.

```text
validated string table
  -> string ID offset + flat Box<[ComponentPos]> compact table
  -> POS-only prefix: &[ComponentPos] 직접 순회, analysis/component Vec 없음
  -> token graph: resource sequence slice를 차용
  -> component span 필요 경로: 기존 검증된 component materialization
  -> 모든 구조 상태기계: typed POS와 상수 시간 분류 사용
```

`ComponentPos`는 `repr(transparent)` 1-byte 값이다. 알려진 fine POS와 `NNBC`, ending, particle,
derivation/root를 직접 표현하고 나머지 `E*`, `J*`, `N*`, `V*` 범주도 기존 문자열 접두사 의미를
보존한다. 구조 모듈의 실행 경로에서는 `split('+')` 42곳과 `E/J/N/V` 접두사 판정 38곳이 모두
사라졌다. Binary resource schema는 바꾸지 않았으며 typed table은 resource decode 뒤의 검증된
in-memory view다.

## Criterion과 표본 편중

양쪽 revision에 동일한 benchmark source와 runner를 사용했다. Benchmark source SHA-256은
`e003135d4d79fc533f4a0f929695d7b5a9cbe6fb9c95a2b9935020f7d6fb81d1`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을 정렬한
nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 구조 후보 판정 | 4.2429 / 4.3951 us | 2.7595 / 3.0001 us | -34.962% / -31.739% |
| 4,032-edge 반복 POS graph 준비 | 0.5377 / 0.5521 ms | 0.2555 / 0.2690 ms | -52.493% / -51.270% |
| 4,032-edge 고유 POS graph 준비 | 0.3661 / 0.3761 ms | 0.2194 / 0.2261 ms | -40.078% / -39.898% |
| 16개 prepared preferred-path 후보 | 0.2160 / 0.2301 ms | 0.2152 / 0.2225 ms | -0.353% / -3.283% |
| particle suffix 12회 거부 | 4.5821 / 4.8436 us | 1.0199 / 1.0405 us | -77.741% / -78.518% |
| particle suffix 20회 거부 | 11.0972 / 11.6991 us | 2.4296 / 2.6066 us | -78.106% / -77.719% |
| 준비된 graph의 nominal 선택 | 0.4966 / 0.5174 us | 0.3938 / 0.4036 us | -20.698% / -22.003% |
| dense graph 생성 + 선택 | 49.7914 / 51.2638 us | 31.4800 / 32.6588 us | -36.776% / -36.293% |
| matcher 생성 + 첫 구조 판정 | 3.5467 / 3.8616 us | 2.8754 / 2.9830 us | -18.929% / -22.753% |

### 문맥 분포

반복 hit이 유리한 입력과 재사용할 수 없는 입력을 같은 benchmark source에서 분리했다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 반복 문맥 | 13.7863 / 14.4240 ms | 13.3965 / 13.6539 ms | -2.827% / -5.338% |
| 고정 인접 token | 13.3761 / 13.6739 ms | 13.1283 / 13.3853 ms | -1.853% / -2.111% |
| 교대 공백 문맥 | 14.2007 / 14.5565 ms | 13.8610 / 14.2211 ms | -2.392% / -2.304% |
| 고유 인접 token | 18.8208 / 19.1756 ms | 18.4859 / 18.8625 ms | -1.780% / -1.633% |
| 고유 현재 token 거부 | 94.0454 / 96.3339 ms | 67.2941 / 73.3587 ms | -28.445% / -23.850% |

입력 SHA-256은 각각 반복
`9e5c3adc0037cca693d6b36db94cb759ebabac548e4b5c9103e31875ebb26035`, 고정 인접
`dcbee0adff204c19234c69d6f49518e85c6552cbf8f11bda8a4aeeaa2ae8846c`, 교대
`b141deb68516b9f9a8c6b1dbb8bdba9225900b9b301b6239355e03616cc7e355`, 고유 인접
`e5e74147ab26571ca929d4da880a9e80202a5c6054290a71797679bd60cebcf3`, 고유 현재
`d78c2495ee5c272f32a664ce4a6c676a88f0e9abb5ebf7a6380de1c64f98606b`다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| matcher 생성 + 첫 구조 판정 | `a3ad47479c12938912a2cfa0f43a3d29bee5f9893f8a838aed95b1856fd699b4` | `7375292a8e42f76baba9dcf85bd19ea5a28c4da6f63a1a4a39ca05380f0e51a7` |
| 반복 문맥 | `57f76c559b8e8f29493ed89f8372048c4b057df0c9737032876bc476d18f419a` | `db8ea447baa6de2cc1fcbfc709fb751ca2bf5a9786b26d1448584bef4b6391ef` |
| 고정 인접 token | `926d8fe5bd42caa63d90bc399dd52e95ccb5af2e1a3c9fca53bdabb0248bbf19` | `85d12d2669b520c4f93d374cae366eb1c2039ec47ffd7acce654afd388e218d5` |
| 교대 공백 문맥 | `6763d85d25a1f33d8671f697ef2a0a27743d7184a319573870d3979138dcb1d3` | `2ec08d5c29a77d5506c5be21b1092775d2fff076a9e91313daad63b361ca245c` |
| 고유 인접 token | `9a304b41d6de47780bf9b4a33fbf785f394bf7c22cc69cb41ac5d29fc5a4c683` | `cb13c7885fabb231efffeb70fdce43a9788411a7a5ddefb809c5ff7ff6ae1988` |
| 고유 현재 token | `b4ceea2741690c24d5835733e95f11938922d8b9d8db5523058e4a571fc9182c` | `18455ba2e77107a78f05107ee278c24e9e10fff1be0d0f936860186b3126587d` |
| 반복 POS graph | `fd949ac350583afdb6c46e5040c605601680262cfee913b9b1c6a354d9d3b6c3` | `1941e9aebf826c3ecfc8f1fdcfbc7dbe69f3429e866b2ef8671a3550b06f426c` |
| 고유 POS graph | `894b30ef9cd8e4f1718b60856fc05eb9b677464a68e588daab0e9fd571343ea1` | `d5b999901a7b8c66303f7b8099f183dec0191dcad13f5d4127bb649cc83c9815` |
| particle suffix 12회 | `e9c35eab6f038c7f35eb8f16078c26a75a3f5a60588fb69a2bdf804247950de8` | `72f22b6b9fdbd33d436efd7643550f06f5e7d233a5c35423562e757aef5b3bd8` |
| particle suffix 20회 | `4b273ca1847f6f1d9c018449655f6cac5b4785c8266a01ae45052b7262119912` | `8c0fd626371da8f33feb59b38314ecf82387598b26a77789ecbb0474b4673d92` |
| 구조 후보 판정 | `dae349536c02b36c98d1c4de616d5a372bc81e02a648f40880ad359d1f484581` | `d5451096c7164acb8fb13430967d4eacbc36d7da6c6a227ac1554831ee046bff` |
| prepared preferred path | `9505bd5b91277c8f63f27b84e60c57d116233a9c432212f93af2e066d5be277e` | `8a8e331abb7aa2b75fe798d31d765d29e3f9c901baa4ab0a43429c2d37459a2c` |
| 준비된 graph의 nominal 선택 | `9f3186620b4b30e61d040059cb80dbe41c59138f15e306237c601758e447b147` | `7ce76a378bc7dee4f9475300179e9375701582d8a5581ee8989cd2ebfe4bd44f` |
| dense graph 생성 + 선택 | `87205de2dc61939113ebb46f29193cc927b228fc12e87e46208f13a081b44d29` | `8f31241aae58fb91e6d016c63b3af0a0ba4be28d2a34ff312132ff292ee8d769` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 기준→후보와 후보→기준
순서로 실행했다. 표는 후보→기준 역순 측정을 기준으로 한 `median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded smart | initialization | 0.044561 [0.044042, 0.047481] s | 0.044800 [0.044170, 0.049612] s | +0.536% |
| embedded smart | cases/s | 31,226.2 [29,064.7, 32,257.9] | 34,396.2 [30,090.6, 36,223.4] | +10.153% |
| embedded smart | p95 | 0.0643 [0.0614, 0.0667] ms | 0.0634 [0.0609, 0.0741] ms | -1.400% |
| embedded smart | RSS | 42,284 [42,276, 42,296] KiB | 42,228 [42,216, 42,232] KiB | -0.132% |
| full-POS smart | initialization | 0.080740 [0.079813, 0.082093] s | 0.079763 [0.079424, 0.081834] s | -1.210% |
| full-POS smart | cases/s | 18,561.6 [15,690.8, 18,708.5] | 20,808.7 [20,082.0, 21,165.2] | +12.106% |
| full-POS smart | p95 | 0.1423 [0.1410, 0.1704] ms | 0.1337 [0.1332, 0.1423] ms | -6.044% |
| full-POS smart | RSS | 57,924 [57,864, 57,992] KiB | 57,984 [57,920, 57,988] KiB | +0.104% |

제품 workload의 두 실행 순서:

| 실행 순서 | workload | cases/s 변화 | p95 변화 |
| --- | --- | ---: | ---: |
| 기준→후보 | embedded smart | +11.399% | -2.160% |
| 기준→후보 | full-POS smart | +12.163% | -5.512% |
| 기준→후보 | Human untagged smart | +10.247% | +0.068% |
| 기준→후보 | Human query matrix | +15.562% | -11.902% |
| 후보→기준 | embedded smart | +10.153% | -1.400% |
| 후보→기준 | full-POS smart | +12.106% | -6.044% |
| 후보→기준 | Human untagged smart | +11.561% | -7.382% |
| 후보→기준 | Human query matrix | +18.240% | -14.269% |

기준→후보의 Human untagged p95는 0.1467 ms에서 0.1468 ms로 0.068% 불리하게 이동했다.
반대 순서에서는 0.1585 ms에서 0.1468 ms로 개선돼 지속적인 회귀로 보지 않는다. 같은 역순
실행에서 component를 읽지 않는 embedded-any 처리량은 5.386%, Agent query matrix는 1.758%
감소했다. 후보의 component 경로 개선은 이 불리한 시스템 이동보다 크다.

Component load 중앙값은 embedded에서 0.026685초에서 0.024362초, full-POS에서
0.027327초에서 0.026641초였다. 기준→후보 측정에서도 회귀는 없었지만 실행 범위가 겹치므로
startup 개선으로 일반화하지 않는다. Peak RSS는 embedded component profile에서 두 실행 기준
각각 60 KiB와 64 KiB 늘었고 full-POS에서는 0~8 KiB 늘었다.

성능·component startup·환경·version 필드를 재귀적으로 제외한 canonical, development,
hard-negative, query matrix, Robust, shadow와 Agent/Human workflow projection은 기준 두 보고서와
후보 보고서가 byte 단위로 같다. Projection SHA-256은
`06c0a70a469e36224e1369e12bf1a427a5d0eca2cbb9639c6bb5290565ed4bf1`다.

Morphology report SHA-256:

| 실행 | 기준 | 후보 |
| --- | --- | --- |
| 기준→후보 | `18d35b8ab802570d56bbeaa871f8008b1c894c501fe90253b5a336b97c835f22` | `71a336df660bff47394454aa5caa594ee7d1f53387bce06c832b5d2be6b6c2c9` |
| 후보→기준 | `2cd09554aded9b8d07662bd7010b0ebf67b726f0f44cb46d7781817d421e3197` | `71a336df660bff47394454aa5caa594ee7d1f53387bce06c832b5d2be6b6c2c9` |

## 정확성과 안전성

Resource decode는 schema, source identity, section digest, UTF-8, string ID와 payload range를 먼저
검증한다. Typed table은 검증된 string ID만 순회하고 position 수가 `u32` 범위를 넘으면 load를
실패시킨다. `ComponentPos`가 1 byte라는 회귀 검사와 알려지지 않은 `E*`, `J*`, `N*`, `V*`,
`NNBC` 범주 보존 검사를 추가했다. POS-only 순회와 token graph는 resource가 소유한 slice를
차용하며 입력 크기에 비례한 token cache나 전역 mutable 상태를 만들지 않는다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 모든 target이 예산을 완료했고 crash, panic, timeout, RSS 초과와
생성된 failure artifact는 0건이었다. 변경 경로인 `component_resource`는 106,340개 입력, peak RSS
406 MiB, `structural_preparation`은 250,222개 입력, peak RSS 578 MiB를 실행했다. 두 target의
`slowest_unit_time_sec`는 0이었다.

## 재현

기준과 후보 worktree의 제품 코드는 각 revision 그대로 측정했다. Morphology runner SHA-256은
`35eb318302ba4e16f36df735eb4a42086b0d124de19e52bbef65c0a204391fd0`, fuzz runner SHA-256은
`3bba3af9906451c92e421b91cbe0c3c45092bf400e5483d7333a1ae64c1a4968`다.

```console
git switch --detach dc5efaccf9a6c3899bcb4602dc6f0366005b8450
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-criterion.sh 'matcher/build_and_find_structural_exact'
scripts/benchmark-morphology.sh target/morph-benchmark-pos-ir-baseline

git switch --detach 436b77756edd9672130d0a13bdc353e3c2598db8
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-criterion.sh 'matcher/build_and_find_structural_exact'
scripts/benchmark-morphology.sh target/morph-benchmark-pos-ir-candidate-final
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
