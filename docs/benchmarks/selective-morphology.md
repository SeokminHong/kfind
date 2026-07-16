# 구조 기반 국소 형태 판정 계약

## 적용 범위

기본 검색은 query-side 형태 컴파일과 anchor 탐색을 사용한다. `smart`의 corpus-side
판정은 anchor가 발견된 bounded token window에서만 실행하며, token 내부 component와
바로 인접한 token 구조를 확인한다.

```text
QueryMorphPattern + CandidateProgram
  + BoundedTokenGraph
  -> ConstraintResolver
  -> StructuralSupport + Proof
  -> ProductPolicy
```

## query IR 계약

- `CandidateProgram`은 anchor, core 투영, 후보 범위, 판정 제약과 provenance를 소유한다.
- 후보 범위는 `Anchor`, `SurroundingToken`, `AnchorAndSurroundingToken` 중 하나다.
  matcher와 benchmark evaluator는 같은 열거기를 사용하고 후보 세트와 span이 일치해야 한다.
- 후보 span은 `core ⊆ anchor ⊆ consumed ⊆ token`을 만족한다. exact는
  `consumed == anchor`, 용언은 완성된 continuation을 포함한 token, 체언은 조사가
  없는 anchor 후보와 완성된 조사 연쇄 후보를 각각 만든다.
- `QueryMorphPattern`은 lexical identity, 세부 품사, continuation DFA, component capability,
  인접 token 제약을 선언한다. 등록된 전체 표면형 목록으로 문맥을 인코딩하지 않는다.
- 표면형 전개 규칙은 검색 anchor와 `Origin`만 만든다. 구조 수용 여부는
  `QueryMorphPattern`과 token graph만으로 결정한다.
- literal, `token`, `any` 및 구조 판정이 필요 없는 `smart` 경로는 boundary program을
  사용하며 component resource를 열지 않는다.

## resolver 계약

- bounded token graph는 source whole/component, runtime, unknown을 구분하고 입력 바이트 span을
  유지한다. NFC 변환을 거친 경우에도 원문 byte offset으로 역매핑해야 한다.
- resolver는 query와 독립적인 whole/component, 세부 품사, continuation과 인접 token
  근거로 corpus 구조를 먼저 선택한다.
- span topology, 품사, continuation과 문맥 제약이 같고 어휘 의미만 다른 후보는
  하나의 `StructuralSignature`로 합친다. 제품 matcher는 이 합집 안의 의미를 해소하지 않는다.
- 분해·품사·인접 성분 배치가 다른 경쟁 path는 하나의 구조가 선택되거나
  모호함이 확정될 때까지 평가한다. 결과는 `Supported`, `Contradicted`, `Unavailable`로 구분한다.
- 서로 다른 구조가 여전히 모호하면 `ProductPolicy`는 recall을 우선해 지원 가능한
  query 후보를 유지한다. `Ambiguous`와 경쟁 proof 전체는 진단 mode에서만 물질화한다.
- program이 보존한 query provenance는 모두 남기되 corpus 의미 분석을 추가하지 않는다.
- resource 누락·손상·schema mismatch·그래프 상한 초과는 `Unavailable`로 구분하고
  기존 boundary 판정이나 raw-cost 임계값으로 바꾸지 않는다.
- 제품 compact resource와 resolver는 source 분석 비용, 연결 행렬과 미등록어 모델을
  읽지 않는다. 별도 full morphology 진단 artifact만 과거 비용 판정 비교에 사용하며,
  `include <= exclude + 1,500` 같은 제품 판정 임계값을 두지 않는다.

## 실행·resource 계약

- anchor가 없는 파일과 줄에서는 token graph를 만들지 않는다.
- token window는 256 bytes, NFC 문자열은 64 Unicode scalar, local graph는 4,096 node로
  제한한다. 후보·그래프 cache는 파일 범위의 bounded cache로 유지한다.
- query plan은 각 program의 component capability를 합성해 resource 필요 여부를 결정한다.
- query-side full POS와 corpus-side morphology resource는 같은 고정 source snapshot에서 생성하지만
  별도 artifact다. compact와 full resource의 source identity, exact/common-prefix hit, graph
  edge와 span provenance가 일치해야 한다.
- resource 없는 Rust/WASM engine과 compact resource를 초기화한 engine의 startup·RSS를
  분리해 측정한다.

```console
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
```

## 전환 게이트

- 제품 matcher와 benchmark evaluator의 candidate coverage는 100%여야 한다.
- 고정 test의 TP를 줄이거나 FP를 늘리지 않는다. dev에서 precision 99.00% 이상과
  revised hard-negative 신규 FP 0을 유지하면서 FN을 늘리지 않는다.
- `query_compile`, morphology fixture 초기화·cases/s·p95·RSS와 실제 CLI workload를 같은
  revision·입력·build profile에서 `origin/main`과 비교한다.
- 전환이 완료되면 `SurfaceBranch`, `BranchVerifier`, `ContextRequirement`, 수동
  lexical-context surface registry, exact-component 1,500 비용 마진과 기존 verifier fallback을
  제품 코드에서 제거한다. 동일 후보를 두 IR에 동시 표현하지 않는다.
