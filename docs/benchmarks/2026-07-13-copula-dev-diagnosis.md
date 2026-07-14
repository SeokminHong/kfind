# 지정사 lattice dev gold 진단

측정일: 2026-07-13

진단 구현: `929becb`

fixture SHA-256: `1e06951581c84f02a4013e8410c113337c1389d3dcc2028b322f887bb181b494`

report JSON SHA-256: `0cb30749bc57f705a0baedd334eb9c273e0892d1d1a94df7f630b21479ed3310`

## 결론

Korean-Kaist·KSL dev의 positive 1,601건 중 gold span과 겹치는 `EojeolLattice` candidate는
1,007건이다. lattice는 957건을 수용하고 50건을 거부했으며 ambiguous는 없다. embedded와
full-POS의 candidate, 판정, 비용과 경로는 같다.

positive case 전체의 reject 151건에는 같은 문장 안에서 gold span과 겹치지 않는 다른 지정사
candidate가 포함된다. 지정사 gold 오거부 기준선은 50건이다.

## 측정 계약

```console
KFIND_MORPH_RUNS=1 scripts/benchmark-morphology.sh \
  target/morph-copula-dev-diagnosis
```

- report schema: 12
- source: UD Korean-Kaist·KSL r2.18 dev
- cases: 2,916건, positive 1,601건, negative 1,315건
- gold 판정: lattice candidate span과 case gold span의 UTF-8 byte overlap
- 진단 비용: backend 성능 측정에서 제외
- 제품 결과: homonym union 유지

## gold candidate 판정

| profile | accept | reject | ambiguous |
| --- | ---: | ---: | ---: |
| embedded | 957 | 50 | 0 |
| full-POS | 957 | 50 | 0 |

## 오거부 원인

주된 원인 분류는 `exclude` 측 최저 비용 경로의 구조를 나타낸다. case별 JSON record는 source·raw tag,
문장, gold·candidate span, 분석 window, include·exclude 비용과 선택 경로를 보존한다.

| source/raw tag | segmented nominal | whole window | segmented other | segmented predicate | 합계 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Korean-Kaist `jp` | 29 | 10 | 2 | 0 | 41 |
| Korean-KSL `vcp` | 4 | 3 | 1 | 1 | 9 |
| 합계 | 33 | 13 | 3 | 1 | 50 |

`exclude` 측 최저 비용 경로는 `NNG + NNG` 19건, 단일 `NNG` 10건, `NP + JX` 5건이 가장 많다.
cost margin은 최소 321, 중앙값 1,438, p95 4,076, 최대 5,169다.

## 제품 경계

- dev의 정상 지정사 gold도 명사·고유명사·용언 경쟁 경로보다 비용이 높을 수 있다.
- lattice `reject`를 그대로 제품 필터로 적용하지 않는다.
- 단일 비용 threshold를 이 결과만으로 선택하지 않는다.
- `copula-lattice` 후보는 UD Korean-PUD r2.18의 밀봉된 fixture로 검증하며, gate 판정 전까지
  homonym union을 유지한다.
