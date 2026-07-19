# 구조 도달성 사실 통합

- 측정일: 2026-07-19
- 기준 코드 revision: `45196748296580e952f4dd651fcbd01bd4704a8c`
- 후보 코드 revision: `2f5db0010d01ebbe825ea45300a3687f0de479b3`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

Token graph를 준비한 뒤 nominal prefix, particle·ending suffix와 predicate-connective 경계를
구조 기능마다 다시 계산하던 경로를 없앴다. `CommonPathFacts`가 graph를 정방향과 역방향으로 한
번씩 순회하고 nominal 선택, 선행·복합 용언, 보조용언, 명사 파생, 관형 파생과 서술격 조사가 같은
도달성 사실을 참조한다. 계산 배열은 `TokenEvidence::collect`가 끝나면 폐기하며 cache나 전역
상태는 추가하지 않았다. 제품 코드인 `structure/mod.rs`는 12줄 줄었다.

매 iteration마다 4,032-edge graph를 새로 만드는 비캐시 workload는 p50/p95
6.545%/6.814%, graph 생성부터 nominal 선택까지는 3.037%/5.150% 단축됐다. 반복·고정·교대
문맥과 고유 인접 token도 모두 개선됐고, 고유 현재 token은 p50 +1.146%, p95 -3.147%였다.
따라서 반복 표본의 비정상적으로 높은 cache hit율을 채택 근거로 사용하지 않았다.

Morphology는 실행 순서를 뒤집은 세 쌍에서 full-POS 처리량 변화가 -0.724%, -0.494%,
+0.411%로 측정 범위가 겹쳐 지속적인 회귀를 관찰하지 못했다. 품질 projection은 모두 같았다.
10개 fuzz target의 4,204,243개 입력에서 crash, panic, timeout과 RSS 초과는 0건이었다.

## 구조

기존 경로는 동일한 edge graph에서 아래 상태를 소비자마다 독립적으로 다시 만들었다.

```text
nominal 선택                 → nominal prefix + particle suffix
선행·명사 파생 용언          → ending suffix
복합 용언·보조용언           → predicate-connective 경계
관형 파생·서술격 조사        → nominal prefix
```

변경 뒤에는 token 준비 단계가 공통 상태를 한 번 계산한다.

```text
EdgeGraph
  → 정방향: nominal prefix, exact nominal end, predicate-connective 경계
  → 역방향: ending suffix, particle suffix
  → 각 구조 소비자가 immutable slice를 참조
  → 결과 사실만 TokenEvidence에 보존하고 계산 배열은 폐기
```

정방향 nominal 상태는 경로에 nominal이 포함됐는지를 별도 bit로 유지해 `XPN`·`XSN`·`XR`만
이어진 경로를 nominal 경로로 잘못 승인하지 않는다. 모든 배열은 `text.len() + 1`로 고정되고 graph
edge 끝 위치는 기존 수집 단계의 token byte 범위 검사를 통과한 값만 사용한다.

## Criterion과 표본 편중

양쪽 revision에 동일한 benchmark source를 사용했다. Benchmark source SHA-256은
`bfb4d72841b3acff6f18121749104610c353ad71f4032094e79ac95fadaa4819`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의 sample별 1회 시간을
정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 짧은 구조 문맥 판정 | 4.0914 / 4.1827 µs | 4.1105 / 4.2392 µs | +0.467% / +1.350% |
| 4,032-edge graph 준비 | 0.5556 / 0.5625 ms | 0.5192 / 0.5242 ms | -6.545% / -6.814% |
| 16개 prepared preferred-path 후보 | 0.2128 / 0.2140 ms | 0.2112 / 0.2186 ms | -0.755% / +2.135% |
| particle suffix 12회 거부 | 4.4636 / 4.6024 µs | 4.4643 / 4.6008 µs | +0.016% / -0.034% |
| particle suffix 20회 거부 | 11.3746 / 11.4474 µs | 11.3540 / 11.7087 µs | -0.181% / +2.283% |
| 준비된 graph의 nominal-particle 선택 | 0.3910 / 0.4031 µs | 0.3899 / 0.4025 µs | -0.292% / -0.140% |
| dense graph 생성 + 선택 | 49.2951 / 51.0789 µs | 47.7979 / 48.4484 µs | -3.037% / -5.150% |

변경되지 않은 짧은 판정과 suffix·prepared-path 대조군의 p95가 1.35~2.28% 불리하게 이동했다.
가장 직접적인 graph 준비 개선 6.81%는 이 이동보다 크지만, 작은 대조군 변화는 개선이나 회귀로
일반화하지 않는다. Dense 입력 SHA-256은
`cd73dcd03891f218e0c807e7918fc4d290fa048b036f5a04170a7e3fdf632bb4`다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| 짧은 구조 문맥 판정 | `e4cfcc169c49c6e3355fbc38ddcbde158a27a4ca106f798f2b73e0fc117fc3e8` | `53288fc759f8b322ecfc51b465d9a319280def15698767e959c9074fbd9d92b4` |
| 4,032-edge graph 준비 | `87786b6a37a9fc8c1a74bf9cb4ba7655216b9f15c6ee7966c53acc3859468d15` | `50db74a6167b1fac612d1535d8c601924b60ee8a2d83c83c5923749c1e4d3386` |
| 16개 prepared preferred-path 후보 | `6b73fe2c810bd692986fa7658589287b3c36e722211cf3549aa51558702fc191` | `1d3e25715bf680c585e4b17549c1ec641986e681be16539773c09f8918bfcc88` |
| particle suffix 12회 거부 | `6fc1b870206a8952a71582cbb5092ef06884b30297eb5fe9b6e15e43eac629c8` | `478a2ad3cbb4018bd0aa884a3a784891929467d69ca378a5ef68adaed790fc1e` |
| particle suffix 20회 거부 | `983b9bea24701bfef32acee63fe4486c7f487721a33a732838618cbc8404d4b2` | `6f4781844bb6321b79c022bb4d561b852f89ba00c552903ae29f1456bea01816` |
| 준비된 graph의 nominal-particle 선택 | `87e82df5f7948df4087bbf2db4e1a0f1928b10a5bf7ba17705abbf69fcb734bc` | `3db58f90d9d3e8b30565206539e313f156b40e1c71ad39d89b584a0a947ffe8d` |
| dense graph 생성 + 선택 | `bbcfa6d81b95547b1b0f7f25bb740143049851dba670b3e9dce1e5b35bf2a6b9` | `9fe77970a53bb880fc5aa0aba00c60be7e43f47d61dabf8b7c0fadcedbbedf10` |

### 문맥 분포

반복 cache hit이 유리한 입력과 재사용할 수 없는 입력을 같은 benchmark source에서 분리했다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 반복 문맥 | 13.2743 / 13.7458 ms | 13.1484 / 13.2705 ms | -0.949% / -3.458% |
| 고정 인접 token | 13.0306 / 13.5015 ms | 12.9341 / 13.4031 ms | -0.741% / -0.729% |
| 교대 공백 문맥 | 13.6758 / 14.1730 ms | 13.6235 / 14.0964 ms | -0.382% / -0.540% |
| 고유 인접 token | 18.2443 / 18.9136 ms | 18.2176 / 18.8847 ms | -0.146% / -0.153% |
| 고유 현재 token 거부 | 84.5033 / 90.2694 ms | 85.4718 / 87.4291 ms | +1.146% / -3.147% |

입력과 Sample JSON SHA-256:

| workload | 입력 | 기준 sample | 후보 sample |
| --- | --- | --- | --- |
| 반복 문맥 | `9e5c3adc0037cca693d6b36db94cb759ebabac548e4b5c9103e31875ebb26035` | `7e0d62ac6ed04a9ad84c067a3940ce293e48d43e25be069905c2dd109261e161` | `623ff9aa0312710ab27a9986e0ea88073dd0323fa69566c7cbd8eef5da515cfc` |
| 고정 인접 token | `dcbee0adff204c19234c69d6f49518e85c6552cbf8f11bda8a4aeeaa2ae8846c` | `971f6caaf04b2d53fe7ff4952ef54387c6ad192fc8b9d897ec216a1d135fb5ff` | `d6d7f9481348e74ac2c86e4238bf675f816c53399eab3294d6aaadbd76c5f26a` |
| 교대 공백 문맥 | `b141deb68516b9f9a8c6b1dbb8bdba9225900b9b301b6239355e03616cc7e355` | `991968d6e055333fccd67cb2f0d7ee0591b0357af66803f0c3feadb1dff9546e` | `6439bd1d8453a860b946bfcc1474b36574c027a9db0243cc3d920b2878f96661` |
| 고유 인접 token | `e5e74147ab26571ca929d4da880a9e80202a5c6054290a71797679bd60cebcf3` | `9032f79694a345219a229d50e58b6510ee26ae5e4a0cdb5b8c7ba49841861c43` | `e0a2bc7c3b40deb87aca5111a1c6105503f1069abbc9316d6f24fa48632ef0df` |
| 고유 현재 token | `d78c2495ee5c272f32a664ce4a6c676a88f0e9abb5ebf7a6380de1c64f98606b` | `4d59658ad7fdfa326bca6b3830703003a4639b8d4c5105133300835e582f1485` | `ea22647cbe952b64309269c4cac6e918967ca7f15a1f6c9db6327a275f7ebfc8` |

이번 변경은 cache를 추가하지 않았고 dense graph는 매 iteration 새로 생성한다. 반복·고정·교대뿐
아니라 고유 인접 token도 개선됐으며 고유 현재 token의 p50 불리함을 채택 판단에 포함했다.

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 실행 순서와 시스템 변동을
확인하려고 기준→후보를 두 번, 후보→기준을 한 번 실행했다. 아래 표는 역순 실행의
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.041540 [0.041109, 0.043848] s | 0.039522 [0.039223, 0.040896] s | -4.858% |
| embedded | cases/s | 33,336.0 [31,866.8, 35,147.7] | 34,789.0 [31,152.6, 35,783.9] | +4.359% |
| embedded | p95 | 0.0591 [0.0581, 0.0630] ms | 0.0582 [0.0569, 0.0658] ms | -1.523% |
| embedded | RSS | 42,284 [42,272, 42,292] KiB | 42,280 [42,268, 42,284] KiB | -0.009% |
| full-POS | initialization | 0.074737 [0.073949, 0.076174] s | 0.074045 [0.072118, 0.076921] s | -0.926% |
| full-POS | cases/s | 21,161.4 [18,178.7, 21,262.6] | 21,248.3 [20,393.5, 21,647.5] | +0.411% |
| full-POS | p95 | 0.1263 [0.1256, 0.1476] ms | 0.1258 [0.1247, 0.1313] ms | -0.396% |
| full-POS | RSS | 57,860 [57,792, 57,980] KiB | 57,984 [57,852, 57,996] KiB | +0.214% |

반복 측정의 중앙값 변화:

| 실행 순서 | profile | initialization | cases/s | p95 | RSS |
| --- | --- | ---: | ---: | ---: | ---: |
| 기준→후보 1 | embedded | +4.535% | +4.643% | -1.977% | +0.019% |
| 기준→후보 1 | full-POS | +1.416% | -0.724% | -0.073% | +0.228% |
| 기준→후보 2 | embedded | +2.693% | -7.104% | +8.246% | 0.000% |
| 기준→후보 2 | full-POS | +0.117% | -0.494% | +0.756% | 0.000% |
| 후보→기준 | embedded | -4.858% | +4.359% | -1.523% | -0.009% |
| 후보→기준 | full-POS | -0.926% | +0.411% | -0.396% | +0.214% |

Embedded는 방향이 바뀌고 범위가 크게 겹쳤다. Full-POS 처리량도 -0.724%~-0.494%의 두 번과
+0.411%의 역순 한 번으로 방향이 바뀌어 지속적인 회귀로 판정하지 않았다. 세 쌍 모두 canonical,
development, hard-negative, query matrix, Robust, shadow와 Agent/Human workflow에서 성능·환경·version
필드를 제외한 품질 projection이 byte 단위로 같았다. Projection SHA-256은
`a0b6b6cece8282a82570c7860a15c1cf94d050f9bb724822bc82fc557bc777cb`다.

Morphology report SHA-256:

| 실행 | 기준 | 후보 |
| --- | --- | --- |
| 기준→후보 1 | `dcc59945354702cb48f53b754a6c71a77e3142e3c06eefafc0f36b8155e47235` | `902d9456a6e33c5680a62dfb16c593f905655970cbb56377d0e2c7c3663ec611` |
| 기준→후보 2 | `199a7cc328c70a08bccd139ba1cba413c48a70832578bd2bc48bfc2886eaa865` | `15b742431d381c8ade5bcc5a94d470fed36b747dbf9d359ed193df5e1ea60481` |
| 후보→기준 | `1ebc85de88e10d5057ed58217057ed4410cbc6a0bac7160e0b0d04d6c7c6b9c1` | `412151ce8a9361b60452562b3c1da5e76625f30e1ba5c325cb33e9d4abfcd26f` |

## 정확성과 안전성

대표 resource와 256-case property test에서 공통 사실을 독립적인 직접 graph 순회와 비교한다.
임의 1~12음절 token, 최대 63개 edge와 nominal·particle·접사·어근·용언·복합 ending POS를 생성해
nominal prefix, ending·particle suffix, exact nominal 끝 위치와 predicate-connective 경계가 같은지
확인한다. POS 생성에는 `VV+EC`, `VA+EP+EC`, `EP`, `EC`, `EF`, `ETM`, `VX+EF`, `XSV+ETM`을
포함한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초, RSS 상한
2 GiB로 10개 target을 실행했다. 총 4,204,243개 입력에서 crash, panic, timeout과 RSS 초과는
0건이었다. `slowest_unit_time_sec`는 모든 target에서 0이었다.

| target | 실행 입력 | peak RSS |
| --- | ---: | ---: |
| `query_lexer` | 921,597 | 557 MiB |
| `matcher_bytes` | 28,599 | 447 MiB |
| `matcher_plan` | 202,514 | 527 MiB |
| `user_lexicon` | 690,598 | 660 MiB |
| `json_output` | 315,420 | 541 MiB |
| `binary_detection` | 48,100 | 483 MiB |
| `pos_resource` | 1,590,021 | 677 MiB |
| `component_resource` | 99,058 | 405 MiB |
| `search_executor` | 67,950 | 459 MiB |
| `structural_preparation` | 240,386 | 580 MiB |

## 재현

기준과 후보 worktree의 제품 코드는 각 revision 그대로 측정했다. Benchmark source와 runner는
양쪽에서 같았다.

```console
git switch --detach 45196748296580e952f4dd651fcbd01bd4704a8c
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-morphology.sh target/morph-reachability-facts-baseline

git switch --detach 2f5db0010d01ebbe825ea45300a3687f0de479b3
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-criterion.sh 'matcher/context_'
scripts/benchmark-morphology.sh target/morph-reachability-facts-candidate
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
