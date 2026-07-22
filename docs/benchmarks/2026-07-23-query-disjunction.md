# 검색 질의 disjunction benchmark

## 대상

- baseline: `cc996439aa5eca9fbc71386440ae707b05527817`
- candidate: `6b27f13273e1c53374e3a7b212871ced94f3a5a3`
- macOS 26.4.1 (25E253), Apple M1 Max, 32 GiB, arm64
- rustc 1.97.0, cargo 1.97.0, release profile

Candidate는 따옴표와 escape 밖의 `|`를 disjunction 연산자로 해석한다. 모든 alternative를 하나의
logical atom plan으로 합쳐 anchor engine이 입력을 한 번만 순회한다.

## 측정 방법

양쪽 revision을 별도 worktree에서 같은 환경과 입력으로 측정했다. Criterion 기본 warm-up 3초 뒤
100 sample을 수집했다. `sample.json`의 `times[i] / iters[i]`를 정렬한 nearest-rank p50과 p95다.

```console
# baseline
scripts/benchmark-criterion.sh query_compile
scripts/benchmark-criterion.sh 'matcher/scan_deterministic_corpus'

# candidate
scripts/benchmark-criterion.sh query_compile
scripts/benchmark-criterion.sh 'matcher/(scan_deterministic_corpus|disjunction_find_all)'
```

입력은 다음과 같다.

| workload | 입력 | byte | SHA-256 |
| --- | --- | ---: | --- |
| `single_atom` | `걷다` | 6 | `82e3bb1b27351e4d45ed77e3e825fcedca811cfd6e40738c8a7c0f535a886fbd` |
| `phrase_8_atoms` | `n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다` | 86 | `baaaf9b426df6b368b591c48eeb3e77af8f42e6e90940d81a2f5ff7f1fee46d4` |
| `disjunction_8_atoms` | `n:사용자\|n:권한\|v:검증하다\|adj:예쁘다\|det:새\|adv:빨리\|n:기술\|v:걷다` | 86 | `f7caf3abe90954c14f80bbd4ee83df69216324c4086c3276b635d6e7604032d1` |
| `disjunction_find_all` | `lit:걸어\|lit:사용자는` | 27 | `99f19f3351eccf89006ebaa35898649f5f1baf6196e4c6e1c67a7f591ff25e36` |
| matcher corpus | 1,024줄, 64줄마다 `길을 걸어 갔다. 권한을 검증했습니다.`, 나머지는 `사용자는 새 문서를 읽고 접근 정책을 확인했습니다.` | 72,400 | `179344011414c9c439eea3700b3f33e844eb21b6f241e97615696eeba25de450` |

## 결과

Query compile 단위는 1회당 마이크로초다.

| workload | baseline p50 / p95 | candidate p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `single_atom` | 46.709 / 47.370 us | 45.652 / 46.974 us | -2.26% / -0.84% |
| `phrase_8_atoms` | 118.801 / 123.592 us | 114.798 / 118.648 us | -3.37% / -4.00% |
| `disjunction_8_atoms` | - | 116.061 / 120.138 us | 신규 workload, p95 목표 750 us 이하 |

Matcher 단위는 corpus 1회당 마이크로초다.

| workload | baseline p50 / p95 | candidate p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `scan_deterministic_corpus` | 233.847 / 240.589 us | 234.042 / 245.300 us | +0.08% / +1.96% |
| `disjunction_find_all` | - | 258.514 / 265.781 us | 신규 workload, p50 267.09 MiB/s |

`disjunction_find_all`은 모든 줄에서 하나의 alternative가 일치하고 기존 scan은 64줄마다 한 번
일치하므로 두 수치를 의미상 동등한 baseline/candidate로 비교하지 않는다.

Sample JSON checksum은 다음과 같다.

| workload | baseline | candidate |
| --- | --- | --- |
| `single_atom` | `a90e767ce2742301854ad0eb3bde25c155d0dabab49faa6179b9a985ce1c2844` | `c78e25fea96ae4a3579cc84f16db5a2c084952aa02eeb7b461e288fe67c75dd2` |
| `phrase_8_atoms` | `f91116c53b8b405671f204e75c236079e6a11d6351937e10da664040981c314a` | `ebf2f1abf64d81205ae7234b0323716c5cc3b5a1c9992f367a629b3896245715` |
| `disjunction_8_atoms` | - | `3f75568107f61364565cd31a691cc52a478a6b5f60538e795b39b56c001446ad` |
| `scan_deterministic_corpus` | `73775e85c8c91cd100eef97a65300c1dfe0cb6a5f0483e0a3e33edde3c00f71e` | `a0f8f60d1dc8d2515b5f6cee901cb17e4779b6b007a758ca57884ec2c17177b3` |
| `disjunction_find_all` | - | `5bb3c569382ce7985d588c42f71a5763663eb0e4281ef15dae6b47eedab051d7` |

## 판정

8 atom disjunction compile p95는 0.120138 ms로 0.75 ms 목표를 충족한다. 기존 query compile
workload는 p95가 0.84%와 4.00% 줄었다. 기존 corpus scan은 p50 +0.08%, p95 +1.96%로 불리하게
움직였으나 절대 차이는 p95 4.711 us이고 구현은 해당 scan 경로를 바꾸지 않았다. 새 disjunction
matcher는 p50 267.09 MiB/s로 전체 corpus를 한 번 순회한다. 기능 이득에 비해 기존 경로의 변동이
작으므로 변경을 채택한다.

정확성은 lexer·compile plan·matcher public API·CLI·WASM package test로 확인했다. 동일 span을
만드는 alternative의 provenance 병합, 공백 유무, literal `|`, 피연산자 누락과 phrase 혼합 오류를
포함한다.
