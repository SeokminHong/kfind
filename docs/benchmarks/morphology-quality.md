# 형태소 검색 품질 검증 계약

## 품질 게이트

명시적 품사 `smart` 품질 변경은 dev precision 99.00% 이상과 revised hard-negative 신규 FP 0을
유지하면서 FN을 늘리지 않아야 한다. FN이 줄어든 후보를 우선하고, FN이 같을 때만 FP가 줄어든
후보를 선택한다. 무품사 결과는 제품 한계와 회귀를 그대로 관측하며 목표 수치에 맞춰 fixture,
gold 또는 negative 선택을 바꾸지 않는다.

현재 제품 기준선:

| fixture/profile | TP / FP / FN | precision | recall | F1 |
| --- | ---: | ---: | ---: | ---: |
| dev embedded smart | 433 / 2 / 67 | 99.54% | 86.6% | 92.62% |
| dev full-POS smart | 437 / 2 / 63 | 99.54% | 87.4% | 93.08% |
| test embedded smart | 409 / 0 / 91 | 100.00% | 81.8% | 89.99% |
| test full-POS smart | 414 / 0 / 86 | 100.00% | 82.8% | 90.59% |

세부 품사와 성능 결과는
[User smart precision 품질·성능](2026-07-14-user-smart-precision.md)과
[`-기` 명사형 조사 continuation 품질·성능](2026-07-14-gi-particle-continuation.md)에 둔다.
현재 development 개선과 비회귀 측정은
[명시적 품사 `-지` 오른쪽 끝 recall](2026-07-14-connective-ji-right-edge-recall.md)에 둔다.

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

1. 명시적 품사 full-POS `smart`의 development FN 63건 중 `어떻다 -> 어떤`, `이렇다 -> 이런`,
   `커다랗다 -> 커다란` ㅎ 불규칙 `surface-missing`을 core lexicon의 `DropH` 분석으로 복구한다.
2. 같은 generator가 규칙형 `어떻은`, `이렇은`, `커다랗은`을 만들지 않는지 회귀 fixture로
   고정한다.
3. dev FN 감소, precision 99.00% 이상과 hard-negative 신규 FP 0을 확인한 뒤 고정 test와 무품사
   결과를 한 번만 회귀 측정한다.
