# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 계약과 바로 이어갈 작업만 유지한다. 완료한 실험과 측정
과정은 날짜별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [anchor automaton 선구축 병목 제거](2026-07-17-lazy-anchor-automaton-performance.md)
- [결합형 보조용언 recall](2026-07-17-attached-auxiliary-recall.md)
- [source 어미 경로 recall](2026-07-17-predicate-ending-path-recall.md)
- [체언 뒤 지정사 구조 recall](2026-07-17-copula-nominal-host-recall.md)
- [지정사 완성형 recall](2026-07-17-copula-surface-recall.md)
- [혼합 수량 구조 recall](2026-07-17-mixed-numeral-unit-recall.md)
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

[지정사 완성형 recall](2026-07-17-copula-surface-recall.md)에서 source `VCP=이`와 완성 어미로
증명되는 `이라고`·`이라는`·`이지`·`이며`를 terminal branch로 열었다. Canonical test의
embedded·full-POS·Human·Agent는 각각 `PNᶜ=500`에서 FNᶜ가 1건 줄어 59·24·27·16이다.
Matrix contract는 건드리지 않았고 reclassified 0인 최신 fixture의 strict·contract-adjusted
지표를 함께 비교했다. Explicit-POS embedded·full-POS·Agent의 FNᶜ는 각각 5건 줄어
157·93·38이고 Human은 3건 줄어 91이다. FP와 FPᶜ는 변하지 않았다. 신규 고유명사 `이지`
대조군도 두 smart profile에서 거부했다.

Matrix full-POS의 남은 `이다` FN 7건은 nominal-host 배치가 거부된 표준형 4건, 무표면 축약
`겁니다` 2건과 비표준 `이예요` 1건이다. 다음 제품 recall 작업은 source component가 있는
`동안이었습니다`·`끝인가`·`곳인`·`공학입니다` 네 case의 공통 nominal-host 구조만 연다.
무표면 축약과 비표준 표기는 canonical 활용에 합치지 않는다.

[체언 뒤 지정사 구조 recall](2026-07-17-copula-nominal-host-recall.md)에서 token 왼쪽 경계부터
VCP core 직전까지 완성된 체언 host를 요구하고, 생성 branch가 suffix를 남길 때는 같은 source
VCP+어미 경로가 token 끝까지 이어지는지 검증했다. Test matrix의 embedded·full-POS·Human은
`동안이었습니다`·`끝인가`·`곳인`·`공학입니다` 4건을 복구해 `PNᶜ=1,401`에서 FNᶜ가
각각 153·89·87이 됐다. FP와 FPᶜ는 변하지 않았고, development embedded·full-POS도 5건씩
복구했다. `매일`과 `큰 일이` 대조군은 두 smart profile에서 거부했다.

[source 어미 경로 recall](2026-07-17-predicate-ending-path-recall.md)에서 일반 용언의
non-terminal generator branch가 token 내부에 멈추면 같은 품사의 source
`EP/EC/EF/ETM/ETN` path가 token 끝까지 이어지는지 검증했다. Core의 token 왼쪽 경계,
whole modifier 부재와 조사 allomorph 부재를 함께 요구한다. Canonical full-POS·Human은
`PNᶜ=500`에서 FNᶜ가 각각 24→22, 27→25다. Matrix full-POS·Human은 `PNᶜ=1,401`에서
7건을 복구해 FNᶜ가 89→82, 87→80이고 완전 회수 문장은 384→390이다. FP와 FPᶜ는
변하지 않았다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

Matrix full-POS의 다음 큰 동일 질의 FN 묶음은 동사 `지다` 4건이다. `오다`와 동사 `있다`는
각각 3건이다. 다음 작업은 `지다` 4건을 보조용언·피동/결과 구조와 독립 동사로 먼저 분류하고,
공통 구조가 확인된 경우에만 제품 규칙을 연다.

[결합형 보조용언 recall](2026-07-17-attached-auxiliary-recall.md)에서 full-POS VX 질의에만
token 왼쪽 경계의 일반 용언 `VV/VA + EC`, query core의 `VX`, token 끝까지의 `E*` 경로를
요구했다. Canonical full-POS·Human은 `PNᶜ=500`에서 3건을 회수해 FNᶜ가 각각 19·22가
됐다. Matrix full-POS·Human은 `PNᶜ=1,401`에서 `지다` 4건, `나다`·`있다` 각 1건을
회수해 FNᶜ가 76·74가 됐고 완전 회수 문장은 390→396이다. FP·FPᶜ는 변하지 않았고
`사진` 대조군도 거부했다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

[anchor automaton 선구축 병목 제거](2026-07-17-lazy-anchor-automaton-performance.md)에서
형태소 평가 CPU의 24.8%를 차지하던 다중 anchor automaton 생성을 짧은 일회성 검색에서
제거했다. 짧은 생성+검색 p95는 74.958µs에서 14.206µs로 81.05% 줄고, 승격 뒤 장문 scan은
유의한 차이가 없다. Canonical과 matrix의 strict·contract-adjusted 결과 및 FNᶜ/PNᶜ는
모두 같다. Agent는 16,911.2에서 26,612.4 cases/s로 57.37% 빨라져 같은 explicit-POS
fixture의 Lindera 20,825.6 cases/s를 27.79% 앞선다. Matrix contract 정의, annotation과
gate는 변경하지 않았다.

Matrix full-POS의 가장 큰 남은 동일 질의 묶음은 각 3건인 `것`, 부사 `안`, 동사 `오다`,
형용사 `이다`다. 다음 작업은 네 묶음을 case-level로 비교해 공통 구조로 가장 많은
contract-positive를 안전하게 회수하는 하나를 고른다.
