# 검색 질의 disjunction benchmark

## 대상

- baseline: `fdef85be6c5c06aa9dc80f191ecb88e4c865b050`
- candidate: `9d9a871f26b396fd378824b423b93396e73be1d9`
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
| `single_atom` | 46.314 / 47.770 us | 45.354 / 46.189 us | -2.07% / -3.31% |
| `phrase_8_atoms` | 115.660 / 123.095 us | 116.070 / 122.618 us | +0.35% / -0.39% |
| `disjunction_8_atoms` | - | 116.922 / 123.445 us | 신규 workload, p95 목표 750 us 이하 |

Matcher 단위는 corpus 1회당 마이크로초다.

| workload | baseline p50 / p95 | candidate p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `scan_deterministic_corpus` | 233.627 / 251.294 us | 239.401 / 248.147 us | +2.47% / -1.25% |
| `disjunction_find_all` | - | 261.816 / 270.111 us | 신규 workload, p50 263.72 MiB/s |

`disjunction_find_all`은 모든 줄에서 하나의 alternative가 일치하고 기존 scan은 64줄마다 한 번
일치하므로 두 수치를 의미상 동등한 baseline/candidate로 비교하지 않는다.

Sample JSON checksum은 다음과 같다.

| workload | baseline | candidate |
| --- | --- | --- |
| `single_atom` | `2aed6a092ab10d2620a880fb57a2e152da74ccd810889b0a550a6e28136e11ea` | `a0221ebf71ac740eff277271fd7368ecfb3948dff97c57c4fbd0808f0c69b776` |
| `phrase_8_atoms` | `7329667331a3606a328dfe2624319d4574a0f99807459b0a5711370df5eefa53` | `6fd757fbb7963e05cfbc1c37debbfa6c0aabaa72d62edb55fc942c0830025045` |
| `disjunction_8_atoms` | - | `d160c269d37df7b48a406cd981cb37afe52455f884a564e0082da99d7c8aa18b` |
| `scan_deterministic_corpus` | `cf2d934b3daa4f2af28184661fdc120f0e903348eac781ad720852018e93aa66` | `ea0416c1c4dd7a89cfb61add1280457c038f12f9d2017d54a7ecd83230dedf83` |
| `disjunction_find_all` | - | `b8460285249fd23b7728bcd4979c6b64f6e9fdbec451e5c1c853255216cc8ac9` |

## 판정

8 atom disjunction compile p95는 0.123445 ms로 0.75 ms 목표를 충족한다. 기존 query compile
workload p95는 3.31%와 0.39% 줄었다. 기존 corpus scan은 p50이 2.47%, 5.774 us 느려졌고 p95는
1.25%, 3.147 us 줄었다. 구현은 해당 scan 경로를 바꾸지 않았지만 불리한 p50 변화도 결과에
포함한다. 새 disjunction matcher는 p50 263.72 MiB/s로 전체 corpus를 한 번 순회한다. 기능 이득에
비해 기존 경로의 변동이 작으므로 변경을 채택한다.

정확성은 lexer·compile plan·matcher public API·CLI·WASM package test로 확인했다. 동일 span을
만드는 alternative의 provenance 병합, 공백 유무, literal `|`, 피연산자 누락과 phrase 혼합 오류를
포함한다.
