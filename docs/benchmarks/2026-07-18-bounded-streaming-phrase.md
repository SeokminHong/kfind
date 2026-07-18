# Bounded streaming phrase benchmark

## 대상

- baseline: `a2945679ebc87be5b4293852e8302509cf56826a`
- candidate: `6f5bc83ea061c0635f3282d1507ca83ad4832588`
- macOS 26.4.1 (25E253), Apple M1 Max
- rustc 1.97.0, cargo 1.97.0, release profile

Phrase matcher가 줄 전체의 검증 span을 atom별 `Vec`에 모은 뒤 역방향 DP를 만들던 구조를
candidate stream과 bounded active layer로 바꿨다. 각 layer는 아직 끝나지 않은 prefix와 현재
연결 가능한 prefix를 분리하고, 우선순위 heap에서 leftmost-longest 선행 경로만 조회한다. Match가
확정돼 non-overlap cursor가 전진할 때는 대기 match 끝 이후의 제한된 candidate group만 재생한다.

제품 cache는 추가하거나 변경하지 않았다.

## Criterion

양쪽 revision에서 공식 wrapper와 동일한 입력을 사용했다. Criterion 기본 warm-up 3초 뒤 100
sample을 측정했다. `sample.json`의 `times[i] / iters[i]`를 정렬한 nearest-rank p50과 p95다.

```console
scripts/benchmark-criterion.sh 'matcher/phrase_'
```

| workload | baseline p50 / p95 | candidate p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `phrase_find_all` | 784.13 / 816.57 µs | 715.52 / 742.71 µs | -8.75% / -9.05% |
| `phrase_find_all_repeated` | 124.42 / 128.57 µs | 141.00 / 148.12 µs | +13.33% / +15.21% |
| `phrase_input_searcher_repeated_line` | 1.6794 / 1.7489 ms | 1.5021 / 1.5539 ms | -10.56% / -11.15% |
| `phrase_input_searcher_repeated_line_exists` | 929.42 / 963.44 µs | 16.58 / 17.07 µs | -98.22% / -98.23% |
| `phrase_input_searcher_missing_atom_long_line` | 5.9254 / 6.1346 ms | 5.9391 / 6.1158 ms | +0.23% / -0.31% |
| `phrase_input_searcher_sparse_tail_long_line` | 62.046 / 64.522 ms | 40.614 / 41.951 ms | -34.54% / -34.98% |

입력은 다음과 같다.

| 입력 | byte | SHA-256 |
| --- | ---: | --- |
| 일반 phrase corpus | 67,840 | `3a4a988768c1ced64293e3cf3c6a850e761ba99ebdd06c17931ead0da4f82375` |
| 8원자 반복 phrase | 384 | `d52a8bc70bef97ac9c43f989776b3288d2b117525d129e9e9d23ace578efd7c1` |
| `가나` 반복 줄 | 24,577 | `2ee6a67f3b3ba05ba0c4ff04a03cf53d7b13bf694ec17176a6abe7153b82fe87` |
| 둘째 atom 누락 1 MiB 줄 | 1,048,576 | `ee6a4f2a0690110c394ef78f94104ed7498d8cd413ea88c68b6afc7c099bea80` |
| 줄 끝에 둘째 atom이 한 번 있는 1 MiB 줄 | 1,048,576 | `0cb80572a0347ba7af8a0d246041af4db46f95b623803b197f01d3f2f665350c` |

Sample JSON checksum:

| workload | baseline | candidate |
| --- | --- | --- |
| 일반 phrase | `5234aafc9ba643ffa21783a8ea3f5960555a8149e393dfe5dbabf5b88d5e375c` | `e6736a2671c338e2f1d3ba3c761e58ec0ddddeb67df4d87709215b127a5fac3e` |
| 8원자 반복 | `678d2bb8cedf6b0a801c00b999b6709d659a087ac4cdadacccef53d0736151cc` | `c33e55a3574b7bce5bc866417e50f98c1edce61f08f2c886018b90d37ff13cca` |
| 반복 줄 metadata | `01b62df5369e05312e395b62c5a76708ddcf61fd1b1e9ba698803cfab37109fa` | `699063b49bc2a7f675fd52822fd3520b135338b40adc9b09aeb5727e3bb2289a` |
| 반복 줄 summary | `d34177be7f85325527442f34db9abc40b6d8dd2885ba438ea6f2f028a54805e8` | `f9b6e8bcf42f6f337c275cac4629dd9b85d60c64493f716c72e9e1e2e9ceb488` |
| 둘째 atom 누락 | `7088e92905e5a9c206922c6dcea0ac78a8d733d08231152c50168c7f8f08b810` | `4220821af2fe8e4c4e808da19fb8103470cc82dd99bf136a3b27e4b30fbcf4da` |
| sparse tail | `34463c1dc566fffd1c74a39d4147d17900483d42932c622fb430a42909750aaf` | `00677c59325435dc7a764c691fcfb16489a8edfb823ed3da8333bbbef77673d9` |

## 긴 줄 RSS

`가 `를 반복하고 마지막 공백을 `나  `로 바꾼 4,194,304 byte 단일 줄을 사용했다. Query는
`가 나`, literal, boundary `any`, max-gap 0이며 match는 한 건이다. 입력 SHA-256은
`0c467a441a9417f5a778926fabfae4eccb5eaf0f5cfe5c3bf9f0c22d122286ed`다.

각 release binary를 fresh process로 한 번 warm-up한 뒤 5회 실행했다. `/usr/bin/time -l`의 wall과
maximum resident set size를 기록했다.

```console
perl -Mutf8 -CS -e '$s="가 " x 1048575; chop $s; print $s, "나  "' |
  /usr/bin/time -l target/release/kfind \
  --embedded --literal --boundary any --max-gap 0 '가 나' >/dev/null
```

| revision | wall min / median / max | RSS min / median / max | median 변화 |
| --- | ---: | ---: | ---: |
| baseline | 0.32 / 0.33 / 0.44 s | 232.33 / 232.41 / 234.14 MiB | - |
| candidate | 0.19 / 0.19 / 0.19 s | 16.13 / 16.19 / 16.22 MiB | wall -42.42%, RSS -93.03% |

## Cache 표본 편중

변경된 selector와 candidate stream은 호출마다 새로 만들어지고 검색 간 결과를 cache하지 않는다.
Literal 반복, missing-atom, sparse-tail workload는 structural context cache를 호출하지 않는다.
동일 matcher를 반복하는 Criterion에서는 anchor automaton의 lazy warm-up이 양쪽에 동일하게 적용되며,
이번 변경은 anchor engine을 수정하지 않았다. Fresh-process RSS 측정도 selector cache hit로 설명할 수
없는 결과다.

따라서 반복 표본의 높은 cache hit율을 성능 개선 근거로 사용하지 않았다. 일반 corpus, 둘째 atom이
없는 긴 줄, 모든 atom이 있으나 대부분 max-gap 밖인 긴 줄, fresh process RSS를 분리해 확인했다.

## 판정

줄 전체 candidate 수에 비례하던 메모리가 query atom 수와 max-gap 안의 active endpoint, 대기
match 이후의 제한된 재생 구간에 비례하도록 바뀌었다. 모든 atom이 있는 4 MiB 줄에서 RSS가 93.03%
감소했고 sparse-tail p50은 34.54% 줄었다. 일반 phrase와 실제 반복 줄도 개선됐고 summary는 첫
match가 확정되면 전체 줄 span을 만들지 않는다.

128개 동일 span에 동일 literal atom 8개를 결합하는 384 byte 조합 밀도 최악 workload는 p50
13.33%, p95 15.21% 느려졌다. 절대 증가는 p50 16.58 µs다. 이 회귀는 cache 편중이 아니라 공유
prefix와 heap 유지 비용이며 보고에서 제외하지 않는다. 짧은 합성 입력의 비용보다 긴 입력의
비례 메모리 제거, first-match 조기 종료, 일반·실제 줄 개선이 장기 안정성과 보안에 더 중요하므로
변경을 채택한다.

## 정확성과 안전성

기존 bulk selector와 streaming selector를 임의의 3원자 span 집합에서 1,024회 대조한다. `All`,
`First`, `Bounded(0..=3)`의 match와 limit 초과 여부가 모두 같아야 한다. 개발 중 발견된
non-overlap cursor 재생 반례는 proptest regression seed로 고정했다.

최종 candidate에서 formatter, workspace test, workspace Clippy `-D warnings`, fuzz manifest build를
통과했다. `nightly-2026-07-11`, `cargo-fuzz 0.13.2`, target당 15초, 입력 timeout 5초, RSS 상한
2 GiB로 8개 target을 실행했다. Crash, panic, timeout, RSS 초과는 0건이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 1,009,938 | 571 MiB |
| `matcher_bytes` | 33,617 | 450 MiB |
| `matcher_plan` | 215,327 | 538 MiB |
| `user_lexicon` | 782,405 | 654 MiB |
| `json_output` | 345,275 | 557 MiB |
| `binary_detection` | 51,883 | 471 MiB |
| `pos_resource` | 1,520,206 | 554 MiB |
| `component_resource` | 113,246 | 403 MiB |

총 4,071,897개 입력을 실행했고 모든 target의 `slowest_unit_time_sec`는 0이었다.

## 재현

```console
git switch --detach a2945679ebc87be5b4293852e8302509cf56826a
scripts/benchmark-criterion.sh 'matcher/phrase_'

git switch --detach 6f5bc83ea061c0635f3282d1507ca83ad4832588
scripts/benchmark-criterion.sh 'matcher/phrase_'
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
scripts/run-fuzz.sh
```
