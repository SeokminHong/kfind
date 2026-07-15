# 형태 분석 그래프 전환 계획

## 문제

현재 `smart` 판정은 compiler가 부여한 `ContextRequirement`에 따라 matcher가 경계, compact component lattice, bounded lexical context와 비용 마진을 직접 조합한다. 이 구조에는 다음 한계가 있다.

- `ContextRequirement`가 필요한 corpus resource, 판정 규칙, fallback과 우선순위를 함께 나타낸다.
- lexical context registry가 구조 규칙의 적용 대상을 query surface 목록으로 관리한다.
- compact component resource가 source의 `analysis_type`, `start_pos`, `end_pos`, `expression`을 버리므로 명시된 분해와 런타임에서 우연히 조합된 경로를 구분할 수 없다.
- include/exclude 경로의 비용 차이가 근거의 종류를 대신한다. 전역 마진을 조절해도 동률, 가까운 경계와 같은 surface의 positive/negative 충돌을 설명할 수 없다.

추가 surface 예외나 비용 임계값 조정은 이 문제를 닫는 방법으로 사용하지 않는다.

## 목표 구조

```text
QueryMorphPattern + BoundedTokenGraph
                 -> ConstraintResolver
                 -> SupportedAnalysisSet + Proof
                 -> ProductPolicy
```

- `QueryMorphPattern`은 query가 요구하는 fine POS, span 관계, candidate span 포함 관계, continuation DFA, 인접 token 제약과 component capability를 선언한다. corpus surface 목록을 포함하지 않는다.
- `BoundedTokenGraph`는 이전·현재·다음 token에서 source가 명시한 whole-token 분석과 component 분해, hard morphotactic edge로 연결한 런타임 조합 경로와 unknown 경로를 구분해 보존한다.
- `ConstraintResolver`는 pattern을 만족하는 모든 분석을 `SupportedAnalysisSet`과 proof로 반환한다. 같은 evidence mask라도 source identity, continuation이나 context proof가 다르면 경로를 버리지 않는다.
- `ProductPolicy`는 `whole`, `explicit-component`, `possible-analysis` 중 하나로 지지 분석 집합을 검색 결과에 투영한다. resolver core는 최종 수용 여부를 결정하지 않는다.
- 형태 분석 비용은 동등한 근거 안의 경로 순서와 진단에만 사용한다. dense matrix의 셀 존재 여부나 비용 차이로 hard edge, proof 종류 또는 제품 수용 여부를 결정하지 않는다.

`Supported`는 query lexical identity, fine POS, span 관계, continuation과 context를 만족하는 완전한 known 분석이 하나 이상 있는 경우다. 다른 분석 경로의 존재만으로 지지 분석을 부정하지 않는다. `Contradicted`는 완전한 분석은 있지만 query pattern을 만족하는 분석이 없을 때다. strict component 노출이나 span을 결정할 수 없으면 `Ambiguous`, resource가 없거나 손상·상한 초과이면 `Unavailable`이다. 의도한 의미를 형태 구조만으로 하나로 정할 수 없으면 가능한 분석을 보존하고 caller 정책 또는 별도 disambiguator로 넘긴다.

## 전환 단계

### 1. Source provenance shadow 감사

제품 결과를 바꾸기 전에 full morphology resource의 source metadata를 기존 local lattice 경로에 연결한다. 각 경로 node는 다음 중 하나로 분류한다.

| 분류 | 의미 |
| --- | --- |
| `source-atomic` | 같은 surface와 품사의 source 분석이 원자 분석으로 존재 |
| `source-decomposition` | source `expression`이 component 분해를 명시 |
| `runtime-composed` | 개별 source node를 런타임 연결했지만 whole-token 분해 근거는 없음 |
| `unknown` | unknown model이 만든 node |
| `unresolved` | source row와 node를 유일하게 대응하지 못함 |

감사는 development, 고정 test, Human, Agent와 hard-negative의 기존 component candidate를 모두 기록한다. 규칙 선택에는 development와 hard-negative만 사용하고 고정 test는 구조를 확정한 뒤 회귀 판정에만 사용한다. 보고서는 positive와 negative가 source decomposition, runtime composition, whole-token 분석 중 무엇으로 구분되는지 보여야 한다.

다음 조건을 모두 만족해야 제품 resource 변경으로 진행한다.

1. known node의 source provenance를 surface, POS, context ID와 비용으로 유일하게 복구한다.
2. 현재 비용 마진이 복구한 development positive와 hard-negative 충돌을 구조 분류로 설명한다.
3. 같은 구조 근거가 positive와 negative에 함께 나타나면 이를 `Ambiguous` 계약 대상으로 기록하고 surface registry나 새 비용 임계값을 만들지 않는다.

### 2. Expression component 관계 shadow

node의 source 분류만으로 positive와 negative가 분리되지 않으면 `expression`의 component를 source resource 계층에서 파싱한다. component 표면형의 canonical decomposition을 합쳐 node surface와 비교하고 다음 관계를 보존한다.

| 관계 | 의미 |
| --- | --- |
| `span-aligned` | component 경계가 NFC node의 안정된 byte span과 일치 |
| `fused` | 전체 표면은 canonical composition과 같지만 component 경계가 한 scalar 안에서 융합 |
| `unaligned` | 축약·교체 등으로 component 표면을 이어도 node surface와 같지 않음 |
| `absent` | source row가 expression을 제공하지 않음 |
| `invalid` | expression 형식을 해석할 수 없음 |

`span-aligned` component의 span과 POS가 query pattern과 같으면 기존 exact node가 아니어도 source가 명시한 component 관계로 기록한다. `fused`와 `unaligned`는 임의 byte span으로 투영하지 않는다. 같은 scoring node에 여러 source row가 대응하면 하나를 고르지 않고 모든 분석과 관계를 보존한다.

이 2차 shadow에서도 positive와 hard-negative가 같은 관계를 가지면 구조만으로 수용 여부를 결정할 수 없다는 뜻이다. 이 경우 resolver shadow 전에 compound exposure, 동형 활용 합집합과 같은 profile ambiguity 정책을 별도 계약한다.

### 3. Graph resource

compact resource의 다음 schema는 source 분석 종류와 정규화된 분해 component를 보존한다. raw `expression` 문자열을 제품 판정 때마다 파싱하지 않고 build 단계에서 span, POS와 관계 edge로 검증·압축한다. 기존 schema 1 loader는 호환 경로로 유지하되 graph resolver에는 사용할 수 없다.

full resource와 graph resource는 exact/common-prefix hit, source node, 연결 비용과 source provenance projection이 같아야 한다. schema, source SHA-256, section digest, span, context ID와 관계 edge를 내용을 노출하기 전에 검증한다.

schema 2는 schema 1과 같은 container magic을 사용하되 별도 `morphology-component-graph.kfc` 실험 artifact와 loader로 격리한다. index, graph payload, string table, connection matrix, `char.def`, `unk.def`의 여섯 section을 보존한다. graph payload는 surface별 source analysis, `analysis_type`, `start_pos`, `end_pos`, expression 관계와 정규화된 component를 저장하며 raw `expression`은 저장하지 않는다. `span-aligned` component는 검증된 NFC byte span을 저장하고 `fused`·`unaligned` component는 span을 갖지 않는다. expression이 없는 source row는 `absent`, 비어 있지 않지만 해석할 수 없는 row는 `invalid`로 구분한다.

graph resource는 `CompoundExposure` 선택을 저장하지 않는 policy-neutral 근거 계층이다. 따라서 schema 2 구현과 full-resource projection 검증은 profile 결정보다 먼저 진행할 수 있지만 resolver verdict와 제품 전환은 profile 계약을 정하기 전까지 진행하지 않는다.

schema 2 구현과 full-resource projection 결과는 [형태 분석 그래프 schema 2 projection과 비용](2026-07-15-morphology-analysis-graph-resource.md)에 기록한다.

### 4. Resolver shadow

기존 matcher 결과와 동시에 graph resolver의 verdict와 proof를 기록한다. 이 단계에서는 `ContextRequirement`, lexical context registry와 1,500 비용 마진을 제품 경로에 유지한다. resolver는 현재 true positive, hard-negative와 known ambiguity를 설명해야 하며 세부 입력·판정·profile 계약은 [형태 구조 제약 resolver 계약](morphology-constraint-resolver-contract.md)을 따른다.

비용을 읽지 않고 source whole, source component, runtime composition, opaque expression과 unknown을 구분하는 resolver core를 구현했다. query compiler는 `smart`의 지원 품사 분석을 surface registry와 무관한 pattern 집합으로 만들며 `token`, `any`, literal과 direct particle에는 pattern을 만들지 않는다. 제품 matcher와 같은 candidate에서 `opaque`, `transparent`, `explicit` verdict를 병렬 계측했으며 결과는 [형태 구조 제약 resolver shadow 결과](2026-07-15-morphology-constraint-resolver.md)에 기록했다.

### 5. 전체 제약 모델과 독립 평가

축소 resolver의 profile 평가를 전체 계약 평가로 교체한다. query compiler가 span 관계, continuation DFA, 인접 token 제약과 component capability를 만들고 graph resource가 source에서 파생한 hard morphotactic edge를 보존하게 한다. resolver는 최종 verdict 하나가 아니라 `SupportedAnalysisSet`을 반환한다.

reference candidate enumerator는 branch anchor만 공유하고 기존 verifier, boundary 판정, lexical context registry와 비용 lattice를 호출하지 않는다. candidate coverage, resolver conditional quality, ambiguity·unavailable, 세 제품 정책의 품질과 기존 제품 disagreement를 별도 지표로 기록한다.

schema 3은 schema 2 payload projection에 source expression과 multi-POS row에서 파생한 categorical transition table을 추가한다. 전체 resolver는 schema 3만 허용하며 schema 2 resource를 dense connection matrix 기반 경로로 fallback하지 않는다.

### 6. 제품 전환

graph resolver가 채택 조건을 통과하면 matcher는 `SupportedAnalysisSet`과 선택된 `ProductPolicy`만 소비한다. query compiler의 manual surface registry와 matcher의 비용 마진·requirement별 예외 분기를 제거하고 resource 필요 여부는 `QueryMorphPattern`의 구조 capability에서 계산한다. `token`, `any`, literal과 component가 필요 없는 `smart` branch는 graph resource를 읽지 않는다.

제품 전환 완료 시 `ContextRequirement`, lexical context registration, `EXACT_COMPONENT_MAX_COST_PENALTY`, registered-prefix raw fallback, predicate exact-token 예외와 비용 기반 `supports_component` 호출은 제품 경로에 남지 않는다. bounded context는 경쟁 분석을 삭제하는 우선순위가 아니라 token graph 제약으로만 표현한다.

축소 resolver shadow에서 세 profile 모두 채택 조건을 통과하지 못했다. 이 결과는 전체 제약 모델의 채택 여부를 증명하지 않으므로 독립 평가가 끝날 때까지 제품 판정, lexical context registry와 1,500 비용 마진을 유지한다.

## 채택 조건

- development와 고정 test의 기존 true positive를 보존하고 새 false positive를 만들지 않는다.
- hard-negative의 기존 결과보다 악화하지 않는다.
- `Supported`, `Contradicted`, `Ambiguous`, `Unavailable`, continuation DFA, adjacent token constraint와 hard-edge 거부가 모두 fixture로 검증된다.
- full resource와 compact graph resource의 verdict와 proof projection이 일치한다.
- morphology benchmark의 초기화, candidate enumeration, resolver, policy 적용, cases/s, p95와 RSS를 동일 revision·입력에서 비교한다.
- 공개 CLI와 stable Rust facade의 resource 오류·fail-fast 계약을 유지한다.

채택 조건을 통과하지 못하면 현재 제품 동작을 유지한다. source metadata로 구분할 수 없는 충돌은 실패가 아니라 명시적 ambiguity이며 제품 정책이나 별도 disambiguator를 정하기 전에는 자동으로 수용하지 않는다.
