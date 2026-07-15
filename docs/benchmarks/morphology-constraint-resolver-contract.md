# 형태 구조 제약 resolver 계약

## 목적

`smart` 검색의 corpus 판정을 surface registry, 비용 차이와 예외 우선순위에서 분리한다. query compiler는 검색 의도를 구조 제약으로 선언하고 corpus resource는 가능한 형태 분석을 보존하며 resolver는 지지 분석 집합과 불확실성을 반환한다. 최종 검색 수용 여부는 resolver가 아니라 명시적인 제품 정책이 결정한다.

## 질의 모델

`QueryMorphPattern`은 다음 값을 갖는다.

| 필드 | 계약 |
| --- | --- |
| lexical identity | query 분석에서 나온 한 표제어이며 corpus surface 목록이 아니다. |
| fine POS | query 분석의 세부 품사다. |
| span relation | query core가 분석 전체, source component 또는 runtime component 중 어디에 대응해야 하는지 선언한다. |
| candidate spans | `core`, `anchor`, verifier가 소비한 `consumed`, surrounding `token`의 포함 관계를 선언한다. |
| continuation | core 뒤에 허용되는 형태 범주를 유한 상태 전이로 선언한다. |
| adjacent token constraints | 이전·다음 token과의 동일성, 시작·완전 POS sequence 조건을 선언한다. |
| component capability | whole-only, source-component, runtime-component 중 caller가 요청한 노출 범위를 선언한다. |

`CandidateSpans`는 `core ⊆ anchor ⊆ consumed ⊆ token` 불변식을 갖는다. exact branch의 `consumed`는 `anchor`와 같고 predicate와 nominal branch의 `consumed`는 verifier가 형태 continuation을 소비한 끝까지다. 독립 평가 경로에서는 verifier를 호출하지 않고 token graph가 같은 continuation 끝을 증명해야 한다.

nominal branch는 조사 0개인 component 후보와 token 끝까지 이어지는 조사 연쇄 후보를 별도로 만든다. 전자는 `consumed == anchor`이며 뒤의 compound node를 조사로 해석하지 않고, 후자는 `consumed == token`이며 suffix 전체가 조사 DFA를 통과해야 한다.

continuation은 `Exact`, `Predicate`, `NominalParticles` 세 DFA로 표현한다. `Exact`는 core 뒤의 형태 continuation을 허용하지 않는다. `Predicate`는 predicate lexical node 뒤의 선어말어미·종결어미·연결어미·관형형·명사형과 계약된 nominal-particle 전이를 허용한다. `NominalParticles`는 nominal lexical node 뒤의 조사 연쇄만 허용한다. DFA는 query rule graph에서 생성하며 corpus surface나 비용을 포함하지 않는다.

같은 branch로 합쳐진 pattern은 합집합으로 한 번에 평가한다. 개별 pattern이 서로를 모순으로 만들지 않으며 동일한 lexical identity와 fine POS를 지지하는 분석은 모두 보존한다.

## 형태 분석 그래프

`TokenAnalysisGraph`는 schema 3 source analysis에서 다음 근거를 구분한다. schema 3은 schema 2의 source analysis와 component projection에 hard morphotactic transition table을 추가한 실험 resource이며 전체 resolver는 schema 2를 fallback으로 사용하지 않는다.

| 근거 | 의미 |
| --- | --- |
| `source-whole` | source row가 token 전체 분석을 명시한다. |
| `source-component` | `span-aligned` expression이 candidate span과 POS를 명시한다. |
| `runtime-composed` | source node를 hard morphotactic edge로 이어 완전한 token 경로를 구성한다. |
| `opaque-expression` | `fused` 또는 `unaligned` expression이 lexical identity와 POS는 명시하지만 안정된 span을 제공하지 못한다. |
| `unknown` | known complete path가 없을 때 unknown model로 구성한 경로다. |

hard morphotactic edge는 source expression과 multi-POS row에서 관찰한 인접 `end_pos -> start_pos` 관계를 build 시점에 중복 제거해 저장한다. runtime path는 이 categorical edge가 있을 때만 source node를 연결한다. dense connection matrix의 셀 존재 여부는 edge로 사용하지 않으며 connection cost와 word cost는 같은 근거 종류 안의 출력 순서와 진단에만 사용한다.

source row 하나가 token 전체를 덮으면 그 자체로 complete analysis다. runtime composition은 모든 인접 node가 hard edge로 연결되고 token을 빈틈없이 덮을 때만 complete analysis다. source expression의 component sequence는 한 source analysis 안에서 평가하며 임의의 component를 서로 다른 source row와 섞지 않는다.

resolver는 query와 무관한 complete path를 먼저 열거하지 않는다. token graph를 start/end reachability가 표시된 packed DAG로 유지하고 query lexical support와 continuation DFA를 graph에 교차시켜 지지 분석만 탐색한다. graph 상한은 source node 수와 서로 다른 지지 proof 수에 적용하며, query와 무관한 경로 조합 수가 상한을 소모해서는 안 된다.

runtime lexical support는 query core를 덮는 연속 node sequence로 표현할 수 있다. 단일 `VV` node뿐 아니라 query 분석이 허용한 `NNG+XSV`, `XR+XSV` 같은 생산적 lexical sequence도 하나의 lexical identity proof가 될 수 있으며, 단순히 POS edge가 연결된다는 이유만으로 임의 sequence를 lexical identity로 승격하지 않는다.

`fused` 또는 `unaligned` component는 내부 byte 경계를 만들지 않는다. 다만 query anchor와 반환할 `consumed` span이 해당 source node 전체를 포함하면 canonical component sequence를 lexical identity proof로 사용할 수 있다. 이 경우 proof는 opaque source relation을 기록하되 반환 span은 enclosing node의 안정된 경계를 사용하며, source node의 strict subspan을 반환해야 할 때만 `OpaqueExpression` ambiguity가 된다.

## bounded token context

resolver 입력은 이전·현재·다음 token의 graph와 현재 candidate spans를 포함한다. 인접 token 제약은 정규화된 token 동일성, 완전 POS sequence, 시작 POS 집합으로 표현하며 세 token 범위를 넘지 않는다. 반복 token, `VCN+EC` 앞 문맥과 `NNB/NNBC` 뒤 문맥은 이 구조 제약으로 표현하고 lexical surface registry로 적용 대상을 선택하지 않는다.

현재 token에 체언 host와 조사 suffix의 유일한 split이 있으면 그 host 전체만 nominal particle context의 대상이다. 같은 token의 adverb whole 분석이나 host 내부 strict component는 조사와 결합한 분석으로 승격하지 않는다. split이 둘 이상이면 하나를 강제하지 않고 ambiguity로 남긴다.

context 제약은 만족 가능한 현재 분석을 줄일 뿐 새로운 분석을 만들거나 비용으로 하나를 강제 선택하지 않는다. 필요한 인접 token이 없거나 graph가 unavailable이면 해당 제약이 필요한 pattern은 지지되지 않거나 원인이 명시된 `Unavailable`이 된다.

인접 token selector는 관측된 구조가 실제로 현재 token의 분석 역할을 유일하게 제한할 때만 hard constraint로 작동한다. 같은 token 반복이나 POS frame이 관측됐다는 사실만으로 해당 구조를 요구하지 않은 pattern을 일괄 제거하지 않으며, 여러 구조가 동시에 가능하면 가능한 분석과 proof를 보존한다.

## resolver 결과

`ConstraintResolver`는 먼저 `SupportedAnalysisSet`을 만든다. 각 `SupportedAnalysis`는 pattern index, lexical evidence, span relation, continuation 전이, context proof와 complete path witness를 포함한다. source analysis identity, lexical sequence, continuation 또는 context proof가 다르면 버리지 않으며, 이 값이 모두 같은 경로의 무관한 prefix segmentation만 다를 때는 하나의 packed proof로 합친다.

| 결과 | 조건 |
| --- | --- |
| `Supported` | pattern의 lexical identity, fine POS, span relation, continuation과 context를 모두 만족하는 complete known analysis가 하나 이상 있다. |
| `Contradicted` | complete known analysis는 있지만 어떤 pattern도 만족하지 않는다. |
| `Ambiguous` | 지지 분석은 있지만 요청한 span 또는 component 노출 관계를 안정적으로 하나로 정할 수 없다. |
| `Unavailable` | resource 오류, source mismatch, 잘못된 span, graph 상한 초과, unknown-only 또는 complete path 부재다. |

query와 무관한 동형 분석의 존재는 지지 분석을 모순으로 만들지 않는다. 동일한 관측 근거가 서로 다른 lexical identity나 의미를 함께 지지하면 resolver는 가능한 분석을 모두 반환하며 의도한 의미 하나를 자동 선택하지 않는다.

## 제품 정책

resolver core는 검색 수용 여부를 반환하지 않는다. caller는 다음 정책 중 하나를 명시한다.

| 정책 | 수용 범위 |
| --- | --- |
| `whole` | query core와 consumed span이 token 전체를 이루는 지지 분석만 수용한다. |
| `explicit-component` | pattern의 component capability가 허용한 source 또는 runtime component만 추가로 수용한다. |
| `possible-analysis` | 안정된 span을 가진 모든 지지 분석을 수용하고 의미 중의성을 보존한다. |
| `unambiguous-analysis` | 안정된 지지 분석이 있고 resolver 결과가 `Supported`일 때만 수용한다. component exposure나 lexical competition에서는 abstain한다. |

`fused`·`unaligned` 관계에는 임의 span을 부여하지 않는다. 통계적 또는 문맥적 disambiguator를 도입하기 전에는 형태 분석만으로 의도한 의미를 자동 선택했다고 주장하지 않는다.

지지 분석의 반환 span이 surrounding token의 strict subspan이면 `CompoundExposure`, query와 다른 fine POS의 whole-token source 분석이 함께 존재하면 `LexicalCompetition` ambiguity로 기록한다. `possible-analysis`는 이를 가능한 분석으로 수용할 수 있지만 `unambiguous-analysis`는 거부한다.

## 평가 계약

reference candidate enumerator는 branch anchor를 원문에서 직접 찾고 surrounding token을 추출한다. 기존 `verify_branch_without_boundary`, boundary 판정, lexical context registry와 비용 lattice를 호출하지 않는다. query branch 생성 결과는 공유할 수 있지만 corpus candidate 검증은 graph와 pattern만으로 수행한다.

평가는 다음 단계를 분리해 기록한다.

1. candidate coverage: positive gold span에 독립 candidate가 생성됐는지 측정한다.
2. resolver conditional quality: 생성된 candidate 중 `Supported`, `Contradicted`, `Ambiguous`, `Unavailable`과 지지 evidence 종류를 측정한다.
3. policy quality: `whole`, `explicit-component`, `possible-analysis`, `unambiguous-analysis` 각각의 precision, recall, hard-negative 결과를 측정한다.
4. disagreement: 기존 제품 판정과 각 정책의 case-level 차이를 원인별로 기록한다.
5. performance: graph load, 독립 candidate enumeration, resolver, policy 적용을 product matcher와 같은 입력에서 별도 측정한다.

development와 hard-negative만 구조 선택에 사용하고 고정 test는 구조 확정 뒤 회귀 판정에만 사용한다. candidate coverage 누락을 resolver false negative로 합치지 않으며 ambiguity와 abstention을 false와 구분해 보고한다.

공식 성능 결과는 full-POS profile과 고정 test fixture를 사용해 fresh process warm-up 1회를 버린 뒤 5회를 측정한다. evaluator cases/s와 p95는 query compile, 독립 candidate 준비, resolver와 policy 적용을 포함하고 product control과 diagnostic serialization은 제외하며, initialization과 단계별 시간 및 peak RSS는 별도로 기록한다.

## 제품 전환 조건

제품 전환 뒤 query compiler는 `ContextRequirement` 대신 `QueryMorphPattern`을 만들고 matcher는 resolver 결과와 한 제품 정책만 소비한다. lexical surface registry, 1,500 비용 마진, registered-prefix raw fallback, predicate exact-token 예외와 bounded context의 강제 분석 선택을 제품 경로에서 제거한다.

`token`, `any`, literal과 형태 구조가 필요 없는 branch는 graph resource를 읽지 않는다. graph가 필요한 `smart` plan은 graph resource 누락·손상·source mismatch와 상한 초과를 fallback하지 않고 관측 가능한 오류로 반환한다.

전환 후보는 development와 고정 test의 기존 true positive를 보존하고 새 false positive를 만들지 않으며 hard-negative를 악화하지 않아야 한다. 모든 resolver 결과, continuation DFA, adjacent token 제약, component capability와 hard-edge 거부가 fixture로 검증돼야 한다. 채택 조건을 통과하지 못하면 전체 구현과 평가 근거는 유지하되 현재 제품 matcher를 전환하지 않는다.

## 제거 감사

제품 전환 완료는 제품 crate에서 `ContextRequirement`, lexical context registration, `EXACT_COMPONENT_MAX_COST_PENALTY`, `registered_lexical_context_prefix_len`과 비용 기반 `supports_component` 호출이 없어야 한다. 비용 lattice가 benchmark·진단 비교용으로 남는 경우 제품 matcher와 resource loading에서 도달할 수 없어야 한다.
