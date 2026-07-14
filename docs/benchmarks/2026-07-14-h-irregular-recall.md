# ㅎ 불규칙 core lexicon recall

- 측정일: 2026-07-14
- 기준 revision: `390d96b`
- 후보 revision: `2d24c5c`
- 환경: Linux/aarch64, 10 logical CPUs, 7.7 GiB memory, Python 3.12.13, Docker 29.6.1
- Rust: 1.97.0
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture: `d75b6482ecd6c4ba43aef78efe8f806a2502f65a886c9d00c20b677019f245d0`
- 무품사 fixture: `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592`
- 기준 report SHA-256: `6a86c38de9333e665a6ac92b4b752784dc71fad1abec30d76bd682a8ef188637`
- 후보 report SHA-256: `fc89e201b7205b3b3b8eb5edb182b540fde6595642d29f599ace0d67b9067b5d`

## 결론

core lexicon에 `어떻다`, `이렇다`, `커다랗다`의 `DropH` 분석을 추가했다. 기존 generator로
`어떤`, `이런`, `커다란`을 만들고 규칙형 `어떻은`, `이렇은`, `커다랗은`은 만들지 않는다.
full-POS 전체 표제어에 대한 철자 추정이나 새 생성 분기는 추가하지 않았다.

development `smart`에서 두 profile 모두 FN 3건을 복구했다. 고정 test `smart`, full-POS 무품사,
hard-negative 결과는 바뀌지 않았다. 고정 test의 Agent `any`는 FN 1건, embedded 무품사는 FN
4건이 줄었다. 모든 품질 경로에서 FP 증가는 없다.

## 품질

| fixture/profile | 기준 TP / FP / FN | 후보 TP / FP / FN | 기준 recall | 후보 recall |
| --- | ---: | ---: | ---: | ---: |
| development embedded `smart` | 433 / 2 / 67 | 436 / 2 / 64 | 86.6% | 87.2% |
| development full-POS `smart` | 437 / 2 / 63 | 440 / 2 / 60 | 87.4% | 88.0% |
| test embedded `smart` | 409 / 0 / 91 | 409 / 0 / 91 | 81.8% | 81.8% |
| test full-POS `smart` | 414 / 0 / 86 | 414 / 0 / 86 | 82.8% | 82.8% |
| test embedded/full-POS `any` | 479 / 11 / 21 | 480 / 11 / 20 | 95.8% | 96.0% |
| hard-negative embedded/full-POS | 0 / 4 / 0 | 0 / 4 / 0 | - | - |
| 무품사 embedded `smart` | 315 / 0 / 185 | 319 / 0 / 181 | 63.0% | 63.8% |
| 무품사 full-POS `smart` | 411 / 0 / 89 | 411 / 0 / 89 | 82.2% | 82.2% |

development precision은 embedded 99.54%, full-POS 99.55%다. hard-negative 15건의 기존 FP
4건과 TN 11건도 같다.

## 성능

| profile | 지표 | 기준 median [min, max] | 후보 median [min, max] | 증감 |
| --- | --- | ---: | ---: | ---: |
| embedded `smart` | initialization | 0.282700 s [0.282357, 0.283279] | 0.280421 s [0.279971, 0.291075] | -0.81% |
| embedded `smart` | cases/s | 14,594.8 [14,331.3, 14,645.7] | 14,585.2 [13,969.4, 14,608.3] | -0.07% |
| embedded `smart` | p95 | 0.1470 ms [0.1441, 0.1521] | 0.1464 ms [0.1439, 0.1562] | -0.41% |
| embedded `smart` | peak RSS | 52,252 KiB [52,252, 52,256] | 52,256 KiB [52,256, 52,260] | +0.01% |
| full-POS `smart` | initialization | 0.432786 s [0.424863, 0.445810] | 0.427854 s [0.426718, 0.435601] | -1.14% |
| full-POS `smart` | cases/s | 13,478.3 [12,847.5, 13,564.9] | 13,498.4 [13,149.6, 13,537.8] | +0.15% |
| full-POS `smart` | p95 | 0.1821 ms [0.1809, 0.1942] | 0.1804 ms [0.1787, 0.1873] | -0.93% |
| full-POS `smart` | peak RSS | 94,344 KiB [94,332, 94,344] | 94,340 KiB [94,336, 94,352] | 0.00% |

처리량, p95, initialization과 RSS의 양쪽 범위가 겹친다. 성능 회귀로 판정하지 않는다.

## 재현

```console
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --volume "$PWD/target/morph-h-irregular-baseline:/output" \
  kfind-morph-benchmark:h-irregular-baseline-390d96b \
  --runs 5 --output /output/report.json

docker build --file tools/morph-compare/Dockerfile \
  --tag kfind-morph-benchmark:h-irregular-recall-2d24c5c .
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --volume "$PWD/target/morph-h-irregular-candidate:/output" \
  kfind-morph-benchmark:h-irregular-recall-2d24c5c \
  --runs 5 --output /output/report.json
```

외부 분석기 snapshot은 test fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
