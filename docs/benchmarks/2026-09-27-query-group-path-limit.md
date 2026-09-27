# 그룹 경로 상한의 컴파일 비용

- 측정일: 2026-09-27
- 기준 revision: `ab18fc4b1bb830365125c3976a930f00166b6b7f`; 후보 코드 revision: `2976bc645aff37643439782d6bd70d20ffa6bfc3`
- 환경: macOS 26.6.2, Apple M1 Max, arm64, 32 GiB, Rust 1.97.0, Criterion 0.7.0. 같은 호스트에서 기준과 후보를 순서대로 release 빌드해 측정했다.
- 명령: 각 worktree에서 `scripts/benchmark-criterion.sh 'query_compile/grouped_32_paths$'`. 기준에도 후보와 동일한 benchmark source를 임시로 적용했다. Source SHA-256은 양쪽 모두 `abdb61ba7ed8717da0ab82254967476073920b5d56988a48a85ca339e0844366`이다.
- 입력: `(lit:가|lit:나) (lit:다|lit:라) (lit:마|lit:바) (lit:사|lit:아) (lit:자|lit:차)`, 89 UTF-8 bytes, SHA-256 `c1d52b4102f62cf28ba9b0431b68f88b37dcd52ad5617f4d6ccfe8a2b19de6be`. 기본 `CompileOptions`와 같은 analyzer를 사용했다. 10개 atom이 32개 완성 경로를 표현한다.
- 측정: fresh benchmark process에서 기본 warm-up 3초 후 100개 sample. Criterion `sample.json`의 `times[i] / iters[i]`를 µs로 환산한 median과 nearest-rank p95를 썼다. 작을수록 빠르다.

| Workload | 기준 median / p95 (µs) | 후보 median / p95 (µs) | median 변화 | p95 변화 |
| --- | ---: | ---: | ---: | ---: |
| `query_compile/grouped_32_paths` | 10.75 / 11.25 | 10.64 / 10.73 | -1.07% | -4.62% |

경로 수 검사는 컴파일 단계에서만 수행한다. 같은 입력의 matcher 실행 경로는 이 변경에 포함되지 않는다.
