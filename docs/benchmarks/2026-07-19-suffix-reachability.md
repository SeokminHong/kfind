# Suffix 위치 도달성 판정

- 측정일: 2026-07-19
- 기준 코드 revision: `bf2e86b38a932a385c6bca13290a5749d236eb7c`
- 후보 코드 revision: `3a8f049678191acd2da13b0081145cf7afb66eb5`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

Particle·ending·numeric unit·인접 token 구조 선택이 공유하는 suffix 완성 판정을 재귀 조합
탐색에서 위치 도달성 판정으로 바꿨다. 각 byte 위치는 한 번만 확장하며 호출 stack은 입력 분기
수와 무관하다. Resource가 범위를 벗어나는 길이를 돌려줘도 overflow와 범위를 확인한 뒤에만 다음
위치를 표시한다.

모든 접두 surface가 particle이고 마지막 문자는 미등록인 거부 입력에서 12회 반복은 p50/p95
98.639%/98.636%, 20회 반복은 99.986%/99.986% 단축됐다. 입력 반복 수가 12에서 20으로
늘 때 비용 증가는 250.22배에서 2.54배로 줄었다. 일반 짧은 판정은 p50 2.54%, p95 0.85%
개선됐고 다른 밀집 graph workload는 ±0.01~0.28%였다.

두 번째 morphology 측정 쌍은 embedded 처리량 +1.48%, p95 -2.78%, full-POS 처리량 -1.16%,
p95 -0.93%였다. Full-POS RSS는 +0.23%였고 첫 측정 쌍에서는 +1.53%였다. 품질 projection은
양쪽 모두 완전히 같았다.

## 구조

기존 함수는 현재 suffix에서 가능한 모든 particle 길이를 모은 뒤 각각의 나머지 suffix에 자신을
재호출했다. 완성 경로가 없고 접두 surface가 조밀하면 같은 위치를 서로 다른 분할 조합마다 다시
방문한다.

```text
recursive suffix(start)
  → start에서 가능한 모든 길이 수집
  → 각 end에 suffix(end) 재호출
  → 미등록 꼬리에서 모든 조합이 실패할 때까지 반복
```

변경 뒤 suffix byte 위치별 도달 여부만 보존한다.

```text
reachable[0] = true
for start in byte positions:
  if reachable[start]:
    허용 POS인 모든 prefix의 end를 reachable로 표시
return reachable[suffix length]
```

시간은 도달 가능한 위치에서 반환된 prefix 분석 수에 선형이고, 보조 메모리는 suffix 길이에
비례한 `Vec<bool>` 하나다. 빈 suffix의 참과 0-length 분석 무시 동작은 유지한다. 전역 cache나
결과 memoization은 없다.

## Criterion과 표본 편중

양쪽 revision에 동일한 benchmark source를 적용했다. Benchmark source SHA-256은
`7169262bc1482642ca5c6726e9b920cc26aa0516a55da625b7999a154dfbb32a`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을
정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 다분기 particle suffix 12회 거부 | 0.3182 / 0.3277 ms | 4.3317 / 4.4688 µs | -98.639% / -98.636% |
| 다분기 particle suffix 20회 거부 | 79.620 / 82.006 ms | 11.0089 / 11.3560 µs | -99.986% / -99.986% |
| 짧은 구조 문맥 판정 | 4.2673 / 4.3397 µs | 4.1590 / 4.3026 µs | -2.54% / -0.85% |
| 4,032-edge graph 준비 | 0.5305 / 0.5507 ms | 0.5292 / 0.5491 ms | -0.24% / -0.28% |
| 16개 밀집 preferred-path 후보 | 0.20519 / 0.21153 ms | 0.20521 / 0.21152 ms | +0.01% / -0.01% |

병리 resource는 `가/VV`, `다/EF`와 `나`의 모든 접두 surface에 대한 `JX` 분석을 포함한다.
입력은 `가다` 뒤에 `나`를 12회 또는 20회 붙이고 미등록 `끝`으로 끝낸다. 각 iteration이 실제
거부 판정을 다시 수행하며 cache나 공유 상태가 없다. 두 입력을 함께 측정해 특정 반복 수의 절대
시간만이 아니라 입력 증가에 따른 비용도 평가한다. 12회/20회 입력 SHA-256은 각각
`a4d4fbee42c90a9751ece360f2ee3000cea8d2139892f7a4689eaac547e38080`,
`5272b166fa8abfcdc11fcfbd8a087d6f1513b65ac835fd99dda2119545eebef5`다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 다분기 particle suffix 12회 거부 | `dc3769816182b83c9f93c1868d0f4d5034fb86fd0cb5a2e5b4403734ef831f25` | `d3345e7e7ad8cf0922e24ae2b047f64466e1c595d0fb63a4a41a62ca36edb468` |
| 다분기 particle suffix 20회 거부 | `d2c18310f24bc6a266bfeee290780bb74d3b4399440fe467c5d93c74b4e2ea34` | `70176361133c6a24320676e0c1b3c96e16c404a6f65b4231e641766474f14df3` |
| 짧은 구조 문맥 판정 | `48ec27f1856f1f4113cbdd856361d90feb1aa53cb738f6e4575c97168ca84b85` | `5cb4f61866c8493009e0d6bf6344a69c02129d13b6535eaeae32ed726265cfb1` |
| 4,032-edge graph 준비 | `8c62adbfb579650500dd71d8a7873a82082babe7833d991915e5fbd73323681a` | `3767f932e63ed00cb51a93c49ff9192e84262e25f2dc8dfb6a0255ef400f9fe5` |
| 16개 밀집 preferred-path 후보 | `6d954600e60cccb4f6fb4ed71ba66ab749358f030dbe60764632a8876beff146` | `fb63af991bf463f906920dc4d6809144d70a30e08d656de942dd1c03886ab8d7` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. RSS 변동을 확인하기 위해
기준과 후보를 각각 한 세트 재측정했으며 표는 두 번째 측정의 `median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.039729 [0.039330, 0.040856] s | 0.039456 [0.039234, 0.040502] s | -0.69% |
| embedded | cases/s | 32,961.8 [31,458.6, 34,387.4] | 33,450.4 [31,273.0, 34,300.2] | +1.48% |
| embedded | p95 | 0.0612 [0.0585, 0.0633] ms | 0.0595 [0.0591, 0.0640] ms | -2.78% |
| embedded | RSS | 42,276 [42,276, 42,288] KiB | 42,280 [42,276, 42,284] KiB | +0.01% |
| full-POS | initialization | 0.072491 [0.071896, 0.073386] s | 0.071705 [0.071379, 0.072377] s | -1.08% |
| full-POS | cases/s | 20,700.8 [20,194.9, 20,803.7] | 20,459.8 [20,036.5, 20,737.1] | -1.16% |
| full-POS | p95 | 0.1292 [0.1281, 0.1318] ms | 0.1280 [0.1263, 0.1322] ms | -0.93% |
| full-POS | RSS | 57,924 [57,852, 57,932] KiB | 58,060 [57,992, 58,812] KiB | +0.23% |

첫 측정 쌍의 full-POS RSS는 기준 57,856 [57,792, 57,916] KiB, 후보 58,744
[58,028, 59,716] KiB로 +1.53%였다. 재측정에서 증가는 축소됐지만 방향이 같으므로 0으로
간주하지 않는다. 제품 RSS 회귀 경고 기준 20%보다 작고, 지수 시간과 호출 stack 제거를 우선한다.

Canonical, development, hard-negative, query matrix, Robust, shadow와 Agent/Human workflow에서
성능·환경 필드를 제외한 품질 projection은 기준과 후보가 byte 단위로 같았다. Projection
SHA-256은 양쪽 모두
`7d5cc7dd21bb79bd3d01f55ad882792988b10fe95bead2917717b0dcee753543`다. 재측정 기준/후보
morphology report SHA-256은 각각
`ff146d0fdbbc116e8aadec176c744d4f937eb27f7c6ecbeeab48bd3c3f1d4121`,
`dc61d373c457cd2bcde52cbbd78ced810dc17bb859af4a7367ed7b4234fa236a`다.

## 정확성과 안전성

제품 분석 창의 최대 normalized scalar 수와 같은 64개 음절에 모든 접두 particle 분석을 등록하고
미등록 꼬리를 붙인 회귀 테스트를 추가했다. 이 입력은 완성 경로가 없어도 유한한 위치 배열만
순회해 거부된다. `checked_add`와 suffix 범위 검사를 통과한 non-zero prefix만 다음 위치로 사용한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 총 4,637,379개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다. `slowest_unit_time_sec`는 모든 target에서 0이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 1,028,877 | 567 MiB |
| `matcher_bytes` | 30,142 | 452 MiB |
| `matcher_plan` | 224,221 | 536 MiB |
| `user_lexicon` | 782,448 | 656 MiB |
| `json_output` | 344,424 | 567 MiB |
| `binary_detection` | 52,741 | 468 MiB |
| `pos_resource` | 1,722,332 | 654 MiB |
| `component_resource` | 116,795 | 404 MiB |
| `search_executor` | 94,774 | 475 MiB |
| `structural_preparation` | 240,625 | 581 MiB |

## 재현

기준 worktree에는 benchmark와 문서 계약만 적용하고 제품 코드는 최신 `origin/main` 그대로
측정했다.

```console
git switch --detach bf2e86b38a932a385c6bca13290a5749d236eb7c
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-suffix-reachability-baseline-rerun

git switch --detach 3a8f049678191acd2da13b0081145cf7afb66eb5
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh target/morph-suffix-reachability-candidate-rerun
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
