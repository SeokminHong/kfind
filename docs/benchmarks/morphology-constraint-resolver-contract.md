# 형태 구조 제약 resolver 계약

## 목적

`smart` 검색의 corpus 판정을 surface registry, 비용 차이와 예외 우선순위에서 분리한다. query는 필요한 형태 관계만 선언하고 corpus 분석은 근거 종류를 보존하며 resolver는 충돌을 숨기지 않는다.

## 입력 모델

`QueryMorphPattern`은 query fine POS, candidate core span, whole-token 또는 source-component 관계, verifier가 소비한 continuation, 인접 token 제약과 명시적 component 노출 capability를 선언한다. query surface 목록, 비용 임계값, resource fallback 순서와 특정 corpus token은 포함하지 않는다.

`TokenAnalysisGraph`는 schema 2 source analysis에서 다음 근거를 구분한다.

| 근거 | 의미 |
| --- | --- |
| `source-whole` | source row가 token 전체 분석을 명시 |
| `source-component` | `span-aligned` expression이 candidate span과 POS를 명시 |
| `runtime-composed` | source node를 이어 완전한 token 경로를 구성했지만 whole-token expression 근거는 없음 |
| `unknown` | known complete path가 없을 때 unknown model로 구성한 경로 |

연결 비용과 word cost는 같은 근거 종류 안의 진단 순서에만 사용할 수 있으며 verdict를 바꾸지 않는다. known complete path가 있으면 unknown path는 모순 근거로 사용하지 않는다.

## 판정

`ConstraintResolver`는 `Proven`, `Contradicted`, `Ambiguous`, `Unavailable`과 proof를 반환한다.

- `Proven`: 완전한 분석들이 query pattern을 지지하고 같은 근거 등급에 양립 가능한 반대 분석이 없다.
- `Contradicted`: 완전한 분석들이 존재하지만 모두 query pattern과 모순된다.
- `Ambiguous`: query를 지지하는 분석과 반대 분석이 함께 존재하거나 span을 안정적으로 투영할 수 없는 `fused`·`unaligned` 관계가 남는다.
- `Unavailable`: graph resource가 없거나 손상됐거나 source가 다르거나 token graph 상한을 넘는다.

bounded context는 별도 예외 분기가 아니라 현재 token과 인접 token graph에 추가하는 구조 제약이다. 반복 token, `VCN+EC` 앞 문맥과 `NNB/NNBC` 뒤 문맥은 후보 분석을 줄일 수 있지만 경쟁하는 whole-token 분석을 삭제하지 않는다. 따라서 `내일`의 whole noun과 `내+이+ㄹ`, `매일`의 noun/adverb와 `매+이+ㄹ`처럼 양립 가능한 분석은 surface에 관계없이 `Ambiguous`다.

## Compound exposure

strict subspan의 `source-component` 근거와 enclosing whole-token 분석이 함께 있으면 resolver는 `CompoundExposure` ambiguity를 반환한다. profile은 이 ambiguity만 다음처럼 해석하며 다른 ambiguity에는 적용하지 않는다.

| profile | 해석 |
| --- | --- |
| `opaque` | component를 노출하지 않음 |
| `transparent` | 모든 `source-component`를 노출 |
| `explicit` | query가 별도 component 노출 capability를 선언한 경우만 노출 |

전역 `opaque`는 `속 -> 산속`, `기업 -> 기업주` positive를 잃고 전역 `transparent`는 `학교 -> 대학교` hard-negative를 허용한다. `explicit`은 이 충돌을 caller 선택으로 이동하지만 capability가 없는 현재 제품 결과를 자동 보존하지 않는다. 세 profile을 같은 development와 hard-negative에서 shadow 평가하고 품질 채택 조건을 통과한 profile이 없으면 제품 전환은 실패로 기록한다.

## 제품 전환 조건

제품 전환 뒤 query compiler는 `ContextRequirement` 대신 `QueryMorphPattern`을 만들고 matcher는 resolver verdict와 선택된 profile만 소비한다. lexical surface registry, 1,500 비용 마진, registered-prefix raw fallback, predicate exact-token 예외와 bounded context의 강제 분석 선택을 제품 경로에서 제거한다.

`token`, `any`, literal과 형태 구조가 필요 없는 branch는 graph resource를 읽지 않는다. graph가 필요한 `smart` plan은 schema 2 resource의 누락·손상·source mismatch와 상한 초과를 fallback하지 않고 관측 가능한 오류로 반환한다.

전환 후보는 development와 고정 test의 기존 true positive를 보존하고 새 false positive를 만들지 않으며 hard-negative를 악화하지 않아야 한다. `Proven`, `Contradicted`, `CompoundExposure`를 포함한 `Ambiguous`, 모든 `Unavailable` 원인은 fixture로 검증한다. 채택 조건을 통과하지 못하면 no-heuristic resolver 구현과 실패 근거는 유지하되 현재 제품 matcher를 전환하지 않는다.

## 제거 감사

제품 전환 완료는 제품 crate에서 `ContextRequirement`, lexical context registration, `EXACT_COMPONENT_MAX_COST_PENALTY`, `registered_lexical_context_prefix_len`과 비용 기반 `supports_component` 호출이 없어야 한다. 비용 lattice가 benchmark·진단 비교용으로 남는 경우 제품 matcher와 resource loading에서 도달할 수 없어야 한다.
