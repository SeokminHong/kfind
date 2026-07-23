# 연결 어미 `-지`와 동형 명사 판정

- 측정일: 2026-07-23
- 기준 코드 revision: `b1dc4c2db504eca8a7496435ebb3401560634f79`
- 후보 코드 revision: `a03acf61f918199aefd8e50f080c339ce9bb8c22`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

`v:주다`가 `주지 스님이 법회를 열었다.`의 명사 `주지`에 매칭되는 결과는 strict FP이며
FPᶜ다. `주/VV + 지/EC`와 `주지/NNG`는 현재 token 내부에서 경쟁하지만, 다음 token에
경쟁 없는 완성 체언 경로가 이어지는 배치는 명사 frame으로 구분할 수 있다.

Full-POS `smart`는 이 frame에서 충돌하는 용언 후보만 제외한다. `사탕을 주지 말자.`처럼
다음 token에 완성 용언 경로가 있는 경우, `나서 흥화문`처럼 다른 연결 어미를 쓰는 경우,
`단지 바람`의 부사와 어절 내부 component는 기존 판정을 유지한다. Embedded와
`boundary=any`도 기존 recall-first 결과를 유지한다.

Hard-negative full-POS의 raw FP는 6→5, FPᶜ는 1→0이다. Canonical, development,
test/development query matrix, robustness, human untagged와 boundary profile의 raw 및
contract-adjusted 행렬은 기준과 후보가 같다.

직접 구조 판정 Criterion은 p50 +1.84%, p95 +2.63%였다. Morphology 중앙값은 embedded
처리량 +1.12%, full-POS 처리량 -0.17%이고 full-POS p95는 같았다. 직접 판정 지연의 불리한
값과 full-POS 처리량의 작은 감소를 포함해도 측정 범위가 겹치고 recall 회귀 없이 FPᶜ를
제거하므로 변경을 채택한다.

## 품질

| profile | 지표 | 기준 TP / FP / TN / FN | 후보 TP / FP / TN / FN |
| --- | --- | ---: | ---: |
| embedded smart | raw | 0 / 5 / 35 / 0 | 0 / 5 / 35 / 0 |
| embedded smart | contract-adjusted | 3 / 2 / 33 / 2 | 3 / 2 / 33 / 2 |
| full-POS smart | raw | 0 / 6 / 34 / 0 | 0 / 5 / 35 / 0 |
| full-POS smart | contract-adjusted | 5 / 1 / 34 / 0 | 5 / 0 / 35 / 0 |

변경된 예측은 `hard:surface:give-abbot`의 full-POS `smart` 한 건뿐이다. Contract review
annotation은 바꾸지 않았으므로 strict와 contract-adjusted 모두 negative다.

회귀선의 full-POS 주요 행렬은 양쪽에서 다음과 같다.

| fixture | raw TP / FP / TN / FN | contract-adjusted TPᶜ / FPᶜ / TNᶜ / FNᶜ |
| --- | ---: | ---: |
| canonical | 498 / 1 / 499 / 2 | 498 / 1 / 499 / 2 |
| development | 483 / 2 / 498 / 17 | 483 / 2 / 498 / 15 |
| test query matrix | 1291 / 4 / 1292 / 5 | 1295 / 0 / 1293 / 1 |
| development query matrix | 1230 / 4 / 1262 / 36 | 1230 / 4 / 1262 / 34 |

## 성능

Criterion 기본 warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의
sample별 1회 시간을 정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `structural_constraint/resolve_candidate` | 2.5456 / 2.5869 µs | 2.5926 / 2.6549 µs | +1.84% / +2.63% |

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.039389 [0.039168, 0.040823] s | 0.039248 [0.038694, 0.039631] s | -0.36% |
| embedded | cases/s | 36,404.3 [35,867.1, 37,557.2] | 36,812.3 [34,663.1, 37,499.8] | +1.12% |
| embedded | p95 | 0.0615 [0.0603, 0.0620] ms | 0.0600 [0.0587, 0.0640] ms | -2.44% |
| embedded | RSS | 42,288 [42,288, 42,300] KiB | 42,288 [42,284, 42,296] KiB | 0.00% |
| full-POS | initialization | 0.073701 [0.072617, 0.074126] s | 0.073380 [0.071999, 0.087587] s | -0.44% |
| full-POS | cases/s | 22,697.8 [22,042.3, 23,199.7] | 22,659.2 [21,543.4, 22,839.8] | -0.17% |
| full-POS | p95 | 0.1237 [0.1214, 0.1294] ms | 0.1237 [0.1206, 0.1298] ms | 0.00% |
| full-POS | RSS | 59,308 [57,968, 59,320] KiB | 57,968 [57,916, 58,964] KiB | -2.26% |

후보 full-POS initialization의 최댓값 0.087587초와 처리량 최솟값 21,543.4 cases/s,
embedded 처리량 최솟값 34,663.1 cases/s는 기준보다 불리하다. 중앙값과 나머지 범위가
겹치므로 구조 분기의 일관된 회귀로 판단하지 않는다.

## 입력과 산출물

- hard-negative fixture SHA-256:
  `489479a5116cff7763805c39b6ebba4208e5f3c9220586e1cdeabaf9865acb54`
- canonical fixture SHA-256:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- development fixture SHA-256:
  `30e013c981767bf1722e06547b4718bbd752ed5fb0fe7ab340802e73d987b780`
- query matrix fixture SHA-256:
  `e862d8af010c23462ba3a9ebf4f1134275b68de5004bc60035565734f5f19999`
- 기준 Criterion sample SHA-256:
  `9939244eb43c86bf102ba3fca0c48408876f6d76ecb282246d01c9969621051a`
- 후보 Criterion sample SHA-256:
  `4ce4e2a8b25ae24428cc1724aec947968f371038fba910a7c7e416b2758ee0cb`
- 기준 morphology report SHA-256:
  `fd6485627cc03cc056945b301eabe3402204b07cc842b74fcb0bc77747b87de9`
- 후보 morphology report SHA-256:
  `cb446039827d8ea8a99d48996a3f014da5148c2291193c152c276cce845797c8`

## 재현

```console
git switch --detach b1dc4c2db504eca8a7496435ebb3401560634f79
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/fpc-baseline-final

git switch --detach a03acf61f918199aefd8e50f080c339ce9bb8c22
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/fpc-candidate-final
```
