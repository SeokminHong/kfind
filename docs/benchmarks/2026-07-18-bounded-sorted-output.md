# Bounded path-sorted output

`--sort path`가 전체 match record를 모아 정렬하던 구조를 경로 선수집·정렬과 bounded
per-file stream으로 분리했다. 고매치 입력의 peak RSS가 98.8% 감소했고 wall time도
9.7~10.7% 짧아져 채택했다. 정렬 대상 경로는 메모리에 남지만 결과 record는
worker 수와 channel capacity에 따라 제한된다.

## Revisions and environment

- baseline: `a50c6bff5aab21369698f6870dded5efdd5175fc`
- candidate product: `00b678353677913e00f3049f48c844e38445f7b9`
- benchmark tool: `0b68c724de6821526851efe85c5c41b2e8f889ac`
- executor fuzz: `d7acacdb306abd757b4800b6b72cbbce07e20da4`
- macOS 26.4.1 `25E253`, Apple M1 Max, arm64
- `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0 (c980f4866 2026-06-30)`
- release, `--threads 12`, `/usr/bin/time`, stdout `/dev/null`
- baseline binary SHA-256: `bd90e64d3bd77dc610a4cc685875038d46bd53e92d63e014defc2e6e2cfcb039`
- candidate binary SHA-256: `14d72f2740f680f9cbd15a583ea8ec36be576f4e6844a26c971c98e8dd794bbe`

`scripts/benchmark-sorted-output.sh`를 사용해 각 revision을 따로 실행했다. 각 workload와
mode는 fresh process warm-up 1회 후 5회 측정했고 sorted·unsorted 실행 순서를 교대했다.
표는 `median [min, max]`다.

## Inputs

| workload | files | bytes | SHA-256 | purpose |
| --- | ---: | ---: | --- | --- |
| repeated | 256 | 14,680,064 | `86edb5e65876f9e6b4d984a5f727d75b0cb6eb377c3c3cc1557a9af7ca5081b2` | 2,097,152개의 동일 high-hit 행 |
| unique | 256 | 37,748,736 | `4efd5916786f668af6fc7c6aeb400bed7a1c55f49ec3e842d5b95334b194ba58` | 2,097,152개의 고유 high-hit 행 |
| low-hit | 8,192 | 139,264 | `79d11b717879f824c288d578d76d6604b08be517023631a5167d472de7c9ea3a` | 경로 수집 비용과 no-match 제어군 |

## Results

| workload | mode | baseline wall (s) | candidate wall (s) | wall | baseline RSS (MiB) | candidate RSS (MiB) | RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| repeated | sorted | 2.42 [2.37, 2.43] | 2.16 [2.11, 2.21] | -10.74% | 723.53 [723.39, 723.61] | 8.80 [8.72, 8.84] | -98.78% |
| repeated | unsorted | 2.12 [2.10, 2.15] | 2.14 [2.09, 2.15] | +0.94% | 8.89 [8.86, 8.92] | 8.88 [8.84, 8.89] | -0.18% |
| unique | sorted | 2.57 [2.57, 2.62] | 2.32 [2.30, 2.36] | -9.73% | 755.67 [755.66, 755.73] | 8.77 [8.72, 8.88] | -98.84% |
| unique | unsorted | 2.36 [2.32, 2.40] | 2.33 [2.26, 2.35] | -1.27% | 8.88 [8.81, 8.92] | 8.84 [8.80, 8.97] | -0.35% |
| low-hit | sorted | 0.13 [0.12, 0.13] | 0.12 [0.12, 0.13] | -7.69% | 22.72 [21.95, 22.88] | 20.48 [19.52, 21.42] | -9.83% |
| low-hit | unsorted | 0.12 [0.11, 0.12] | 0.11 [0.11, 0.12] | -8.33% | 20.73 [20.64, 20.81] | 20.95 [20.77, 21.02] | +1.06% |

Unsorted repeated wall은 0.94%, low-hit RSS는 1.06% 높았지만 양쪽 범위가 겹친다.
Low-hit wall은 0.01초 해상도이므로 증감률을 독립적인 개선으로 해석하지 않는다.
정렬 경로에서는 세 workload 모두 wall·RSS 회귀가 없었다.

## Sample bias and cache assessment

제품 변경은 캐시를 추가하지 않았다. 모든 경로 queue, per-file channel과 matcher
scratch는 search invocation과 worker에 귀속되며 후속 process에 재사용되는 전역 결과
cache가 없다. 따라서 cache hit rate는 정의되지 않는다.

반복 샘플 편중은 동일 match 행과 고유 match 행을 같은 건수로 분리해 확인했다.
Sorted RSS 감소는 98.78%와 98.84%, wall 감소는 10.74%와 9.73%로 같은 방향이었다.
반복 내용의 비정상적인 cache hit로만 설명되지 않는다. Low-hit 8,192-file 제어군도
경로 선수집 비용이 회귀를 만들지 않았다.

## Correctness and fuzzing

- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `python3 tools/readme-guard/check_readmes.py`
- `cargo +nightly-2026-07-11 fuzz build`
- `scripts/run-fuzz.sh`: 9 targets, target별 15초, timeout 5초, RSS 상한 2,048 MiB

`search_executor` fuzz target은 임의 byte input, context 0~2, channel capacity 0~3, record 수집
on/off에서 unsorted와 sorted executor의 event·summary 동등성과 span 범위를 확인한다.
요약 mode seed를 회귀 corpus에 포함했다. 전체 fuzz 실행에서 crash, product panic, timeout,
RSS 상한 초과가 없었고 `search_executor`는 90,897 units를 실행했다.

## Decision

결과 전체를 소유하던 writer 후처리를 삭제하고 path ordering을 검색 scheduling의
책임으로 옮긴다. 이로써 고매치 입력이 정상적인 출력 옵션만으로 수백 MiB의
메모리를 사용하던 구조적 DoS 표면을 제거했다. 기본 unsorted stream은 변경하지
않았다. `--sort path`는 경로 탐색 완료 전에 출력하지 않고 파일 수에 비례한 경로
메모리를 사용하는 제약을 계속 가진다.
