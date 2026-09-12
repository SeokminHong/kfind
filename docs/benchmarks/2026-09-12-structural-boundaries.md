# 구조 판정 책임 분리 성능 검증

## 결과

기준 `28f4d7c1`과 후보 `209427e2`의 구조 workload 9개를 같은 환경에서 비교했다.
5회 중앙값의 변화는 -1.28%~+1.73%다. 일반 후보 판정은 2.507→2.550 µs로
1.73% 증가했고, 밀집 graph 준비 중 두 경로는 1.08%·1.28% 감소했다.
내부 정리의 비용은 작지만 성능이 완전히 같다고 해석하지 않는다.

`structure/mod.rs`는 3,862줄에서 365줄로 줄었다. Resource 조회, graph 인덱스,
token 근거 준비, 구조별 경로 사실과 선택·수용을 나누었으며 공개 API는 유지한다.
함수 150개의 시그니처와 본문 token은 접근 범위·주석·서식용 인자 끝 쉼표를 제외하면 같다.
Workspace tests, Clippy, fuzz 대상 컴파일, morphology gold 590개와 걷다·걸다 stress를 통과했다.
품질 gold의 통과는 회귀 검증이며 새로운 precision·recall 측정으로 표현하지 않는다.

## 구조 workload

시간 단위는 µs/benchmark iteration이다. 각 칸은 5개 프로세스의 추정 중앙값을 다시
집계한 `median [min, max]`다. 서로 다른 workload의 비용을 합산하지 않는다.
`resolve_candidate` iteration은 두 후보 판정을 묶고, `resolve_dense_preferred_paths`는
16개 후보 순환 묶음을 측정한다. 나머지는 이름에 해당하는 준비·선택·거부 1회를 측정한다.

| workload | 기준 median [min, max] | 후보 median [min, max] | 중앙값 변화 | sample p95 변화 |
| --- | ---: | ---: | ---: | ---: |
| prepare_dense_component_token_graph | 672.997 [663.077, 691.948] | 665.717 [652.377, 673.362] | -1.08% | +0.40% |
| prepare_dense_nominal_particle_context | 25.201 [25.034, 25.305] | 24.971 [24.771, 25.139] | -0.91% | -1.68% |
| prepare_dense_token_graph | 185.630 [184.329, 186.261] | 186.114 [184.968, 186.662] | +0.26% | -1.09% |
| prepare_dense_unique_pos_token_graph | 120.066 [119.254, 120.621] | 118.525 [117.549, 119.129] | -1.28% | -1.33% |
| reject_ambiguous_particle_suffix_12 | 1.024 [1.008, 1.027] | 1.017 [1.008, 1.021] | -0.65% | -0.17% |
| reject_ambiguous_particle_suffix_20 | 2.419 [2.401, 2.443] | 2.408 [2.401, 2.425] | -0.47% | -0.38% |
| resolve_candidate | 2.507 [2.469, 2.540] | 2.550 [2.536, 2.565] | +1.73% | +0.66% |
| resolve_dense_preferred_paths | 213.439 [211.870, 217.538] | 212.642 [209.889, 214.391] | -0.37% | -1.00% |
| select_dense_nominal_particle_facts | 0.595 [0.591, 0.602] | 0.599 [0.593, 0.601] | +0.66% | +0.17% |

Sample p95는 Criterion의 각 sample에 대해 `times / iters`로 구한 100개 평균 비용 중
nearest-rank 95번째 값이다. 표의 변화율은 그 값의 프로세스 5개 중앙값을 비교한다.
개별 사용자 요청의 tail latency를 측정한 값은 아니다. Raw sample, iteration 수,
추정치와 workload별 throughput metadata는 JSON에 보존한다.

전체 benchmark 프로세스의 최대 RSS는 기준 median 53,248,000 bytes
[min 51,314,688, max 54,132,736], 후보 median 51,363,840 bytes
[min 50,970,624, max 51,658,752]다. Criterion과 모든 fixture가 포함된 값이며
kfind 검색 프로세스의 상주 메모리로 일반화하지 않는다.

## 측정 조건

- 측정일: 2026-09-12. macOS 26.6.2 arm64, Apple M1 Max, rustc 1.97.0.
- 기준: `28f4d7c1d4ec88bbd6a20484fa85003ac6ac3243`.
- 후보: `209427e27229ef7d95e6a52e70c62fd21626e0a5`.
- 양쪽 모두 `cargo bench --locked -p kfind-testkit --bench query_matcher --no-run`으로
  빌드한 동일 bench profile이다. 별도 profile override는 없다.
- 입력은 `query_matcher.rs`의 고정 구조 fixture와 합성 graph다. Source·Cargo 설정·lockfile·
  양쪽 binary·측정 runner의 SHA-256을 JSON에 기록했다.
- 양쪽 binary 각각 fresh-process warm-up 1회(`--profile-time 1`) 후 5회 측정했다.
  실행 순서는 기준→후보와 후보→기준을 교대했다. 각 measured process는 기본 설정인
  workload별 warm-up 3초, 측정 5초, 100 samples를 사용했다.
- 측정 중 다른 빌드·테스트는 실행하지 않았다. 모든 측정은 저장소 benchmark lock 안에서 수행했다.
- Resource decode와 fixture 초기화는 Criterion의 반복 측정 밖이다. 이번 변경 경로의
  graph 준비 비용은 `prepare_*` workload로 측정하며 사전·프로세스 초기화 시간은 별도 측정하지 않았다.

## 재현

각 revision에서 위 빌드 명령으로 만든 `query_matcher` 실행 파일을 각각
`target/structure-review/baseline-bench`, `target/structure-review/candidate-bench`에 둔다.
JSON의 `runner_source`를 `target/structure-review/measure.py`에 저장한 뒤 실행한다.

```sh
scripts/benchmark-run.sh run --name structural-responsibility -- \
  python3 target/structure-review/measure.py
```

정확한 실행 argv는 JSON의 `runs`에 있으며, Criterion 원본 결과는
`target/criterion/structural_constraint/<workload>/structure-<side>-<round>`에 생성된다.
