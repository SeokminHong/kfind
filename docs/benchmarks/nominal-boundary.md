# 구조 기반 smart-boundary 계약

## 제품 동작

`smart`는 문자열 token 경계와 완전한 형태 경로의 whole/component span을 검색 근거로
사용한다. query의 세부 품사와 같은 node가 query core와 정확히 일치해야 하며, component
경계를 가로지르거나 더 큰 node 내부에만 존재하는 substring은 근거가 아니다.

의미에 따른 동음이의어 해소는 목표가 아니다. 구조와 품사가 같은 후보는 같은
`StructuralSignature`로 취급한다. 따라서 `걷다`와 `걸다` query는 모두 `걸었고`에 매칭될 수
있다. 구조 근거가 여러 갈래로 남으면 recall을 우선해 지원 가능한 후보를 유지한다.

문장 성분 배치로 구조를 증명할 수 있을 때는 후보를 선택한다.

- `매일 보고 싶어`에서는 `매일/MAG`를 선택하고 `매/NNG`를 거부한다.
- `독수리가 아니라 매일 수도 있어`에서는 `매/NNG + 이/VCP + ㄹ/ETM` 구조를 선택하고
  `매일/MAG`를 거부한다.
- `매일 매일 보고 싶어`에서는 반복 token 근거로 `매일/MAG`를 선택한다.
- `매일을`에서는 가장 긴 체언 host와 조사 연쇄를 선택하고 host 내부 substring을 거부한다.

특정 corpus 표면형 registry, denylist나 query 어휘 의미는 구조 선택에 사용하지 않는다.
단순 substring 검색은 `--boundary any`의 범위다.

## query와 matcher 계약

- query compiler는 `SurfaceBranch` 대신 `CandidateProgram`을 실행 IR로 만든다.
- program은 anchor, core 투영, 후보 범위, 조사·어미 소비 상태, provenance와 boundary 또는
  `StructuralConstraint`를 직접 소유한다.
- matcher는 program을 직접 실행한 뒤 동일한 `ConstraintResolver`로 구조를 판정한다.
  별도 `BranchVerifier`, `ContextRequirement`와 비용 기반 fallback은 없다.
- literal, `token`, `any` 및 구조 제약이 없는 `smart` program은 component resource를 읽지
  않는다.
- 구조 resource가 필요한 plan에서 누락·손상·schema·source mismatch는 초기화 오류다. 기존
  문자열 경계 판정으로 fallback하지 않는다.

## resource 계약

제품용 compact component resource는 schema 4다. 다음 정보만 보존한다.

- NFC surface double-array index
- source 분석의 POS
- 빌드 시 NFC 안정 경계에 정렬한 component POS와 byte span
- source SHA-256과 section digest

left/right context ID, word cost, 연결 행렬, unknown model과 원본 expression 문자열은 싣지
않는다. loader는 header, section digest, UTF-8, group·analysis·component offset과 span 범위를
검증한 뒤 resource를 노출한다. 비용 경로가 필요한 연구·진단은 별도 full morphology artifact를
사용하며 제품 판정을 바꾸지 않는다.

전체 `mecab-ko-dic-2.1.1-20180720` 입력의 schema 4 artifact는 773,105 surface, 806,568
structural analysis와 570,984 aligned component를 보존한다. 크기는 37,103,781 bytes이며 source
cost artifact 72,164,646 bytes의 51.42%다.

## 검증 계약

- 구조적 positive와 component-crossing·잘못된 조사 host negative를 같은 fixture에서 평가한다.
- 위의 `매일` 두 문장을 exact regression case로 유지한다.
- 걷/걸다처럼 구조가 같은 의미 후보의 합집합을 보존한다.
- compact structural projection과 full source artifact의 exact/common-prefix POS·component span이
  일치해야 한다. cost path의 결론 일치는 요구하지 않는다.
- 기존 조사 이형태와 predicate continuation 검증을 우회하지 않는다.
- 제품 matcher와 benchmark evaluator의 candidate coverage는 100%여야 한다.
- [비표준·오타·띄어쓰기 입력 robustness 후속 설계](noisy-text-robustness-plan.md)
- [비표준·오타·띄어쓰기 입력 평가 계약](noisy-text-robustness-evaluation.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)
