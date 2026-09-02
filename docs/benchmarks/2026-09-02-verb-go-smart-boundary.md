# `가다` smart 경계 판정

- 측정일: 2026-09-02
- 기준 코드 revision: `6d2562b38e9c12d5eb891cba3b3aa900538ecfbc`
- 후보 코드 revision: `7b5a2b6d904ef125c427aa341c196bcb1abb8823`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

`v:가다`가 `되감기`, `문서가`, `가리키는지`, `민감`, `가진`에 매칭되는 결과는 strict
FP이며 FPᶜ다. 고정 MeCab source에서 `가진`은 `가지/VV + ㄴ/ETM`으로 분석된다. Query core
`가/VV`보다 긴 별도 용언 component가 있으므로 `가다`와 같은 구조의 동형 활용이 아니다.

Full-POS `smart`는 선택된 체언+조사 path, 명사형 활용의 내부 왼쪽 경계, 보조용언 suffix와
token 전체의 별도 용언 분석을 함께 판정한다. 다섯 hard-negative를 모두 TN으로 옮겼으며,
일반 활용 `간다`, `가`, `갔다`, `가는`, `갈`, `감`과 source 정렬 합성용언 `올라가`는
유지했다.

Canonical, development, query matrix, robustness, human untagged와 boundary profile의 raw 및
contract-adjusted 품질은 기준과 후보가 같았다. 직접 구조 판정 Criterion은 p50 +0.48%, p95
+1.05%였다. Morphology 측정에서는 full-POS 처리량이 1.81% 줄고 p95가 1.53% 늘었지만 측정
범위가 겹치므로 다섯 FPᶜ 제거를 위해 변경을 채택한다.

## 품질

기준 코드에도 후보의 hard-negative fixture를 적용해 같은 45개 입력으로 비교했다.

| profile | 지표 | 기준 TP / FP / TN / FN | 후보 TP / FP / TN / FN |
| --- | --- | ---: | ---: |
| embedded smart | raw | 0 / 6 / 39 / 0 | 0 / 6 / 39 / 0 |
| embedded smart | contract-adjusted | 3 / 3 / 37 / 2 | 3 / 3 / 37 / 2 |
| full-POS smart | raw | 0 / 10 / 35 / 0 | 0 / 5 / 40 / 0 |
| full-POS smart | contract-adjusted | 5 / 5 / 35 / 0 | 5 / 0 / 40 / 0 |

변경된 예측은 새 `가다` hard-negative 다섯 건의 full-POS `smart` 결과뿐이다. Embedded는
component 판정으로 앞의 네 건을 이미 거부했지만, full-POS의 정확한 whole predicate 분석이
필요한 `가진`은 recall-first 후보로 유지한다. 새 fixture에는 contract review annotation이
없으므로 다섯 건은 strict와 contract-adjusted 모두 negative다.

나머지 품질 projection은 다음 영역에서 기준과 후보가 같았다.

- canonical과 development의 raw 및 contract-adjusted confusion matrix
- test/development query matrix의 raw 및 contract-adjusted confusion matrix
- robustness와 human untagged 품질
- embedded/full-POS의 `smart`, `token`, `any` boundary 품질

## 성능

Criterion 기본 warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의
sample별 1회 시간을 정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `structural_constraint/resolve_candidate` | 2.4721 / 2.5119 µs | 2.4841 / 2.5384 µs | +0.48% / +1.05% |

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.037662 [0.037263, 0.037928] s | 0.038122 [0.037721, 0.038332] s | +1.22% |
| embedded | cases/s | 38,648.4 [38,418.1, 39,479.1] | 38,823.8 [38,219.5, 39,232.8] | +0.45% |
| embedded | p95 | 0.0556 [0.0555, 0.0568] ms | 0.0575 [0.0555, 0.0580] ms | +3.42% |
| embedded | RSS | 42,272 [42,260, 42,284] KiB | 42,284 [42,280, 42,292] KiB | +0.03% |
| full-POS | initialization | 0.069793 [0.069475, 0.070813] s | 0.071535 [0.071453, 0.072138] s | +2.50% |
| full-POS | cases/s | 23,727.2 [22,301.0, 24,017.9] | 23,298.9 [21,909.0, 23,647.6] | -1.81% |
| full-POS | p95 | 0.1173 [0.1139, 0.1245] ms | 0.1191 [0.1160, 0.1293] ms | +1.53% |
| full-POS | RSS | 58,768 [58,588, 59,512] KiB | 58,948 [58,004, 59,308] KiB | +0.31% |

초기화는 변경 경로를 실행하지 않는다. Embedded와 full-POS의 initialization, embedded p95,
full-POS 처리량과 p95의 불리한 값을 회귀 판단에 포함했다.

## 입력과 산출물

- hard-negative fixture SHA-256:
  `51b0229880144e9d228de2427dd7f20d49a4c66edc25a43f901b436e1a71326a`
- canonical fixture SHA-256:
  `59c4d84de5cbafd3b134bc132c2fcdfaac75c945323b6f2880ad7ffa6aae7cec`
- 기준 Criterion sample SHA-256:
  `bd76eea38fbd9a86df0cbaeeccc0785a46fdff69e669312391e8d9d12825e4a7`
- 후보 Criterion sample SHA-256:
  `d4aed31f3f2eea1aae5c19c101a83e49ad63a906e98012f6de7264debc4598d8`
- 기준 morphology report SHA-256:
  `76a787d1a21dd2826c70081609dde8b899433517300266b3fde0a435208c8697`
- 후보 morphology report SHA-256:
  `6476ddd286fc3a9303e167af6a261593946b26b96717b749693b58f960410d2c`

## 재현

기준 worktree에는 후보의 hard-negative fixture만 적용해 입력을 같게 했다. Docker VM의
`/tmp` 용량과 측정 대상을 분리하기 위해 container `TMPDIR`은 host-mounted 출력 디렉터리를
사용했다.

```console
git switch --detach 6d2562b38e9c12d5eb891cba3b3aa900538ecfbc
# tools/morph-compare/hard-negatives.jsonl은 후보와 같은 입력을 사용한다.
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
docker build --file tools/morph-compare/Dockerfile \
  --tag kfind-morph-benchmark:issue-277-baseline .
mkdir -p target/issue-277-baseline-report/tmp
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --env TMPDIR=/output/tmp \
  --volume "$PWD/target/issue-277-baseline-report:/output" \
  kfind-morph-benchmark:issue-277-baseline \
  --runs 5 --progress --output /output/report.json

git switch --detach 7b5a2b6d904ef125c427aa341c196bcb1abb8823
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
docker build --file tools/morph-compare/Dockerfile \
  --tag kfind-morph-benchmark:issue-277-candidate .
mkdir -p target/issue-277-candidate-report/tmp
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --env TMPDIR=/output/tmp \
  --volume "$PWD/target/issue-277-candidate-report:/output" \
  kfind-morph-benchmark:issue-277-candidate \
  --runs 5 --progress --output /output/report.json
```
