# 형태 구조 제약 모델 독립 평가

## 결론

비용 임계값, lexical context registry, 제품 verifier와 boundary 판정을 호출하지 않는 독립 evaluator로 전체 형태 구조 제약 모델을 평가했다. 구조 모델은 query 의도, 가능한 corpus 분석, 중의성과 최종 제품 정책을 분리하고 `possible-analysis`의 고정 test recall을 90.6%까지 회복했지만 현재 제품의 93.2% recall과 0 FP를 보존하지 못했다. `unambiguous-analysis`는 고정 test와 development에서 0 FP를 유지하고 hard-negative FP를 제품 4건에서 3건으로 줄였지만 recall이 57.8%와 58.8%에 그쳤다. 어느 정책도 채택 조건을 통과하지 못했으므로 제품 matcher는 전환하지 않고 구현은 후속 실험용 stacked draft에 유지한다.

## 구조

`QueryMorphPattern`은 lexical identity, fine POS, span relation, continuation DFA, 인접 token 제약과 component capability를 선언한다. Schema 3 graph는 source analysis와 expression component, categorical morphotactic edge를 저장하고, resolver는 packed DAG를 query-directed로 탐색해 `SupportedAnalysisSet`과 `Supported`, `Contradicted`, `Ambiguous`, `Unavailable`을 반환한다. 비용은 같은 종류의 근거 안에서 출력 순서를 정하는 진단 값일 뿐 hard edge, proof 종류와 수용 여부를 결정하지 않는다.

독립 evaluator는 query branch의 anchor만 공유하고 제품 verifier, boundary policy, lexical context registry, component cost threshold를 호출하지 않는다. 따라서 제품과 구조 정책의 일치는 같은 판정 코드를 두 번 실행한 결과가 아니며 candidate coverage, resolver outcome, 제품 정책과 성능을 따로 측정할 수 있다.

조사 continuation은 표면형의 문법 역할, 이형태 조건과 격조사 slot 수를 hard constraint로 보존한다. `이/가`, `을/를`, `으로/로`를 다른 조사 POS로 재분석해 조건을 우회할 수 없고, fused 또는 unaligned expression은 enclosing source node의 안정된 span만 반환한다. 다른 lexical identity, component 노출과 불안정한 내부 span은 각각 별도 ambiguity로 남긴다.

## 입력과 재현

| 항목 | 값 |
| --- | --- |
| candidate와 product control revision | `5fbd91d12ed397762620e8cd724fbb034f32390a` |
| 명령 | `./scripts/benchmark-morphology.sh` |
| 환경 | Linux 6.12.76 linuxkit, aarch64, logical CPU 10, Rust 1.97.0, Python 3.12.13 |
| 측정 | fresh process warm-up 1회 후 5회, median과 min/max |
| 고정 test fixture SHA-256 | `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff` |
| development fixture SHA-256 | `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c` |
| hard-negative fixture SHA-256 | `cb8634491cba65916c9af510c50f909eaddfd9bb89935598875e134a01cbce99` |
| graph resource SHA-256 | `827384e13799473e15c0b0ff815683bc99ca41f9f6187f32cc390fa83c925362` |
| 생성 `report.json` SHA-256 | `56ad3d0d761bd10968ef0fc9ca875419d5791c7996126d340a3bac397c2eb2c9` |
| 생성 `report.md` SHA-256 | `306f3f04b4dacd7e293811ff055ea678d0e19eb07c90fac29d1f7f91471bdd97` |

## 품질

### 고정 test

독립 candidate coverage는 positive 500건 중 494건인 98.8%다.

| 정책 | TP | FP | TN | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 466 | 0 | 500 | 34 | 100.00% | 93.20% |
| `whole` | 94 | 0 | 500 | 406 | 100.00% | 18.80% |
| `explicit-component` | 450 | 4 | 496 | 50 | 99.12% | 90.00% |
| `possible-analysis` | 453 | 5 | 495 | 47 | 98.91% | 90.60% |
| `unambiguous-analysis` | 289 | 0 | 500 | 211 | 100.00% | 57.80% |

### Development

독립 candidate coverage는 positive 500건 중 489건인 97.8%다.

| 정책 | TP | FP | TN | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 475 | 2 | 498 | 25 | 99.58% | 95.00% |
| `whole` | 81 | 1 | 499 | 419 | 98.78% | 16.20% |
| `explicit-component` | 441 | 5 | 495 | 59 | 98.88% | 88.20% |
| `possible-analysis` | 441 | 5 | 495 | 59 | 98.88% | 88.20% |
| `unambiguous-analysis` | 294 | 0 | 500 | 206 | 100.00% | 58.80% |

### Hard-negative

| 정책 | FP | TN |
| --- | ---: | ---: |
| 현재 제품 | 4 | 18 |
| `whole` | 1 | 21 |
| `explicit-component` | 8 | 14 |
| `possible-analysis` | 8 | 14 |
| `unambiguous-analysis` | 3 | 19 |

## 성능

| 경로 | 초기화 median (min-max) | cases/s median (min-max) | p95 ms median (min-max) | RSS MiB median |
| --- | ---: | ---: | ---: | ---: |
| 현재 제품 full-POS `smart` | 0.4451 | 9,629.0 (9,260.7-9,928.4) | 0.2659 (0.2487-0.2696) | 92.5 |
| 독립 constraint evaluator | 1.4381 (1.4312-1.5692) | 897.6 (889.0-899.0) | 4.8469 (4.7969-4.8687) | 399.7 |

독립 evaluator는 제품보다 초기화가 3.23배, p95가 18.23배, RSS가 4.32배 크고 처리량은 10.73배 낮다. 측정된 evaluation 1.1141초 중 compile은 0.0707초, candidate enumeration은 0.0088초, resolver는 1.0350초, policy 적용은 0.0005초이며 resolver가 92.9%를 차지한다.

## 장점

구조 모델의 가장 큰 장점은 판정 근거의 종류와 불확실성을 관측할 수 있다는 점이다. 비용 마진은 include와 exclude 경로가 가까운 이유, component를 노출해도 되는 이유와 같은 표면의 다른 표제어를 선택한 이유를 구분하지 못하지만, resolver는 완전 경로, lexical identity, span, continuation, context와 ambiguity를 각각 증명한다. 새 surface를 registry에 추가하거나 전역 임계값을 움직이지 않고도 규칙의 적용 범위와 실패 원인을 계약과 fixture로 고정할 수 있다.

가능한 분석 집합과 제품 정책을 분리한 것도 장점이다. 형태적으로 가능한 분석을 보존하는 정책, 중의성에서 abstain하는 정책과 whole-token만 허용하는 정책을 resolver core를 바꾸지 않고 비교할 수 있다. 향후 문맥 disambiguator를 붙이더라도 morphology graph를 다시 휴리스틱 우선순위로 오염시키지 않고 `SupportedAnalysisSet`을 입력으로 사용할 수 있다.

## 문제와 개선 경로

첫째, candidate coverage가 완전하지 않다. 고정 test 6건과 development 11건은 resolver가 거부한 것이 아니라 독립 enumerator가 gold candidate를 만들지 못한 경우다. 다음 단계에서는 query compiler가 만든 선언형 anchor IR을 제품 matcher와 reference enumerator가 각각 실행하게 하고, corpus 판정 코드는 공유하지 않는 방식으로 compiler와 enumerator의 표현력 차이를 제거해야 한다.

둘째, 형태 graph가 완전 경로를 만들지 못하거나 unknown에만 의존하는 사례가 남는다. source row와 runtime node 사이의 lemma identity bridge, 생산적 lexical sequence의 생성 근거와 source alignment를 artifact에 더 보존하면 `NoCompletePath`, `UnknownOnly`와 일부 `OpaqueExpression`을 줄일 수 있다. 실제로 융합된 형태의 strict 내부 span은 정보가 존재하지 않으므로 임의 경계를 만들지 말고 enclosing span을 반환하거나 abstain해야 한다.

셋째, component 노출은 하나의 전역 정책으로 닫히지 않는다. 복합어 내부 component를 모두 허용하면 `학교 -> 대학교` 같은 FP가 생기고 모두 막으면 실제 component positive를 잃는다. source construction root, 파생·합성 관계와 query capability를 graph에 보강해 구조적으로 구분 가능한 범위를 넓히되, 같은 근거가 positive와 negative에 함께 나타나면 `CompoundExposure` ambiguity를 유지해야 한다.

넷째, 같은 표면의 다른 표제어와 의미는 형태 정보만으로 항상 해결할 수 없다. lemma identity를 보강하면 오귀속 일부는 줄일 수 있지만 동일한 형태 분석이 여러 의미를 지지하는 경우에는 문장 문맥, collocation 또는 통계적 disambiguator가 필요하다. 이 계층은 surface 예외나 morphology 비용 마진이 아니라 `SupportedAnalysisSet`을 입력으로 받아 confidence와 abstention을 명시하는 별도 정책이어야 한다.

다섯째, 현재 구현 비용은 제품 경로에 넣을 수 없다. resolver가 실행 시간의 92.9%를 차지하므로 graph resource의 mmap·zero-copy load, surface section lazy decode, token graph cache, 같은 token과 query의 batch 평가, continuation state의 bitset 교차와 proof 진단의 지연 생성을 우선해야 한다. 정책 적용 최적화는 전체 시간에 영향이 거의 없다.

## 결정

현재 제품의 lexical context registry와 1,500 비용 마진은 구조적으로 이상적인 설계라서가 아니라 지금의 품질 기준선을 보존하는 임시 제품 계약으로 유지한다. 전체 제약 모델은 그 계약을 제거할 수 있는 방향을 증명했지만 품질과 성능 채택 조건을 통과하지 못했다. 다음 실험은 candidate coverage 정합, source identity와 alignment 보강, resolver hot path 최적화, 별도 context disambiguator 순서로 stacked draft를 이어가며 각 단계가 독립적으로 측정 가능해야 한다.
