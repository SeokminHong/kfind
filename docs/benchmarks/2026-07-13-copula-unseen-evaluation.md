# 지정사 lattice unseen 제품 판정

측정일: 2026-07-13

구현 기준: `3a22cce`

fixture SHA-256: `d02cd5e78ebc4d02d626ead6206b3ed1dddc6d4c71d7a19543981699e45ebebd`

report JSON SHA-256: `c8dae494f11c1a5c6d2232a12b9e4a0c3031f9b299da52c38128ba914303c3b7`

## 결론

`copula-lattice` 후보는 제품 gate를 통과하지 못했다. embedded와 full-POS 모두 target-level
precision은 100.00%지만 gold recall은 65.37%로 기준 80.00%보다 낮다. 공개 제품 옵션을
추가하지 않고 지정사 homonym union과 lattice shadow 계측을 유지한다.

현재 union도 recall 72.71%로 gate에 미달한다. lattice 투영은 union의 false positive 35개를
모두 제거하지만 true positive 32개도 제거한다. 436개 gold에서 gate에 필요한 true positive는
349개이며 투영 결과는 285개로 64개 부족하다. 이 결과로 비용, threshold 또는 fixture를
변경하지 않는다.

## 측정 계약

```console
KFIND_MORPH_UNSEEN=1 scripts/benchmark-morphology.sh target/morph-unseen-report
```

- source: UD Korean-PUD r2.18 test, 1,000문장
- fixture: positive 436개, negative 485개, 합계 921개
- 제외된 source copula lemma 누락: 22개
- Korean-Kaist·KSL dev/test, Korean-GSD test와 NFC 문장 중복: 0개
- 실행: fixture digest를 먼저 검증한 뒤 warm-up 없이 backend별 1회
- 환경: Linux aarch64, 10 logical CPUs, Python 3.12.13
- report schema: 13
- adapter 오류: 0개

밀봉된 backend 평가는 이 1회로 끝낸다. 결과를 본 뒤 재실행하거나 선택 기준을 조정하지 않는다.

## 검색 품질과 정책 투영

두 kfind profile의 결과는 같다.

| 정책 | precision | recall | TP | FP | TN | FN |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 union | 90.06% | 72.71% | 317 | 35 | 450 | 119 |
| `copula-lattice` 투영 | 100.00% | 65.37% | 285 | 0 | 485 | 151 |

투영의 contextual origin 판정은 accept 389개, reject 135개, ambiguous 0개, unresolved 0개다.
positive origin은 accept 389개와 reject 96개이며 negative origin은 reject 39개다. 실제 gold
span과 정렬된 candidate는 accept 285개, reject 32개다.

## gold 오거부

| primary cause | 건수 |
| --- | ---: |
| segmented nominal competitor | 19 |
| whole-window competitor | 10 |
| unknown competitor | 2 |
| segmented other competitor | 1 |

union이 놓친 gold 119개에 lattice 오거부 32개가 더해져 최종 false negative는 151개다.

## gate 판정

| 조건 | 결과 | 판정 |
| --- | ---: | --- |
| gold recall 80.00% 이상 | 65.37% | 실패 |
| target precision 99.00% 이상 | 100.00% | 통과 |
| unresolved 0개 | 0 | 통과 |
| revised hard-negative와 기존 품질 gate | 제품 구현 없음 | 추가 판정 중단 |

recall 실패로 공개 정책의 필요조건이 깨졌으므로 제품 구현과 나머지 회귀 판정을 진행하지 않는다.
현재 제품 검색 동작과 기존 품질 기준선은 바뀌지 않는다.
