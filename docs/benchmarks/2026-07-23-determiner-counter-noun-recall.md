# 관형사와 단위명사 문맥 recall

- 측정일: 2026-07-23
- 기준 코드 revision: `e0e9d70a8c26c0601eddac64ddd2c8958da48604`
- 후보 코드 revision: `9a10cf9455ae84db8c2ec9ba87df431bc95a1558`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1

## 결론

Test query matrix의 `몇/determiner → 지난 몇 년 동안…`은 구현 대상 FNᶜ다.
Component source에는 `몇/MM`과 `년/NNB|NNBC`가 있지만, `년/XSN|XR` 동형 분석을
독립 어절 경쟁으로 보아 관형사 구조가 탈락했다.

후보는 다음 token 전체에 `NNB/NNBC` 분석이 있을 때 같은 표면의 `XSN/XR`만
독립 경쟁에서 제외한다. Predicate, modifier와 다른 독립 품사 경쟁은 계속 차단한다.
Core modifier lexicon에도 `몇/MM`을 추가해 embedded 자동 분석을 full-POS와 맞췄다.
`몇몇` 내부 prefix는 계속 거부한다.

Test query matrix full-POS는 raw FN 5→4, FNᶜ 1→0이며 FP 4, FPᶜ 0을 유지했다.
Embedded도 같은 한 건을 회수해 raw FN 100→99, FNᶜ 96→95가 됐다. 변경된
query-matrix 예측은 이 한 건뿐이다. Canonical, development, development query matrix,
hard-negative, robustness와 human untagged confusion matrix는 같다.

직접 구조 판정 Criterion은 p50 -2.42%, p95 -1.87%였다. Target query-matrix
full-POS 처리량은 +4.65%, p95는 -3.81%였다. Canonical morphology 구간에서는 두
profile의 처리량과 p95가 함께 불리했고 full-POS 처리량은 -9.57%였다. 측정 범위가
겹치고 직접 판정과 target workload에서 같은 회귀가 재현되지 않아 품질 이득을 위해
변경을 채택한다.

## 품질

| fixture | 기준 raw TP / FP / TN / FN | 후보 raw TP / FP / TN / FN | 기준 TPᶜ / FPᶜ / TNᶜ / FNᶜ | 후보 TPᶜ / FPᶜ / TNᶜ / FNᶜ |
| --- | ---: | ---: | ---: | ---: |
| canonical full-POS | 498 / 1 / 499 / 2 | 498 / 1 / 499 / 2 | 498 / 1 / 499 / 2 | 498 / 1 / 499 / 2 |
| development full-POS | 483 / 2 / 498 / 17 | 483 / 2 / 498 / 17 | 483 / 2 / 498 / 15 | 483 / 2 / 498 / 15 |
| test matrix embedded | 1196 / 4 / 1292 / 100 | 1197 / 4 / 1292 / 99 | 1200 / 0 / 1293 / 96 | 1201 / 0 / 1293 / 95 |
| test matrix full-POS | 1291 / 4 / 1292 / 5 | 1292 / 4 / 1292 / 4 | 1295 / 0 / 1293 / 1 | 1296 / 0 / 1293 / 0 |
| development matrix full-POS | 1230 / 4 / 1262 / 36 | 1230 / 4 / 1262 / 36 | 1230 / 4 / 1262 / 34 | 1230 / 4 / 1262 / 34 |
| hard-negative full-POS | 0 / 5 / 35 / 0 | 0 / 5 / 35 / 0 | 5 / 0 / 35 / 0 | 5 / 0 / 35 / 0 |

Contract review annotation은 바꾸지 않았다. Full-POS에 남은 raw FN 4건은
`gold-alignment-error` 1건과 `nonstandard-input` 3건이며, 구현 대상 FNᶜ와 미분류는
각각 0건이다.

## 성능

Criterion 기본 warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 `sample.json`의
sample별 1회 시간을 정렬한 nearest-rank p50/p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| `structural_constraint/resolve_candidate` | 2.4461 / 2.5157 µs | 2.3868 / 2.4685 µs | -2.42% / -1.87% |

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 표는
`median [min, max]`다.

### Test query matrix

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.041034 [0.040934, 0.041076] s | 0.042307 [0.041521, 0.042572] s | +3.10% |
| embedded | cases/s | 40,723.4 [39,174.6, 40,918.6] | 40,655.5 [39,651.5, 40,957.4] | -0.17% |
| embedded | p95 | 0.0537 [0.0532, 0.0544] ms | 0.0537 [0.0534, 0.0541] ms | 0.00% |
| embedded | RSS | 45,072 [45,068, 45,076] KiB | 45,072 [45,064, 45,076] KiB | 0.00% |
| full-POS | initialization | 0.074560 [0.074259, 0.076199] s | 0.074469 [0.073732, 0.077889] s | -0.12% |
| full-POS | cases/s | 24,320.9 [22,698.3, 25,585.3] | 25,451.6 [23,776.9, 25,599.0] | +4.65% |
| full-POS | p95 | 0.1051 [0.0993, 0.1149] ms | 0.1011 [0.0981, 0.1083] ms | -3.81% |
| full-POS | RSS | 58,784 [58,780, 58,792] KiB | 58,796 [58,792, 59,548] KiB | +0.02% |

### Canonical morphology

| profile | metric | 기준 | 후보 | 중앙값 변화 |
| --- | --- | ---: | ---: | ---: |
| embedded | initialization | 0.040486 [0.040257, 0.043719] s | 0.042829 [0.041909, 0.045255] s | +5.79% |
| embedded | cases/s | 41,063.6 [37,179.0, 41,459.1] | 38,237.2 [33,969.3, 40,317.9] | -6.88% |
| embedded | p95 | 0.0540 [0.0528, 0.0578] ms | 0.0580 [0.0533, 0.0643] ms | +7.41% |
| embedded | RSS | 42,280 [42,268, 42,288] KiB | 42,284 [42,280, 42,284] KiB | +0.01% |
| full-POS | initialization | 0.075866 [0.073261, 0.080758] s | 0.075061 [0.074810, 0.076829] s | -1.06% |
| full-POS | cases/s | 25,178.5 [23,255.5, 25,333.7] | 22,769.1 [22,352.3, 25,237.1] | -9.57% |
| full-POS | p95 | 0.1110 [0.1103, 0.1213] ms | 0.1187 [0.1119, 0.1225] ms | +6.94% |
| full-POS | RSS | 58,008 [57,984, 58,076] KiB | 58,052 [57,980, 59,532] KiB | +0.08% |

Canonical 구간의 불리한 처리량과 p95는 두 profile에서 함께 나타났고 모든 범위가
겹친다. 기준 initialization 중앙값도 첫 측정 0.069082초에서 후보 직후 재측정
0.075866초로 이동했다. Direct Criterion과 target query-matrix 결과를 함께 보면
구조 분기의 일관된 회귀로 판단할 근거가 없다.

## 입력과 산출물

- canonical fixture SHA-256:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- development fixture SHA-256:
  `30e013c981767bf1722e06547b4718bbd752ed5fb0fe7ab340802e73d987b780`
- test query matrix fixture SHA-256:
  `e862d8af010c23462ba3a9ebf4f1134275b68de5004bc60035565734f5f19999`
- development query matrix fixture SHA-256:
  `cc6b67f87fde082bbf6d55de7f20576da824088c2f841a37c58df73ee84f79e7`
- hard-negative fixture SHA-256:
  `489479a5116cff7763805c39b6ebba4208e5f3c9220586e1cdeabaf9865acb54`
- 기준 Criterion sample SHA-256:
  `f16ca5e086aba74f0d06edfffabbbbffd5617414954b25f595ea4a552e132601`
- 후보 Criterion sample SHA-256:
  `20d1f24d0a30ca8e64f5424007c003582378f91e1a4a9d1e223f574a50d391d0`
- 기준 morphology report SHA-256:
  `c11f7ab9cfcdfc079aa783a16ddbe53cd4fecb047c603be98857f644d509dfd1`
- 후보 morphology report SHA-256:
  `26794f58be7a6f633d562afebaa1aa458f49ece5a09e51a01a5502906d07f806`

## 재현

```console
git switch --detach e0e9d70a8c26c0601eddac64ddd2c8958da48604
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/fnc-baseline-confirm

git switch --detach 9a10cf9455ae84db8c2ec9ba87df431bc95a1558
scripts/benchmark-criterion.sh 'structural_constraint/resolve_candidate'
KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/fnc-candidate-final
```
