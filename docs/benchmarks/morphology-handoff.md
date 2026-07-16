# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 계약과 바로 이어갈 작업만 유지한다. 완료한 실험과 측정
과정은 날짜별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [한글 수사 연쇄 recall](2026-07-17-hangul-numeral-recall.md)
- [같은 문장의 누적 검색 누락 검증](2026-07-17-query-matrix.md)
- [숫자 뒤 단위 구조 recall](2026-07-17-numeric-unit-recall.md)
- [질의 컴파일 병목 제거](2026-07-17-compile-hotpath-performance.md)
- [구조 증거로 줄인 검색 누락](2026-07-17-structural-recall.md)
- [계약 보정 지표와 구조 판정 품질](2026-07-16-contract-adjusted-structural-quality.md)
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
- strict TP·FP·TN·FN은 항상 보존한다. 제품 실행 전에 검토한 `same-pos-homograph`와
  `aligned-source-component`만 TPᶜ·FPᶜ·TNᶜ·FNᶜ에 병렬로 반영한다.
- canonical·hard-negative recall 개선은 contract-positive 분모 `PNᶜ = TPᶜ + FNᶜ`,
  `FNᶜ`와 `recallᶜ = TPᶜ / PNᶜ`를 함께 기록한다.

## 현재 경계

- corpus 전체를 형태 분석하지 않는다. anchor hit를 포함한 원문 256 bytes, NFC 64 Unicode
  scalar와 중복 제거 후 4,096 node 안에서만 구조를 판정한다.
- 비표준 활용, 오타와 불안정한 띄어쓰기는 canonical 형태 규칙에 합치지 않는다. 별도 opt-in
  robustness 축에서 자연 원문 fixture를 먼저 검증한다.
- full-POS가 없는 embedded workflow는 core lexicon만 사용한다. 구조 resource가 필요한
  `smart` query는 resource 없는 library/WASM engine에서 compile 오류다.

## 다음 작업

[구조 증거로 줄인 검색 누락](2026-07-17-structural-recall.md)에서 조사 없는 exact 체언 token과
축약 `-아/어` 뒤의 compact `VX+어미` 연쇄를 열었다. development full-POS smart는
TP 452→456, FN 48→44이고 test는 TP 466→470, FN 34→30이다. 두 fixture의 FP와 revised
hard-negative는 변하지 않았다. morphology microbenchmark와 100 MiB CLI의 모든 변화는
10% 경고선 안이다.

성능 profile에서는 full-POS 평가 CPU의 79.8%가 query compile에 있었고, 대부분이 branch key의
공유 rule vocabulary를 반복 hash하는 비용이었다. anchor별 bucket으로 중복 제거를 바꾼 뒤
full-POS `smart`는 5,389.7에서 12,364.5 cases/s, Agent workflow는 10,129.4에서 15,972.6
cases/s가 됐다. 품질과 전체 test span은 그대로다. 이후 query matrix 도입과 외부 snapshot
refresh로 현재 Agent matrix는 15,957.8 cases/s, Lindera는 19,829.6 cases/s다.

[숫자 뒤 단위 구조 recall](2026-07-17-numeric-unit-recall.md)에서 선행 ASCII 숫자와 source
`NNB/NNBC/NR` 단위, 선택적 조사 연쇄를 함께 요구하는 typed path를 열었다. development
full-POS smart는 TP 456→459, FN 44→41이고 test는 TP 470→472, FN 30→28이다. 두 fixture의
FP와 기존 hard-negative는 변하지 않았고 신규 대조군 4건도 모두 거부했다. 일반 token은
const-specialized fast path로 분리했으며 fresh 기준선 대비 full-POS와 Human 처리량 변화는
각각 -0.54%, -0.21%다.

[한글 수사 연쇄 recall](2026-07-17-hangul-numeral-recall.md)에서 token 왼쪽 경계부터 완성된
`NR` 연쇄와 선택적 `NNB/NNBC`, 조사 연쇄를 함께 요구하는 typed path를 열었다. development
full-POS smart는 TPᶜ 459→461, `PNᶜ=500`, FNᶜ 41→39이고 test는 TPᶜ 472→473,
FNᶜ 28→27이다. Human은 FNᶜ 31→30이며 strict FP와 FPᶜ는 변하지 않았다. 신규 대조군
`백명사전`, `일월산맥길`도 모두 거부했다. Matrix contract는 건드리지 않고 strict 지표를
paired 비교했으며 full-POS FN은 102→100, 완전 회수 문장은 373→375, Human FN은 97→96,
완전 회수 문장은 376→377이다. Matrix FP와 Agent 품질은 변하지 않았다. 최신 main 기준
matrix Agent와 Human 처리량 변화는 각각 -2.04%, +0.81%다.

다음 recall 작업은 남은 수사 FN `백명`과 `5천톤`, `6백미터`처럼 ASCII 숫자와 한글 수사 뒤
일반 단위명사가 이어지는 구조를 별도 typed path로 증명하는 것이다. `서사극이라`,
`인쇄업자가`처럼 구조적으로 증명하지 못한 조사 host와 의미상 동음이의어는 열지 않는다.
