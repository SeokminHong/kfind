# 지정사 lattice blind 평가

측정일: 2026-07-13

구현 기준: `7d233a6`

fixture SHA-256: `4be12e060c4bc3faf35b78bb3c9189cafb49e7c885108383c0dd1fb5aeb1b188`

report JSON SHA-256: `fab077bc4d9b76a0d4e75977e8af0e8ffea8f702612e9c2a8e280ac56c1f076a`

## 결론

중복 제거한 candidate 기준으로 schema 3 lattice는 gold target 142개 중 127개를 수용하고
non-gold target 101개 중 97개를 거절했다. target-level precision은 96.95%, recall은
89.44%, F1은 93.04%다.

non-gold 구분력은 확인했지만 정상 gold도 최소 13개 거절한다. P3 local disambiguation은
계속 보류한다. 이 결과로 비용, threshold, fixture 가중치나 검색 결과를 변경하지 않는다.
Korean-GSD fixture는 이 측정부터 regression baseline이다.

## 측정 계약

```console
KFIND_MORPH_BLIND=1 scripts/benchmark-morphology.sh target/morph-blind-report
```

- source: UD Korean-GSD r2.18 test, 989문장
- fixture: VCP/VCN positive 321개, surface-cue negative 460개
- 기존 Korean-Kaist·KSL dev/test와 NFC 문장 중복: 0개
- 실행: warm-up 없이 backend별 1회
- 환경: Linux aarch64, 10 logical CPUs, Python 3.12.13
- adapter 오류: 0개

## union 검색 품질

이 표는 현재 union 검색 결과다. lattice shadow 판정은 결과를 필터링하지 않았다.

| backend | precision | recall | F1 | TP | FP | TN | FN |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded | 68.66% | 46.42% | 55.39% | 149 | 68 | 392 | 172 |
| kfind full-POS | 68.66% | 46.42% | 55.39% | 149 | 68 | 392 | 172 |
| Kiwi 0.23.2 | 96.23% | 95.33% | 95.77% | 306 | 12 | 448 | 15 |
| Lindera 4.0.0 | 87.75% | 95.95% | 91.67% | 308 | 43 | 417 | 13 |

embedded와 full-POS의 검색 결과와 lattice evidence는 같았다. query-side 사전 coverage는 이
slice의 차이를 설명하지 않는다.

## lattice 판별력

raw report에는 255개 case의 candidate hit 319개가 있다. occurrence별 positive case가 같은
문장을 반복하므로 동일한 `(sent_id, query, POS, target span)` 76개가 중복된다.

| 집계 | gold accept | gold reject | non-gold accept | non-gold reject |
| --- | ---: | ---: | ---: | ---: |
| occurrence case 기준 | 127 | 15 | 67 | 110 |
| 문장 gold 합집합·candidate 중복 제거 | 127 | 15 | 4 | 97 |

보조 집계는 같은 문장의 모든 gold occurrence span을 합친 뒤 candidate를 중복 제거한다. 제품
판별력은 이 값을 사용한다. occurrence case 기준 수치는 기존 report schema와의 재현성을 위해
함께 남긴다.

| 지표 | 값 |
| --- | ---: |
| target-level precision | 96.95% |
| gold target accept | 89.44% |
| non-gold target reject | 96.04% |
| target-level F1 | 93.04% |

## cost margin

| target | N | min | p25 | median | p75 | p95 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| gold | 142 | 22 | 2,313 | 2,539 | 3,547 | 4,899 | 6,156 |
| non-gold | 100 | 345 | 3,772 | 6,150 | 7,520 | 7,520 | 7,755 |

non-gold 1개(`신페인` 내부 `인`)는 query 포함 완전 경로가 없어 margin 없이 reject됐다. 두
분포는 중앙값에서 갈리지만 겹친다. 이 blind 분포로 threshold를 선택하지 않는다.

## 오류 분석

non-gold 오수용 4개는 모두 Korean-GSD VCP case다.

| surface/window | source 분석 | lattice 선택 경로 | 건수 |
| --- | --- | --- | ---: |
| `이` / `이어` | MAG 또는 VV+EC | VCP / EC | 3 |
| `인` / `무술인` | NNG | NNG / VCP+ETM | 1 |

gold 거절 15개는 surface `인` 9개, `이` 3개, `일` 3개다. exclude 최적 경로는 NNG 6개,
NP/JX 3개, NNG/NNG 3개와 기타 3개다. `생과일`의 `XPN+NNG+VCP+ETM`, `모바일`의
`MM+NNB+VCP+ETM`은 원문 표면과 맞지 않는 source 분해이므로 gold/adapter 이상 2개로
분리한다. 나머지 13개는 정상 VCP gold인데 lattice가 체언 경로를 더 낮게 평가했다.

## 판정

- P3 제품 필터링은 진행하지 않는다.
- Korean-GSD 결과를 이용해 비용이나 threshold를 조정하지 않는다.
- 정상 gold reject 원인은 기존 Kaist·KSL dev에서 다시 분류한다.
- 다음 제품 판정은 UD Korean-PUD r2.18의 밀봉된 fixture로 수행한다.

검증 판정은 caveat와 함께 공유 가능이다. source·fixture·report digest, case 합계, confusion
matrix와 candidate 중복 제거 집계를 독립 재계산했다. 이 측정은 품질 판별용이며 성능 수치를
보고하지 않는다.
