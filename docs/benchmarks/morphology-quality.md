# 형태소 검색 품질 검증 계약

## 품질 게이트

명시적 품사 `smart` 품질 변경은 dev strict precision 99.00% 이상과 revised hard-negative
신규 contract FP 0을 유지하면서 표준 띄어쓰기 FN을 늘리지 않아야 한다. 필요한 공백이 빠진
`nonstandard-spacing` case는 strict FP/FN과 row-level delta에 그대로 남기되 이 gate에서 제외하고
별도 robust 지원에서 해소한다. FN이 줄어든 후보를 우선하고, FN이 같을 때만 FP가 줄어든
후보를 선택한다. 무품사 결과는 제품 한계와 회귀를 그대로 관측하며 목표 수치에 맞춰 fixture,
gold 또는 negative 선택을 바꾸지 않는다.

품질 gold는 표제어에서 생성 가능한 형태·표면 match를 평가하며 문맥 의미를 평가하지
않는다. 같은 structural signature를 가진 동음이의어·동형이의어 match는 의미가
다르다는 이유로 false positive로 분류하지 않는다. whole/component 분해, 품사 또는
인접 성분 배치로 구분 가능한 경우에는 선택되지 않은 구조의 match를 오답으로 계산한다.

보고서의 기본 TP·FP·TN·FN은 corpus gold를 그대로 적용한 strict 지표다. 별도의
`contract_adjusted` 지표는 버전 관리 fixture에 제품 실행 전에 선언한 `contract_expected`만
적용한다. 같은 품사의 동형 활용은 `same-pos-homograph`, source에 정렬된 내부 성분 검색은
`aligned-source-component`로 근거를 제한한다. 결과 표의 TPᶜ·FPᶜ·TNᶜ·FNᶜ는 각각
`contract_tp`·`contract_fp`·`contract_tn`·`contract_fn`이며 strict 지표를 대체하지 않는다.

현재 제품 기준선:

| fixture/profile | TP / FP / FN | precision | recall | F1 |
| --- | ---: | ---: | ---: | ---: |
| dev embedded smart | 447 / 4 / 53 | 99.11% | 89.4% | 94.01% |
| dev full-POS smart | 456 / 4 / 44 | 99.13% | 91.2% | 95.00% |
| test embedded smart | 435 / 0 / 65 | 100.00% | 87.0% | 93.05% |
| test full-POS smart | 470 / 0 / 30 | 100.00% | 94.0% | 96.91% |

세부 품사, strict/contract-adjusted hard-negative와 성능 결과는
[구조 증거로 줄인 검색 누락](2026-07-17-structural-recall.md)에 둔다.

비표준 활용, 오타와 불안정한 띄어쓰기는 이 canonical 기준선에 합치지 않는다. 별도 fixture와
robust-only precision, over-acceptance, canonical retention, raw-span 지표는
[robustness 평가 계약](noisy-text-robustness-evaluation.md)을 따른다.

## 제품 workflow 판정

- 에이전트 CLI는 `embedded + any + 명시적 품사`의 recall과 처리량을 주 지표로 삼는다.
  false positive는 실패 gate가 아니라 문맥 확인이 필요한 후보 수로 보고한다.
- `smart + 명시적 품사`는 FN을 1차, FP를 2차 최적화 대상으로 삼는다. precision 99.00% 하한과
  hard-negative 보호 안에서는 관련성이 낮은 후보 노출보다 검색 누락을 더 큰 회귀로 판정한다.
- 사람 CLI는 별도 무품사 fixture의 `full-POS + smart` precision·recall, 기대 품사 plan 포함률과
  초기화 비용을 함께 본다. 품사 모호성으로 생긴 오차를 숨기지 않고 명시적 품사 결과와 분리해
  보고한다.
- 라이브러리는 resource 없는 embedded engine을 기본으로 측정하고 full-POS lexicon과 component
  resource 초기화를 선택 비용으로 분리한다.
- 전체 lexicon·boundary 행렬은 원인 분석에 사용하며 workflow들을 하나의 점수로 합치지 않는다.
- 실제 CLI 비용은 고정 100 MiB source corpus에서 에이전트 JSON 경로와 사람 기본 출력 경로를
  fresh process로 측정한다. fixture runner의 query별 compile·match 처리량과 CLI wall time·corpus
  처리량을 같은 지표로 해석하지 않는다.
- 제품 profile 차트는 각 workflow의 precision·recall·F1·false-positive 후보 수와 실제 CLI
  wall time·corpus 처리량·peak RSS를 함께 보여 준다. 품질 fixture와 CLI corpus는 측정 단위가
  다르며 하나의 종합 점수로 합치지 않는다.
- 제품 persona 비교는 같은 explicit-POS fixture와 gold에서 Agent, User, Kiwi, Lindera,
  MeCab-ko, KOMORAN의 precision·recall·F1과 fixture 처리 성능을 함께 보여 준다. Agent와 외부
  분석기는 품사를 명시하고, User는 같은 query의 품사를 제거해 `full-POS + smart`로 실행한다.
  차트 label에는 persona와 backend명만 표시하고 품사 입력 조건은 본문에서 설명한다.
- 이 결과는 동일 입력의 backend 순위가 아니라 실제 persona 입력을 반영한 제품 비교다. User는
  자동 품사 계획과 모호성 비용을 포함하고, 다른 품사의 lemma match도 explicit-POS gold에서
  오답으로 계산된다. 별도 사람용 무품사 fixture는 production-like negative를 검증하며 이 비교에
  섞지 않는다.
- 외부 성능은 snapshot 갱신 시 fresh process 1회 warm-up 뒤 5회 측정하며 기본 benchmark에서는
  다시 실행하지 않는다.

## 데이터 역할

- Korean-Kaist·KSL dev: 규칙, 비용 정책과 threshold 개발
- Korean-Kaist·KSL test: 고정 회귀 확인
- revised hard-negative: 경계 정밀도와 신규 FP 확인
- 외부 분석기 스냅샷: 같은 test fixture에서 Kiwi·Lindera·MeCab-ko·KOMORAN 품질·성능 비교

Kaist·KSL test와 무품사 결과에 맞춰 규칙, 비용, threshold, fixture, gold 또는 negative 선택을
변경하지 않는다. 규칙 선택은 고정 dev와 독립 hard-negative만 사용한다.
외부 분석기 스냅샷은 test fixture SHA-256과 어댑터 schema에 묶는다. 기본 벤치마크는
외부 분석기를 실행하지 않으며 fixture나 고정한 도구·어댑터 설정이 바뀔 때만 명시적으로 갱신한다.

## 실패 분류

false negative는 다음 하나의 primary cause를 갖는다.

| 분류 | 판정 기준 |
| --- | --- |
| `lexicon-missing` | 기대 품사 분석이 query plan에 없음 |
| `surface-missing` | 분석은 있으나 gold 활용형 anchor가 없음 |
| `continuation-rejected` | core anchor는 있으나 ending continuation이 거부됨 |
| `boundary-rejected` | 형태는 있으나 `smart` 경계가 거부함 |
| `span-mismatch` | 같은 lemma/POS 결과가 gold 어절과 겹치지 않음 |
| `gold-or-adapter` | 스냅샷 외부 도구 둘 이상이 모두 놓치거나 source 정렬이 의심됨 |

embedded와 full-POS 원인을 분리하고, 분류용 추가 compile·검색은 성능 측정 구간에서 제외한다.

## 남은 검증

1. 명시적 품사 full-POS `smart`의 development FN 44건을 기준으로 다음 후보를 조사한다.
2. 남은 `boundary-rejected` 32건은 품사·token 위치별 같은 표면형 대조군과 component 경로를 먼저
   고정한다. `서사극이라`와 `인쇄업자가`처럼 구분되지 않는 유형은 제품에 열지 않는다.
3. dev FN 감소, precision 99.00% 이상과 hard-negative 신규 FP 0을 확인한 뒤 고정 test와 무품사
   결과를 한 번만 회귀 측정한다.
