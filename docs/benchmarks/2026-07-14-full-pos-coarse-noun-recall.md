# Full POS coarse noun 분석 합집합 recall

- 측정일: 2026-07-14
- 기준 revision: `64f523f`
- 후보 revision: `63a75f4`
- 환경: Linux/aarch64, 10 logical CPUs, 7.7 GiB memory, Python 3.12.13, Docker 29.6.1
- Rust: 1.97.0
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture: `068f0ea1f9083dfcbdcbae9aae1d265c4c978e34c0d991b0578f64ed859c6546`
- 무품사 fixture: `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592`
- 기준 report SHA-256: `10e648254ec04b42ddbb11b32548fe522f56d83c30c87681f795395f4415808b`
- 후보 report SHA-256: `3a08e6e10b2e86ca26dd6e5208b8699fccb6492dc030d3fc6ddc2a9dbb3573e4`

## 결론

명시적 coarse `noun`에 full POS 분석이 있으면 기존 분석과 누락된 보통명사·고유명사·의존명사
fallback을 합집합으로 보존한다. user lexicon의 `replace = true`는 이 합집합보다 우선한다.

development full-POS `smart`에서 `명/NNG` 분석이 억제하던 `NNB` component 근거를 복구해
`197명이`를 찾았다. test에서는 `27일`의 단위 의존명사 `일`을 추가로 찾았다. 두 fixture의 FN이
각각 1건 줄었고 FP는 늘지 않았다. `명 -> 익명이`를 포함한 16개 hard negative와 무품사 결과도
바뀌지 않았다.

## 품질

| fixture/profile | 기준 TP / FP / FN | 후보 TP / FP / FN | 기준 recall | 후보 recall |
| --- | ---: | ---: | ---: | ---: |
| development embedded `smart` | 442 / 2 / 58 | 442 / 2 / 58 | 88.4% | 88.4% |
| development full-POS `smart` | 441 / 2 / 59 | 442 / 2 / 58 | 88.2% | 88.4% |
| test embedded `smart` | 415 / 0 / 85 | 415 / 0 / 85 | 83.0% | 83.0% |
| test full-POS `smart` | 414 / 0 / 86 | 415 / 0 / 85 | 82.8% | 83.0% |
| hard-negative embedded/full-POS | 0 / 4 / 0 | 0 / 4 / 0 | - | - |
| 무품사 embedded `smart` | 319 / 0 / 181 | 319 / 0 / 181 | 63.8% | 63.8% |
| 무품사 full-POS `smart` | 411 / 0 / 89 | 411 / 0 / 89 | 82.2% | 82.2% |

후보 development precision은 99.55%, test precision은 100.00%다. hard-negative 16건의 기존
FP 4건과 TN 12건도 같다.

## 성능

| profile | 지표 | 기준 median [min, max] | 후보 median [min, max] | 증감 |
| --- | --- | ---: | ---: | ---: |
| embedded `smart` | initialization | 0.283199 s [0.283035, 0.290261] | 0.282820 s [0.282319, 0.285930] | -0.13% |
| embedded `smart` | cases/s | 14,203.7 [13,733.4, 14,229.8] | 14,230.4 [9,051.8, 14,233.5] | +0.19% |
| embedded `smart` | p95 | 0.1483 ms [0.1467, 0.1511] | 0.1456 ms [0.1455, 0.3192] | -1.82% |
| embedded `smart` | peak RSS | 51,988 KiB [51,984, 51,992] | 51,988 KiB [51,984, 51,992] | 0.00% |
| full-POS `smart` | initialization | 0.430413 s [0.427431, 0.464438] | 0.426015 s [0.425040, 0.427944] | -1.02% |
| full-POS `smart` | cases/s | 13,481.1 [13,444.5, 13,513.0] | 12,922.3 [12,518.8, 13,298.5] | -4.15% |
| full-POS `smart` | p95 | 0.1802 ms [0.1791, 0.1813] | 0.1836 ms [0.1807, 0.1890] | +1.89% |
| full-POS `smart` | peak RSS | 94,120 KiB [94,068, 94,140] | 94,072 KiB [94,056, 94,076] | -0.05% |

full-POS cases/s는 4.15% 낮아졌고 양쪽 범위가 겹치지 않는다. p95는 1.89% 높아졌지만 범위가
겹치며 initialization은 1.02%, RSS는 0.05% 낮아졌다. full-POS에서 명시적 coarse `noun`의
대체 세부 품사 근거를 보존하는 recall 개선 비용으로 처리량 감소를 허용한다.

## 재현

```console
git switch --detach 64f523f
scripts/benchmark-morphology.sh target/morph-recall-baseline-64f523f

git switch --detach 63a75f4
scripts/benchmark-morphology.sh target/morph-recall-candidate-63a75f4
```

외부 분석기 snapshot은 fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
