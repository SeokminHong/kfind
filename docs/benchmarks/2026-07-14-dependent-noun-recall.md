# 의존명사 coarse-POS fallback recall

- 측정일: 2026-07-14
- 기준 revision: `e62f0d7`
- 후보 revision: `19c2028`
- 환경: Linux/aarch64, 10 logical CPUs, 7.7 GiB memory, Python 3.12.13, Docker 29.6.1
- Rust: 1.97.0
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture: `068f0ea1f9083dfcbdcbae9aae1d265c4c978e34c0d991b0578f64ed859c6546`
- 무품사 fixture: `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592`
- 기준 report SHA-256: `d1c15ad7b59e3dde067926f6d95a68ec64f10fae0172a54359dbb69bee08ad7c`
- 후보 report SHA-256: `1b16c980099a09965a7e17e1fa5f84d6def0e1add1d53b0a9f097a43e58ea892`

## 결론

명시적 coarse `noun`의 사전 분석이 없을 때 보통명사·고유명사·의존명사 fallback을 모두
보존한다. compact component 판정에서는 corpus의 `NNBC`를 query-side `NNB`와 같은
의존명사로 비교하되 artifact와 진단의 source tag는 바꾸지 않는다.

development `smart`에서 embedded FN은 64에서 58, full-POS FN은 60에서 59로 줄었다. test의
embedded FN도 91에서 85로 줄었으며 full-POS test, 무품사와 모든 FP는 바뀌지 않았다. 같은
candidate surface를 쓰는 `명 -> 익명이`를 hard negative로 추가했고 두 profile 모두 이를
거부했다.

`197명이`는 사전 분석이 없는 embedded fallback에서 복구된다. full-POS는 `명/NNG` 분석이
이미 있어 fallback 조건에 들어가지 않으므로 그대로 거부한다. 기존 사전 분석까지 coarse
품사의 모든 세부 품사로 넓히는 변경은 이번 범위에 포함하지 않았다.

## 품질

기준선과 후보 모두 새 hard negative를 포함한 같은 16-case fixture를 사용했다.

| fixture/profile | 기준 TP / FP / FN | 후보 TP / FP / FN | 기준 recall | 후보 recall |
| --- | ---: | ---: | ---: | ---: |
| development embedded `smart` | 436 / 2 / 64 | 442 / 2 / 58 | 87.2% | 88.4% |
| development full-POS `smart` | 440 / 2 / 60 | 441 / 2 / 59 | 88.0% | 88.2% |
| test embedded `smart` | 409 / 0 / 91 | 415 / 0 / 85 | 81.8% | 83.0% |
| test full-POS `smart` | 414 / 0 / 86 | 414 / 0 / 86 | 82.8% | 82.8% |
| hard-negative embedded/full-POS | 0 / 4 / 0 | 0 / 4 / 0 | - | - |
| 무품사 embedded `smart` | 319 / 0 / 181 | 319 / 0 / 181 | 63.8% | 63.8% |
| 무품사 full-POS `smart` | 411 / 0 / 89 | 411 / 0 / 89 | 82.2% | 82.2% |

development precision은 embedded와 full-POS 모두 99.55%다. hard-negative 16건의 기존 FP
4건과 TN 12건도 같다.

## 성능

후보를 먼저, 기준선을 다음에 실행한 재측정 쌍이다.

| profile | 지표 | 기준 median [min, max] | 후보 median [min, max] | 증감 |
| --- | --- | ---: | ---: | ---: |
| embedded `smart` | initialization | 0.285403 s [0.285315, 0.290644] | 0.284096 s [0.283610, 0.289971] | -0.46% |
| embedded `smart` | cases/s | 14,562.4 [13,954.7, 14,626.6] | 14,193.8 [13,608.9, 14,239.9] | -2.53% |
| embedded `smart` | p95 | 0.1457 ms [0.1450, 0.1550] | 0.1473 ms [0.1455, 0.1545] | +1.10% |
| embedded `smart` | peak RSS | 51,984 KiB [51,976, 51,988] | 51,988 KiB [51,968, 51,988] | +0.01% |
| full-POS `smart` | initialization | 0.429290 s [0.428623, 0.430350] | 0.430818 s [0.429025, 0.437445] | +0.36% |
| full-POS `smart` | cases/s | 13,520.9 [13,018.8, 13,538.9] | 13,438.7 [13,317.6, 13,480.0] | -0.61% |
| full-POS `smart` | p95 | 0.1832 ms [0.1796, 0.1879] | 0.1830 ms [0.1796, 0.1833] | -0.11% |
| full-POS `smart` | peak RSS | 94,136 KiB [94,072, 94,136] | 94,076 KiB [94,060, 94,132] | -0.06% |

모든 지표의 양쪽 범위가 겹친다. 중앙값 변화는 최대 2.53%이고 RSS 변화는 0.1% 미만이므로
성능 회귀로 판정하지 않는다.

## 재현

```console
docker build --file tools/morph-compare/Dockerfile \
  --tag kfind-morph-benchmark:noun-recall-candidate-19c2028 .
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --volume "$PWD/target/morph-noun-recall-candidate-19c2028:/output" \
  kfind-morph-benchmark:noun-recall-candidate-19c2028 \
  --runs 5 --output /output/report.json

docker run --rm --network none --user "$(id -u):$(id -g)" \
  --volume "$PWD/target/morph-noun-recall-baseline-e62f0d7:/output" \
  --volume "$PWD/tools/morph-compare/hard-negatives.jsonl:/input/hard-negatives.jsonl:ro" \
  kfind-morph-benchmark:noun-recall-main-e62f0d7 \
  --runs 5 --hard-negatives /input/hard-negatives.jsonl \
  --output /output/report.json
```

외부 분석기 snapshot은 test fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
