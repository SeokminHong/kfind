# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 계약과 바로 이어갈 작업만 유지한다. 완료한 실험과 측정
과정은 날짜별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [component SHA-256 hardware backend](2026-07-17-component-hardware-digest-startup.md)
- [full POS validation prefix 재검사 제거](2026-07-17-full-pos-validation-startup.md)
- [full POS packed lookup index](2026-07-17-full-pos-packed-startup.md)
- [component section digest 병렬 검증](2026-07-17-component-digest-startup.md)
- [full POS decoder 중복 소유 제거](2026-07-17-full-pos-decoder-startup.md)
- [관형사 뒤 명사 우선 경로 recall](2026-07-17-modifier-noun-preferred-path-recall.md)
- [숫자 단위 뒤 의존명사 tail recall](2026-07-17-numeric-unit-dependent-tail-recall.md)
- [관형사 뒤 명사 component recall](2026-07-17-modifier-noun-component-recall.md)
- [whole 체언 내부 source component recall](2026-07-17-whole-nominal-component-recall.md)
- [관형형 의문 종결 recall](2026-07-17-adnominal-interrogative-recall.md)
- [연결 어미 뒤 topic recall](2026-07-17-connective-topic-recall.md)
- [`-다는` source 어미 recall](2026-07-17-declarative-adnominal-recall.md)
- [분해된 체언 host recall](2026-07-17-graph-nominal-host-recall.md)
- [지정사 앞 체언 recall](2026-07-17-nominal-copula-host-recall.md)
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

[지정사 앞 체언 recall](2026-07-17-nominal-copula-host-recall.md)에서 명사 후보 뒤 suffix가
`이` 또는 `입`으로 시작하고 source `VCP + E+ + J*`가 token 끝까지 이어질 때 조사 allomorph
선거부를 건너뛰었다. Canonical embedded·full-POS는 `것이었다`의 `것`을 회수해
`PNᶜ=500`에서 FNᶜ가 각각 59→58, 19→18이다. Matrix embedded·full-POS는
`결과이다`·`고체이긴`·`왕친입니다`·`것이었다` 4건을 회수해 `PNᶜ=1,401`에서 FNᶜ가
153→149, 76→72이고 완전 회수 문장은 각각 4개 늘었다. Human은 3건을 회수해 FNᶜ가
74→71이다. FP·FPᶜ와 hard-negative 결과는 변하지 않았다. 후보 Agent는 25,573.5
cases/s로 Lindera snapshot보다 22.80% 빠르며, 모든 성능 변화는 10% 경고선 안이다. Matrix
contract 정의, annotation과 gate는 변경하지 않았다.

남은 3건 동률 묶음은 부사 `안`, 동사 `오다`, 형용사 `이다`다. `안`은 모두 비표준
붙여쓰기이고 `이다`는 무표면 축약 2건과 비표준 표기 1건이며, `오다`는 원인이 서로 다르다.
다음 제품 recall 작업은 남은 standard-form FN을 cause별로 다시 묶어 공통 구조가 가장 큰
slice를 고른다. 비표준 입력은 canonical 규칙에 합치지 않는다.

[분해된 체언 host recall](2026-07-17-graph-nominal-host-recall.md)에서 token 왼쪽 경계부터
여러 명사 edge로 조합한 체언 host 전체와 query core가 정확히 같고 조사 연쇄를 token 끝까지
소비한 경우를 runtime support로 승격했다. Canonical embedded·full-POS·Human은
`포터소만과`·`캔맥주와`·`대영제국의` 3건을 회수해 `PNᶜ=500`에서 FNᶜ가 각각
58→55, 18→15, 22→19다. Matrix embedded·full-POS·Human은 6건을 회수해
`PNᶜ=1,401`에서 FNᶜ가 각각 149→143, 72→66, 71→65다. FP·FPᶜ와 hard-negative 결과는
변하지 않았다. 후보 Agent는 26,868.0 cases/s로 Lindera snapshot보다 29.01% 빠르며 모든
성능 변화는 10% 경고선 안이다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

남은 가장 큰 동일 질의 묶음은 각 3건인 부사 `안`, 동사 `오다`, 형용사 `이다`다. `안`과
`이다`의 6건은 비표준 붙여쓰기·축약·표기다. 다음 제품 recall 작업은 `누구→누가`,
`위하다→위해서는`, `오다→온지를`, `어떻다→어떤가`를 matrix의 동일 원인 case와 묶어
가장 큰 typed standard-form 구조부터 연다.

[`-다는` source 어미 recall](2026-07-17-declarative-adnominal-recall.md)에서 candidate가
`다`까지 소비하고 정확히 `는`만 남기며 source graph가 `용언 + E*` 완성 경로를 증명할 때
조사형 선거부를 건너뛰었다. Test matrix full-POS는 `있다는` 2건, `왔다는`·`않다는` 각
1건을 회수해 `PNᶜ=1,401`에서 FNᶜ가 66→62, recallᶜ가 95.29%→95.57%가 됐다. 완전 회수
문장은 406→409/468이다. Human은 `왔다는` 1건을 회수해 FNᶜ가 65→64가 됐다. FP와 FPᶜ,
canonical 및 Agent 품질은 변하지 않았다. 후보 Agent는 26,808.2 cases/s로 Lindera
snapshot보다 28.73% 빠르다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

남은 가장 큰 동일 질의 묶음인 부사 `안` 3건과 형용사 `이다` 3건은 비표준 입력이다. 다음
제품 recall 작업은 standard-form인 `위해서는`·`대해서는`과 development의 `없지는`을
connective 뒤 topic particle이라는 공통 source path로 먼저 검증한다. `온지를`의
nominalizer-particle 구조는 별도 slice로 둔다.

[연결 어미 뒤 topic recall](2026-07-17-connective-topic-recall.md)에서
`ending.aoeo-seo`·`ending.connective-ji` candidate가 소비한 경계까지 source
`predicate + E+`, 그 뒤 token 끝까지 `J+`인 경우에만 topic `는`을 열었다. Canonical
full-POS·Human은 `PNᶜ=500`에서 FNᶜ가 각각 15→14, 19→18이다. Test matrix full-POS·Human은
`대해서는`·`위해서는` 2건을 회수해 `PNᶜ=1,401`에서 FNᶜ가 각각 62→60, 64→62가 됐다.
Development full-POS는 `하지는`·`해서는`·`없지는` 3건을 회수해 FNᶜ가 107→104가 됐다.
FP와 FPᶜ, Agent 품질은 변하지 않았다. 후보 Agent는 26,826.3 cases/s로 Lindera
snapshot보다 28.81% 빠르다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

남은 가장 큰 동일 질의 묶음인 부사 `안` 3건과 형용사 `이다` 3건은 비표준 입력이다. 다음
제품 recall 작업은 기존 대명사 override 계약과 같은 `저→제` 2건, `누구→누가` 1건을
검증한다. `오다→온지를`의 nominalizer-particle 구조는 그다음 별도 slice로 둔다.

[대명사 축약 표면 recall](2026-07-17-pronoun-surface-recall.md)에서 주격 replacement
`누구+가→누가`와 기본 `저의`를 보존하는 속격 alias `저+의→제`를 분리했다. Canonical
embedded·full-POS·Human은 `누가 당선`을 회수해 `PNᶜ=500`에서 FNᶜ가 각각 55→54,
14→13, 18→17이다. Matrix embedded·full-POS·Human은 `제 생각`, `제 전공`, `누가 당선`
3건을 회수해 `PNᶜ=1,401`에서 FNᶜ가 각각 143→140, 60→57, 62→59다. Smart FP·FPᶜ와
hard-negative는 변하지 않았다. Agent `any`는 FNᶜ를 같은 수만큼 줄였지만 paired negative의
`체제`·`경제` 속 `제` 때문에 FPᶜ가 1건 늘었다. Matrix Agent F1은 97.88%→97.96%다.
후보 Agent는 25,943.6 cases/s로 Lindera snapshot보다 24.58% 빠르다. Matrix contract 정의,
annotation과 gate는 변경하지 않았다.

남은 표준형 `오다→온지를`는 adnominal 뒤 nominalizer와 조사 경로다. 다음 제품 recall 작업은
hard-negative `nominalizer-particle` slice와 함께 source `용언 + ETM + NNB/NNG + J*`의
정확한 경계를 검증한다.

[관형형 뒤 의존명사·조사 recall](2026-07-17-adnominal-dependent-noun-recall.md)에서 관형형
candidate가 소비한 경계까지 마지막 어미가 `ETM`인 `predicate + E* + ETM`, 그 뒤 token
끝까지 `NNB/NNBC + J+`인 source path를 열었다. Canonical full-POS·Human은 `온지를`의
`오다`를 회수해 `PNᶜ=500`에서 FNᶜ가 각각 13→12, 17→16이다. Matrix full-POS·Human은
`온지를`의 `오다`, `좋아할지도`의 `좋아하다`를 회수해 `PNᶜ=1,401`에서 FNᶜ가 각각
57→55, 59→57이고 완전 회수 문장은 각각 414→416, 411→413이다. FP·FPᶜ와 hard-negative는
변하지 않았다. 후보 Agent는 26,661.7 cases/s로 Lindera snapshot보다 28.02% 빠르다. Matrix
contract 정의, annotation과 gate는 변경하지 않았다.

남은 큰 동일 질의 묶음인 부사 `안` 3건과 형용사 `이다` 3건은 비표준 입력이다. 다음 제품
recall 작업은 표준형 `어떻다→어떤가`의 source `VA + EC`와 경쟁 `MM + EC` 경로를 대조해
용언 완성 경로만 안전하게 회수할 수 있는지 검증한다.

[관형형 의문 종결 recall](2026-07-17-adnominal-interrogative-recall.md)에서 관형형 candidate가
정확히 `가`만 남기고 source graph가 같은 품사의 `predicate + E+` 완성 경로를 증명하면
경쟁 whole modifier가 있어도 용언 후보를 유지했다. 실제 고정 resource는 `어떤/VA + 가/EC`와
`어떤가/MM+EC`를 함께 보존한다. Canonical embedded·full-POS·Human은 `어떻다→어떤가`
1건을 회수해 `PNᶜ=500`에서 FNᶜ가 각각 54→53, 12→11, 16→15다. Matrix의 세 smart
profile도 같은 1건을 회수해 embedded·full-POS·Human FNᶜ가 각각 140→139, 55→54,
57→56이고 완전 회수 문장은 각각 1개 늘었다. FP·FPᶜ와 hard-negative는 변하지 않았다.
후보 Agent는 25,096.2 cases/s로 Lindera snapshot보다 20.51% 빠르다. Matrix contract 정의,
annotation과 gate는 변경하지 않았다.

남은 가장 큰 동일 질의 묶음은 비표준 입력이 섞인 부사 `안`, 형용사 `이다`, `되다` 각
3건이다. 다음 제품 recall 작업은 표준형 내부 체언 성분인 `1년간→간`, `어느날→날`,
`첫번째로→번째`, `자본주의→주의`를 `compound-substring` hard-negative와 함께 분류해
공통 typed 구조로 안전하게 회수할 수 있는지 검증한다.

[whole 체언 내부 source component recall](2026-07-17-whole-nominal-component-recall.md)에서
whole `자본주의/NNG`가 선언한 `주의/NNG` component를 더 짧은 `자본주 + 의/JKG` 조사
host가 가리지 않도록 했다. Test matrix embedded·full-POS·Human은 이 1건을 회수해
`PNᶜ=1,401`에서 FNᶜ가 각각 139→138, 54→53, 56→55가 됐고 완전 회수 문장도 각각
1개 늘었다. FP·FPᶜ와 hard-negative는 변하지 않았다. Component provenance는 기존
`Unit`에 합쳐 추가 collection 할당을 만들지 않는다. 후보 Agent는 25,459.7 cases/s로
Lindera snapshot보다 22.25% 빠르며 모든 성능 변화는 10% 경고선 안이다. Matrix contract
정의, annotation과 gate는 변경하지 않았다.

함께 분류한 `1년간→간`은 ASCII 숫자+단위 뒤 nominal tail, `어느날→날`은 `MM + NNG`
path라서 서로 다른 구조다. `첫번째로→번째`는 `NNBC + XSN` 경계를 가로질러 exact component
계약으로 열지 않는다. 다음 제품 recall 작업은 앞의 두 표준형을 hard-negative와 대조해 더
큰 typed 구조 하나를 고른다.

[관형사 뒤 명사 component recall](2026-07-17-modifier-noun-component-recall.md)에서 token
왼쪽 경계의 `MM` 뒤에 `NNG/NNP/NNB/NNBC`가 이어지는 완성 path를 열었다. 한 음절
`MM + 명사` 두 성분 구조는 같은 선행 span의 `NR`도 요구해 `칠/MM|NR + 월/NNBC`은
지원하고 `소/MM + 년/NNB`은 거부한다. Test matrix embedded·full-POS·Human은 `칠월에`의
`월` 1건을 회수해 `PNᶜ=1,401`에서 FNᶜ가 각각 138→137, 53→52, 55→54가 됐다.
FP·FPᶜ와 hard-negative는 변하지 않았다. 후보 Agent는 26,761.3 cases/s로 Lindera
snapshot보다 28.51% 빠르며 모든 성능 변화는 10% 경고선 안이다. Matrix contract 정의,
annotation과 gate는 변경하지 않았다.

현재 수사 경로는 `사십구억오천이백육십오만이천백팔십칠` 같은 긴 `NR` 연쇄의 component를
이미 지원하지만 단위 크기 순서와 숫자값은 검산하지 않는다. 다음 제품 recall 작업은
`1년간→간`의 ASCII 숫자+단위+nominal tail을 먼저 분리 검증한다. 숫자 단위 순서 검산은
precision 후보로 별도 측정하고 matrix contract는 변경하지 않는다.

[숫자 단위 뒤 의존명사 tail recall](2026-07-17-numeric-unit-dependent-tail-recall.md)에서
ASCII 숫자와 `NNB/NNBC/NR` 단위 뒤의 정확한 `NNB/NNBC` tail, 선택적 조사 연쇄를 하나의
완성 path로 유지했다. 같은 범위의 긴 단일 단위를 먼저 골라 `10시간`을 `10시+간`으로
분해하지 않는다. Test matrix embedded·full-POS·Human은 `1년간→간` 1건을 회수해
`PNᶜ=1,401`에서 FNᶜ가 각각 137→136, 52→51, 54→53이 됐다. Development matrix의
두 explicit-POS profile은 `8시간쯤→시간`, `15층이상→층`을 회수해 FNᶜ가 각각 2건 줄었다.
FP·FPᶜ, canonical과 hard-negative는 변하지 않았다. 후보 Agent는 26,797.8 cases/s로
Lindera snapshot보다 28.68% 빠르다. Matrix contract 정의, annotation과 gate는 변경하지
않았다.

남은 matrix full-POS FNᶜ는 51건이다. 반복 2건 이상인 묶음은 비표준·붙여쓰기 입력 또는
무표면 지정사라 canonical 규칙으로 넓히지 않는다. 다음 recall 진단은 제품 규칙이 이미
의도한 `어느날→날`이 고정 resource 단위 검증과 달리 matrix에서 남는 candidate span 경로를
먼저 재현한다. 한글 수사 단위 순서와 산술값 검산은 recall과 분리한 precision 후보로 둔다.

[관형사 뒤 명사 우선 경로 recall](2026-07-17-modifier-noun-preferred-path-recall.md)에서
`어느/MM + 날/NNG` 완성 경로가 더 짧은 `어/VV` prefix와 경쟁해도 최소 component 수의
`MM + 명사` 경로를 우선하도록 했다. Predicate-frame 충돌을 먼저 확인한 후보에서만 우선
경로를 검사한다. Test matrix embedded·full-POS·Human은 `어느날→날` 1건을 회수해
`PNᶜ=1,401`에서 FNᶜ가 각각 136→135, 51→50, 53→52가 됐고 완전 회수 문장도 각각
1개 늘었다. FP·FPᶜ, canonical, development와 hard-negative는 변하지 않았다. 후보 Agent는
26,712.9 cases/s로 Lindera snapshot보다 28.27% 빠르다. Matrix contract 정의, annotation과
gate는 변경하지 않았다.

남은 test matrix full-POS FNᶜ는 50건, Human FNᶜ는 52건이다. 반복 2건 이상인 묶음은
비표준·붙여쓰기 또는 무표면 지정사다. 다음 작업은 canonical 규칙을 무리하게 넓히지 않고
profile로 현재 scan/startup 병목을 다시 고른다. 한글 수사 단위 순서와 산술값 검산은 recall과
분리한 precision 후보로 둔다.

[full POS decoder 중복 소유 제거](2026-07-17-full-pos-decoder-startup.md)에서
front-compressed 632,667 entry를 복원할 때 결과와 별도로 직전 표제어를 clone하던 소유를
없앴다. Full POS 단독 base 초기화는 131.80ms에서 120.44ms로 8.62%, Human 전체 초기화는
4.60% 줄었다. 100MiB CLI Human 처리량은 340.70에서 358.57MiB/s로 5.24% 늘었다. 모든
canonical·matrix·hard-negative prediction과 FNᶜ/PNᶜ는 같다. Matrix contract 정의,
annotation과 gate는 변경하지 않았다.

Component resource decoder는 여전히 약 130ms로 가장 큰 다음 optional startup 병목이다.
다음 작업은 section digest, index decode와 component vector 검증 시간을 분리 측정한 뒤 가장
큰 구간만 최적화한다.

[component section digest 병렬 검증](2026-07-17-component-digest-startup.md)에서 Time
Profiler 기준 component 초기화의 약 106ms를 차지한 SHA-256 section 검증을 native의 큰
index와 payload 사이에서 병렬화했다. Component 구간은 129.20ms에서 91.07ms로 29.51%,
full-POS 조합의 전체 초기화는 15.61% 줄었다. 100MiB CLI Human 처리량은 359.76에서
420.23MiB/s로 16.81% 늘었다. 모든 canonical·matrix·Human·hard-negative prediction과
span, FNᶜ/PNᶜ는 같다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

이제 optional resource 조합에서는 full-POS base 초기화 약 119ms가 component 약 93ms보다
크다. 다음 성능 작업은 full-POS decode의 남은 allocation, index 구축과 검증을 다시
profile한다. Component payload 구조 검증 약 16ms는 그 뒤 후보로 둔다.

[full POS packed lookup index](2026-07-17-full-pos-packed-startup.md)에서 front-compressed
632,667개 entry를 entry별 소유 문자열로 전개하지 않고 하나의 lemma blob과 offset·품사
index로 보존했다. Full POS 단독 peak RSS는 47,208→21,500KiB로 54.46%, Human과 CLI의
peak RSS는 약 31.6% 줄었다. Full POS 단독 초기화는 15.40%, 100MiB CLI Human wall은
7.29% 줄고 처리량은 7.87% 늘었다. 검색 cases/s와 p95는 회귀하지 않았다. 모든
canonical·matrix·Human·hard-negative prediction과 span, FNᶜ/PNᶜ는 같다. Matrix contract
정의, annotation과 gate는 변경하지 않았다.

남은 full-POS base 110.71ms에는 file read, decoder, embedded lexicon·rule 생성과 enriched
predicate merge가 함께 들어 있다. 다음 성능 작업은 이 하위 구간을 따로 계측해 두 자릿수
millisecond 병목만 고른다. Component payload 구조 검증 약 16ms와 per-entry 미세 최적화는
우선하지 않는다.

[full POS validation prefix 재검사 제거](2026-07-17-full-pos-validation-startup.md)에서
profile의 49개 startup CPU sample 중 42개를 차지한 decoder 검증을 줄였다. 새 suffix만
UTF-8로 검사해 검증된 prefix에 붙이고, ASCII·완성형 한글 lemma는 구성상 NFC를 증명한다.
그 밖의 Unicode는 기존 일반 NFC 검사를 유지한다. Full POS 단독 초기화는
109.10→37.58ms로 65.55%, full-POS+component 전체는 34.33% 줄었다. 100MiB CLI Human
wall은 31.52% 줄고 처리량은 46.03% 늘었다. 모든 canonical·matrix·Human·hard-negative
prediction과 span, FNᶜ/PNᶜ는 같다. Matrix contract 정의, annotation과 gate는 변경하지
않았다.

이제 full-POS+component 전체 132.75ms 중 component가 94.46ms로 71%다. 다음 성능 작업은
component file read, 병렬 section digest와 payload 구조 검증을 최신 코드에서 다시 분리
측정해 가장 큰 구간만 줄인다. Full POS decoder의 남은 38.54ms는 우선하지 않는다.

[component SHA-256 hardware backend](2026-07-17-component-hardware-digest-startup.md)에서
최신 profile의 164개 CPU sample 중 135개를 차지한 portable SHA-256을 runtime-detected
hardware backend로 바꿨다. Embedded component load는 92.85→49.49ms로 46.70%, full-POS
조합의 component load는 94.38→47.28ms로 49.91% 줄었다. 100MiB CLI Human wall은 30.95%
줄고 처리량은 44.83% 늘었다. 모든 canonical·matrix·Human·hard-negative prediction과
span, FNᶜ/PNᶜ는 같다. Matrix contract 정의, annotation과 gate는 변경하지 않았다.

이제 full-POS+component 전체 86.57ms 중 component는 47.28ms, full-POS base는 39.20ms다.
Component의 다음 성능 후보는 payload record 검증과 file read지만, profile에서 payload parse는
21개 sample이다. 두 자릿수 millisecond 개선을 확인할 때만 진행하고, 다음 제품 품질 작업은
남은 test matrix full-POS FNᶜ 50건을 현재 contract 그대로 원인별 재분류한다.
