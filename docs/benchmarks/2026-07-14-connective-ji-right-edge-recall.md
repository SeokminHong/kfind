# 명시적 품사 `-지` 오른쪽 끝 recall

- 측정일: 2026-07-14
- 기준 revision: `926c426`
- 후보 revision: `3c3c0c4`
- 환경: Linux/aarch64, 10 logical CPUs, 7.7 GiB memory, Python 3.12.13, Docker 29.6.1
- Rust: 1.97.0
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture: `d75b6482ecd6c4ba43aef78efe8f806a2502f65a886c9d00c20b677019f245d0`
- 무품사 fixture: `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592`
- 기준 report SHA-256: `6de61cca745d82fd3549b72598b7643b2fed107267fe24de82327cdca0ef4bfc`
- 후보 report SHA-256: `21d2ef550eaf0b67f5ff3e14816f1eee2553489147164f209a69b411c573fb71`

## 결론

명시적 동사·형용사 품사의 `smart`에서 `ending.connective-ji` branch의 왼쪽 core 경계만
열고 완성된 token span의 오른쪽 경계는 유지했다. development의 `주다 -> 심어주지` 1건을
embedded와 full-POS 모두 복구했다. FP, 고정 test와 무품사 결과는 바뀌지 않았다.

같은 표면형의 `주다 -> 주지 스님`은 기준선부터 FP였고 후보에서도 그대로다. 새 FP를 만들지
않으면서 FN을 1건 줄였으므로 명시적 품사 `smart`의 FN 우선 품질 gate를 통과한다.

## 품질

| fixture/profile | 기준 TP / FP / FN | 후보 TP / FP / FN | 기준 recall | 후보 recall |
| --- | ---: | ---: | ---: | ---: |
| development embedded `smart` | 432 / 2 / 68 | 433 / 2 / 67 | 86.4% | 86.6% |
| development full-POS `smart` | 436 / 2 / 64 | 437 / 2 / 63 | 87.2% | 87.4% |
| test embedded `smart` | 409 / 0 / 91 | 409 / 0 / 91 | 81.8% | 81.8% |
| test full-POS `smart` | 414 / 0 / 86 | 414 / 0 / 86 | 82.8% | 82.8% |
| hard-negative embedded/full-POS | 0 / 4 / 0 | 0 / 4 / 0 | - | - |
| 무품사 embedded `smart` | 315 / 0 / 185 | 315 / 0 / 185 | 63.0% | 63.0% |
| 무품사 full-POS `smart` | 411 / 0 / 89 | 411 / 0 / 89 | 82.2% | 82.2% |

development precision은 두 profile 모두 99.54%로 유지됐다. hard-negative 15건의 기존 FP 4건과
TN 11건도 같다.

## 성능

| profile | 지표 | 기준 median [min, max] | 후보 median [min, max] | 증감 |
| --- | --- | ---: | ---: | ---: |
| embedded `smart` | initialization | 0.280666 s [0.279583, 0.282187] | 0.282672 s [0.282419, 0.288408] | +0.71% |
| embedded `smart` | cases/s | 14,655.4 [13,939.3, 14,694.5] | 14,651.3 [14,622.1, 14,716.6] | -0.03% |
| embedded `smart` | p95 | 0.1458 ms [0.1434, 0.1558] | 0.1451 ms [0.1429, 0.1457] | -0.48% |
| embedded `smart` | peak RSS | 52,252 KiB [52,240, 52,252] | 52,256 KiB [52,248, 52,256] | +0.01% |
| full-POS `smart` | initialization | 0.423553 s [0.421357, 0.429007] | 0.424799 s [0.423265, 0.428289] | +0.29% |
| full-POS `smart` | cases/s | 13,565.2 [13,232.0, 13,574.9] | 13,570.3 [13,533.9, 13,579.2] | +0.04% |
| full-POS `smart` | p95 | 0.1815 ms [0.1800, 0.1842] | 0.1808 ms [0.1795, 0.1837] | -0.39% |
| full-POS `smart` | peak RSS | 94,344 KiB [94,328, 94,348] | 94,344 KiB [94,340, 94,344] | 0.00% |

처리량, p95와 RSS는 양쪽 범위가 겹친다. initialization 중앙값 증가는 1% 미만이고 full-POS
범위도 겹치므로 성능 회귀로 판정하지 않는다.

## 재현

```console
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --volume "$PWD/target/morph-connective-ji-926c426:/output" \
  kfind-morph-benchmark:user-recall-main-926c426 \
  --runs 5 --output /output/report.json

docker build --file tools/morph-compare/Dockerfile \
  --tag kfind-morph-benchmark:connective-ji-right-edge-3c3c0c4 .
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --volume "$PWD/target/morph-connective-ji-3c3c0c4:/output" \
  kfind-morph-benchmark:connective-ji-right-edge-3c3c0c4 \
  --runs 5 --output /output/report.json
```

외부 분석기 snapshot은 test fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
