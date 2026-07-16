# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 계약과 바로 이어갈 작업만 유지한다. 완료한 실험과 측정
과정은 날짜별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [구조 기반 국소 형태 판정 계약](selective-morphology.md)
- [구조 기반 smart-boundary 계약](nominal-boundary.md)
- [비표준·오타·띄어쓰기 입력 robustness 후속 설계](noisy-text-robustness-plan.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)

## 제품 계약

- CLI, Rust library와 WASM binding은 같은 query compiler, `CandidateProgram`과 matcher를
  사용한다.
- query compiler는 anchor, core 투영, 후보 범위, 조사·어미 consumption, decision과 모든
  `Origin`을 program 하나에 보존한다. matcher와 benchmark evaluator는 동일한 후보 열거기를
  실행한다.
- literal, `token`, `any` 및 구조 판정이 필요 없는 `smart` program은 `Boundary` decision만
  사용한다. 구조 근거가 필요한 `smart` program은 `QueryMorphPattern`을 소유하고
  `ConstraintResolver`에서 `Structural` decision을 실행한다.
- 제품 경로에는 `SurfaceBranch`, `BranchVerifier`, `ContextRequirement`, 수동 lexical-context
  surface registry, 비용 마진과 예외 fallback이 없다.
- 필요한 compact resource의 누락·손상·schema·source mismatch는 초기화 오류다. 문자열 경계
  판정으로 fallback하지 않는다.

## 모호성 정책

의미상 동음이의어 해소는 제품 목표가 아니다. span topology, 품사, continuation과 인접 제약이
같으면 하나의 `StructuralSignature`로 합친다. `걷다`와 `걸다` query는 모두 `걸었고`에
매칭될 수 있다. 구조적으로도 결론을 내릴 수 없으면 재현율을 우선해 지원 가능한 후보를
유지한다.

문장 성분 배치로 구조를 증명할 수 있을 때는 해당 품사 구조를 선택한다.

- `매일 보고 싶어`: `매일/MAG`를 선택하고 `n:매`를 거부한다.
- `독수리가 아니라 매일 수도 있어`: `매/NNG + 이/VCP + ㄹ/ETM`을 선택하고
  `adv:매일`을 거부한다.
- `매일 매일 보고 싶어`: repeated-token 근거로 `매일/MAG`를 선택한다.
- `매일을`: 가장 긴 체언 host와 조사 연쇄를 선택하고 host 내부 substring을 거부한다.

## 구조 resource

제품 compact component resource는 schema 4다. 다음만 보존한다.

- NFC surface double-array index
- source 분석의 POS
- NFC 안정 경계에 정렬한 component POS와 byte span
- source SHA-256과 section digest

left/right context ID, word cost, 연결 행렬, unknown model과 원본 expression 문자열은 제품
artifact에 싣지 않는다. 전체 `mecab-ko-dic-2.1.1-20180720` 입력의 artifact는 773,105 surface,
806,568 structural analysis와 570,984 aligned component를 보존한다. 크기는 37,103,781
bytes다. full morphology lattice는 제품 판정을 바꾸지 않는 진단 도구로만 유지한다.

## 검증 게이트

- 고정 test의 TP를 줄이거나 FP를 늘리지 않는다.
- development precision 99.00% 이상, revised hard-negative 신규 FP 0과 FN 비증가를 유지한다.
- 제품 matcher와 benchmark evaluator의 candidate coverage는 100%여야 한다.
- compact structural projection과 full source artifact의 exact/common-prefix POS·component span이
  일치해야 한다. cost path 결론 일치는 제품 gate가 아니다.
- morphology benchmark는 fresh process에서 warm-up 1회 후 5회 측정한다. initialization,
  cases/s, p95 latency와 RSS의 median/min/max를 최신 `origin/main`과 같은 환경에서 비교한다.
- query compile은 Criterion `new/sample.json`의 sample별 `times[i] / iters[i]`를 nearest-rank
  p95로 비교한다.

## 현재 경계

- corpus 전체를 형태 분석하지 않는다. anchor hit를 포함한 원문 256 bytes, NFC 64 Unicode
  scalar와 중복 제거 후 4,096 node 안에서만 구조를 판정한다.
- 비표준 활용, 오타와 불안정한 띄어쓰기는 canonical 형태 규칙에 합치지 않는다. 별도 opt-in
  robustness 축에서 자연 원문 fixture를 먼저 검증한다.
- full-POS가 없는 embedded workflow는 core lexicon만 사용한다. 구조 resource가 필요한
  `smart` query는 resource 없는 library/WASM engine에서 compile 오류다.

## 다음 작업

최신 `origin/main`과 전환 결과의 고정 품질, morphology 성능, query compile과 structural
constraint microbenchmark를 같은 환경에서 다시 측정한다. 결과와 exact revision을 날짜별
보고서에 기록하고 이 문서의 게이트 상태를 갱신한다.
