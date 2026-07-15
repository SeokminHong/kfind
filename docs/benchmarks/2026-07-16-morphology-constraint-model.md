# 형태 구조 제약 모델 독립 평가

## 결론

비용 임계값, lexical context registry, 제품 verifier와 boundary 판정을 호출하지 않는 독립 evaluator로 전체 형태 구조 제약 모델을 평가했다. prepared graph 공유, 정수 POS transition, 증분 continuation traversal과 진단 경로 분리로 5회 성능 gate를 모두 통과했지만 `possible-analysis`의 고정 test TP 461 / FP 5가 현재 제품 TP 466 / FP 0을 보존하지 못했다. 구조 실행 방식은 채택 가능 범위에 들어왔고 source identity와 ambiguity 품질 gate는 남았으므로 제품 matcher는 전환하지 않고 구현은 후속 실험용 stacked draft에 유지한다.

## 구조

`QueryMorphPattern`은 lexical identity, fine POS, span relation, continuation DFA, 인접 token 제약과 component capability를 선언한다. Schema 4 graph는 source analysis와 expression component, categorical morphotactic edge, 정수 POS class와 dense transition bit matrix를 저장하고, resolver는 packed DAG를 query-directed로 탐색해 `SupportedAnalysisSet`과 `Supported`, `Contradicted`, `Ambiguous`, `Unavailable`을 반환한다. 비용은 같은 종류의 근거 안에서 출력 순서를 정하는 진단 값일 뿐 hard edge, proof 종류와 수용 여부를 결정하지 않는다.

정규화 token graph는 같은 token의 모든 candidate가 공유하고 adjacent graph와 nominal-particle context fact는 실제 pattern이 요구할 때만 만든다. compact decision은 시작 lexical path의 unit stream을 제자리에서 확장·복원하고 borrowed suffix view로 continuation prefix를 검증하며 reverse edge, witness path와 전체 proof는 별도 diagnostic process에서만 materialize한다. 제품 control도 component resource를 로드하는 별도 process에서 case별 span만 전달하므로 구조 후보의 peak RSS에 control 고수위 메모리가 섞이지 않는다.

독립 evaluator는 query branch의 anchor만 공유하고 제품 verifier, boundary policy, lexical context registry, component cost threshold를 호출하지 않는다. 따라서 제품과 구조 정책의 일치는 같은 판정 코드를 두 번 실행한 결과가 아니며 candidate coverage, resolver outcome, 제품 정책과 성능을 따로 측정할 수 있다.

조사 continuation은 표면형의 문법 역할, 이형태 조건과 격조사 slot 수를 hard constraint로 보존한다. `이/가`, `을/를`, `으로/로`를 다른 조사 POS로 재분석해 조건을 우회할 수 없고, fused 또는 unaligned expression은 enclosing source node의 안정된 span만 반환한다. 다른 lexical identity, component 노출과 불안정한 내부 span은 각각 별도 ambiguity로 남긴다.

## 입력과 재현

| 항목 | 값 |
| --- | --- |
| candidate와 product control revision | `35351f061e68c2d5c073eae98bcd9222a898159b` |
| 명령 | `KFIND_MORPH_RUNS=5 ./scripts/benchmark-morphology.sh target/morph-hierarchical-streaming-5run` |
| 환경 | Linux 6.12.76 linuxkit, aarch64, logical CPU 10, Rust 1.97.0, Python 3.12.13 |
| 측정 | fresh process warm-up 1회 후 5회, median과 min/max |
| 고정 test fixture SHA-256 | `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff` |
| development fixture SHA-256 | `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c` |
| hard-negative fixture SHA-256 | `cb8634491cba65916c9af510c50f909eaddfd9bb89935598875e134a01cbce99` |
| graph resource SHA-256 | `c7b0c4b1f01c4d2e60f453ae63f4f24dc6af132599dfd0152cefa7380691426b` |
| 생성 `report.json` SHA-256 | `db2043b4f44a17d36a9e44ef2d85cc82d716796dc4bf7f56d48031452a9f466a` |
| 생성 `report.md` SHA-256 | `cc9cdb15870e914736d45ecacfcda340130e5ec8c9ab0b3e8abb22a3e3828bf4` |

## 품질

### 고정 test

독립 candidate coverage는 positive 500건 중 494건인 98.8%다.

| 정책 | TP | FP | TN | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 466 | 0 | 500 | 34 | 100.00% | 93.20% |
| `whole` | 102 | 0 | 500 | 398 | 100.00% | 20.40% |
| `explicit-component` | 458 | 3 | 497 | 42 | 99.35% | 91.60% |
| `possible-analysis` | 461 | 5 | 495 | 39 | 98.93% | 92.20% |
| `unambiguous-analysis` | 282 | 0 | 500 | 218 | 100.00% | 56.40% |

### Development

독립 candidate coverage는 positive 500건 중 489건인 97.8%다.

| 정책 | TP | FP | TN | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 475 | 2 | 498 | 25 | 99.58% | 95.00% |
| `whole` | 91 | 1 | 499 | 409 | 98.91% | 18.20% |
| `explicit-component` | 452 | 7 | 493 | 48 | 98.47% | 90.40% |
| `possible-analysis` | 452 | 7 | 493 | 48 | 98.47% | 90.40% |
| `unambiguous-analysis` | 286 | 0 | 500 | 214 | 100.00% | 57.20% |

### Hard-negative

| 정책 | FP | TN |
| --- | ---: | ---: |
| 현재 제품 | 4 | 18 |
| `whole` | 2 | 20 |
| `explicit-component` | 9 | 13 |
| `possible-analysis` | 10 | 12 |
| `unambiguous-analysis` | 2 | 20 |

`possible-analysis`는 제품이 놓친 positive 19건을 새로 수용했지만 제품이 맞힌 positive 24건을 구조 근거 부족으로 놓치고 negative 5건을 `CompoundExposure`로 수용했다. 손실 24건은 `Contradicted` 19건, `NoCompletePath` 4건, `UnknownOnly` 1건이며 adjective 6건, noun 4건, numeral 6건, pronoun 1건, verb 7건이다. 따라서 전역 component 정책을 다시 조정하는 것으로는 제품 품질을 보존할 수 없고 source lexical identity와 complete-path 표현력을 먼저 보강해야 한다.

## 성능

| 경로 | 초기화 median (min-max) | cases/s median (min-max) | p95 ms median (min-max) | RSS MiB median |
| --- | ---: | ---: | ---: | ---: |
| 현재 제품 full-POS `smart` | 0.4283 | 10,441.4 (9,914.6-10,508.4) | 0.2444 (0.2412-0.2530) | 92.6 |
| 독립 constraint evaluator | 0.9264 (0.9104-0.9619) | 10,189.3 (9,798.2-10,284.3) | 0.2643 (0.2595-0.2829) | 104.9 |

독립 evaluator는 제품 대비 처리량 -2.4%, p95 +8.1%, RSS +13.3%로 각각 -10%, +10%, +20% gate 안에 있다. 측정된 evaluation 0.0981초 중 compile은 0.0567초, candidate enumeration은 0.0059초, graph preparation은 0.0078초, decision은 0.0273초, policy 적용은 0.0003초다. 초기화는 0.9264초로 제품보다 크지만 성능 채택 계약의 세 지표는 모두 통과했다.

## 장점

구조 모델의 가장 큰 장점은 판정 근거의 종류와 불확실성을 관측할 수 있다는 점이다. 비용 마진은 include와 exclude 경로가 가까운 이유, component를 노출해도 되는 이유와 같은 표면의 다른 표제어를 선택한 이유를 구분하지 못하지만, resolver는 완전 경로, lexical identity, span, continuation, context와 ambiguity를 각각 증명한다. 새 surface를 registry에 추가하거나 전역 임계값을 움직이지 않고도 규칙의 적용 범위와 실패 원인을 계약과 fixture로 고정할 수 있다.

가능한 분석 집합과 제품 정책을 분리한 것도 장점이다. 형태적으로 가능한 분석을 보존하는 정책, 중의성에서 abstain하는 정책과 whole-token만 허용하는 정책을 resolver core를 바꾸지 않고 비교할 수 있다. 향후 문맥 disambiguator를 붙이더라도 morphology graph를 다시 휴리스틱 우선순위로 오염시키지 않고 `SupportedAnalysisSet`을 입력으로 사용할 수 있다.

## 문제와 개선 경로

첫째, candidate coverage가 완전하지 않다. 고정 test 6건과 development 11건은 resolver가 거부한 것이 아니라 독립 enumerator가 gold candidate를 만들지 못한 경우다. 다음 단계에서는 query compiler가 만든 선언형 anchor IR을 제품 matcher와 reference enumerator가 각각 실행하게 하고, corpus 판정 코드는 공유하지 않는 방식으로 compiler와 enumerator의 표현력 차이를 제거해야 한다.

둘째, 형태 graph가 완전 경로를 만들지 못하거나 unknown에만 의존하는 사례가 남는다. source row와 runtime node 사이의 lemma identity bridge, 생산적 lexical sequence의 생성 근거와 source alignment를 artifact에 더 보존하면 `NoCompletePath`, `UnknownOnly`와 일부 `OpaqueExpression`을 줄일 수 있다. 실제로 융합된 형태의 strict 내부 span은 정보가 존재하지 않으므로 임의 경계를 만들지 말고 enclosing span을 반환하거나 abstain해야 한다.

셋째, component 노출은 하나의 전역 정책으로 닫히지 않는다. 복합어 내부 component를 모두 허용하면 `학교 -> 대학교` 같은 FP가 생기고 모두 막으면 실제 component positive를 잃는다. source construction root, 파생·합성 관계와 query capability를 graph에 보강해 구조적으로 구분 가능한 범위를 넓히되, 같은 근거가 positive와 negative에 함께 나타나면 `CompoundExposure` ambiguity를 유지해야 한다.

넷째, 같은 표면의 다른 표제어와 의미는 형태 정보만으로 항상 해결할 수 없다. lemma identity를 보강하면 오귀속 일부는 줄일 수 있지만 동일한 형태 분석이 여러 의미를 지지하는 경우에는 문장 문맥, collocation 또는 통계적 disambiguator가 필요하다. 이 계층은 surface 예외나 morphology 비용 마진이 아니라 `SupportedAnalysisSet`을 입력으로 받아 confidence와 abstention을 명시하는 별도 정책이어야 한다.

다섯째, steady-state 성능은 제품 전환 gate를 통과했지만 초기화는 0.9264초로 제품 0.4283초보다 크다. 제품 통합 단계에서는 graph resource의 mmap·zero-copy load와 surface section lazy decode로 초기화를 줄이고, query compile이 evaluation의 가장 큰 단계이므로 제품과 구조 경로가 같은 compiled pattern을 소비하도록 중복 생성을 없애야 한다. policy 적용은 0.0003초이므로 최적화 우선순위가 아니다.

## 결정

현재 제품의 lexical context registry와 1,500 비용 마진은 구조적으로 이상적인 설계라서가 아니라 지금의 품질 기준선을 보존하는 임시 제품 계약으로 유지한다. 전체 제약 모델은 성능 채택 조건을 통과해 비용 방식 없이 실행 가능한 구조를 증명했지만 source identity와 ambiguity 품질 조건은 통과하지 못했다. 다음 실험은 source identity·alignment 보강으로 구조 근거가 없는 24개 제품 TP를 먼저 회복하고, candidate coverage 미달 6건을 닫은 뒤 남은 `CompoundExposure`와 lexical meaning ambiguity만 별도 context disambiguator에 넘기는 순서로 stacked draft를 이어간다.
