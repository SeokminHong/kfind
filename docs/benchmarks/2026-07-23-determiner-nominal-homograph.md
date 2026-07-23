# 관형사 뒤 동형 명사 판정

- 측정일: 2026-07-23
- 기준 코드 revision: `04e902649c53c00b123012b0c4067b24410a4911`
- 후보 코드 revision: `6b4b441a1343e45324085a8c411e0d96d847704f`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

`v:박다`가 `값이 한 박자 늦게 저장된다.`의 명사 `박자`에 매칭되는 결과는 strict FP이며
FPᶜ다. 같은 품사에서 형태 구조가 같은 동형 활용은 contract positive지만, `한/MM + 박자/NNG`
구조는 `박/VV + 자/EF`와 품사·인접 성분 배치로 구분할 수 있다.

Full-POS `smart`는 앞 token의 exact 관형사 분석과 현재 token의 whole 명사 분석이 함께 있을 때
whole 명사를 선택한다. 따라서 해당 hard-negative를 FP에서 TN으로 옮겼고 FPᶜ도 함께 줄였다.
`못을 박자 바로 고정됐다.`의 동사 활용, 현재 token이 관형형으로 다음 명사를 꾸미는 구조,
embedded와 `boundary=any`의 recall-first 결과는 유지했다.

Canonical, development, query matrix, robustness, human untagged와 boundary profile의 raw 및
contract-adjusted 품질 projection은 기준과 후보가 같았다. 직접 구조 판정 Criterion은 p50
-0.28%, p95 +0.74%였고 통계적으로 변화가 없었다. Morphology 측정에서는 embedded 처리량이
5.08%, full-POS 처리량이 0.79% 줄고 p95가 각각 2.12%, 1.29% 늘었다. 측정 범위가 겹치고 직접
판정 변화가 작아 품질 이득을 위해 변경을 채택한다.

## 품질

기준 코드에도 후보의 hard-negative fixture를 적용해 같은 40개 입력으로 비교했다.

| profile | 지표 | 기준 TP / FP / TN / FN | 후보 TP / FP / TN / FN |
| --- | --- | ---: | ---: |
| embedded smart | raw | 0 / 5 / 35 / 0 | 0 / 5 / 35 / 0 |
| embedded smart | contract-adjusted | 3 / 2 / 33 / 2 | 3 / 2 / 33 / 2 |
| full-POS smart | raw | 0 / 7 / 33 / 0 | 0 / 6 / 34 / 0 |
| full-POS smart | contract-adjusted | 5 / 2 / 33 / 0 | 5 / 1 / 34 / 0 |

변경된 예측은 `hard:surface:hammer-beat`의 full-POS `smart` 한 건뿐이다. Embedded는 같은
surface를 후보로 유지하고 full-POS `boundary=any`도 매칭한다. Contract review annotation을
추가하지 않았으므로 이 건은 strict와 contract-adjusted 모두 negative다.

나머지 품질 projection은 다음 영역을 포함한다.

- canonical과 development의 raw 및 contract-adjusted confusion matrix
- query matrix의 raw 및 contract-adjusted confusion matrix
- robustness와 human untagged 품질
- embedded/full-POS의 `smart`, `token`, `any` boundary 품질

Projection JSON SHA-256은 양쪽 모두
`6686823040ab0cea711f1126582f94ce7405c5cc7dc19c25509546eab07fb416`이다.

## 성능

Criterion 기본 warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의
sample별 1회 시간을 정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `structural_constraint/resolve_candidate` | 2.5431 / 2.6334 µs | 2.5360 / 2.6529 µs | -0.28% / +0.74% |

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.041008 [0.040129, 0.043725] s | 0.041424 [0.039632, 0.047671] s | +1.01% |
| embedded | cases/s | 37,121.9 [35,656.0, 38,719.4] | 35,237.3 [21,048.2, 37,658.9] | -5.08% |
| embedded | p95 | 0.0612 [0.0570, 0.0628] ms | 0.0625 [0.0581, 0.0972] ms | +2.12% |
| embedded | RSS | 42,296 [42,284, 42,304] KiB | 42,292 [42,288, 42,296] KiB | -0.01% |
| full-POS | initialization | 0.074086 [0.072954, 0.075205] s | 0.073208 [0.072780, 0.074044] s | -1.19% |
| full-POS | cases/s | 22,304.1 [22,028.7, 23,660.9] | 22,128.2 [22,108.6, 23,100.6] | -0.79% |
| full-POS | p95 | 0.1241 [0.1194, 0.1273] ms | 0.1257 [0.1206, 0.1290] ms | +1.29% |
| full-POS | RSS | 58,812 [57,984, 58,984] KiB | 58,012 [57,960, 58,700] KiB | -1.36% |

변경은 full-POS `smart`의 일부 구조 선택에만 새 분기를 추가한다. Embedded를 포함한 처리량
변화는 분기 자체의 효과로 일반화하지 않는다. Embedded와 full-POS 처리량, 양쪽 p95의 불리한
값을 회귀 판단에 포함했다.

## 입력과 산출물

- hard-negative fixture SHA-256:
  `489479a5116cff7763805c39b6ebba4208e5f3c9220586e1cdeabaf9865acb54`
- canonical fixture SHA-256:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- development fixture SHA-256:
  `30e013c981767bf1722e06547b4718bbd752ed5fb0fe7ab340802e73d987b780`
- 기준 Criterion sample SHA-256:
  `54eb7ad871f8e9f6497c2a419f2e0d2ecf63d9d9c3ba9bcabee88e2982fba3d7`
- 후보 Criterion sample SHA-256:
  `a6b745561ac4e4dcc2ac20e8fb2523d64c11874eaef8e104a0ec38c2ffd293ab`
- 기준 morphology report SHA-256:
  `c65cff885bf5c16790f10210fd5b1ebe063146984415e4ccc59be58ff0aa5dfc`
- 후보 morphology report SHA-256:
  `74dbc40843aa3bc1bb48e29edf8b223d94cd112159e98480cb10d990979ba56c`

## 재현

기준 worktree에는 후보의 hard-negative fixture만 적용해 입력을 같게 했다. 제품 코드는 각
revision 그대로 측정했다.

```console
git switch --detach 04e902649c53c00b123012b0c4067b24410a4911
# tools/morph-compare/hard-negatives.jsonl은 후보와 같은 입력을 사용한다.
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/issue-223-rebased-baseline

git switch --detach 6b4b441a1343e45324085a8c411e0d96d847704f
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/issue-223-rebased-candidate
```
