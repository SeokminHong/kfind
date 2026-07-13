# 형태소 검색 품질 검증 계약

## 품질 게이트

형태 품질 변경은 dev recall 80.00% 이상, precision 99.00% 이상과 revised hard-negative 신규
FP 0을 동시에 유지해야 한다.

현재 제품 기준선:

| fixture/profile | TP / FP / FN | precision | recall | F1 |
| --- | ---: | ---: | ---: | ---: |
| dev embedded smart | 432 / 2 / 68 | 99.54% | 86.4% | 92.51% |
| dev full-POS smart | 436 / 2 / 64 | 99.54% | 87.2% | 92.96% |
| test embedded smart | 408 / 1 / 92 | 99.76% | 81.6% | 89.77% |
| test full-POS smart | 413 / 1 / 87 | 99.76% | 82.6% | 90.37% |

세부 품사와 성능 결과는 [smart component 검색 근거](2026-07-13-smart-component-evidence.md)에
둔다.

## 데이터 역할

- Korean-Kaist·KSL dev: 규칙, 비용 정책과 threshold 개발
- Korean-Kaist·KSL test: 고정 회귀 확인
- revised hard-negative: 경계 정밀도와 신규 FP 확인
- Korean-GSD 지정사 fixture: 고정 regression baseline
- UD Korean-PUD r2.18 test: 지정사 필터링 제품 판정용 unseen 검증

Kaist·KSL test, Korean-GSD와 PUD 결과에 맞춰 규칙, 비용, threshold와 fixture 선택을 변경하지
않는다.
PUD fixture는 양성 436개, 음성 485개, excluded source copula 22개와 expected digest
`d02cd5e78ebc4d02d626ead6206b3ed1dddc6d4c71d7a19543981699e45ebebd`를 사용한다.
Kaist·KSL dev/test와 GSD test의 NFC 문장 hash 중복은 0건이다.

## 실패 분류

false negative는 다음 하나의 primary cause를 갖는다.

| 분류 | 판정 기준 |
| --- | --- |
| `lexicon-missing` | 기대 품사 분석이 query plan에 없음 |
| `surface-missing` | 분석은 있으나 gold 활용형 anchor가 없음 |
| `continuation-rejected` | core anchor는 있으나 ending continuation이 거부됨 |
| `boundary-rejected` | 형태는 있으나 `smart` 경계가 거부함 |
| `span-mismatch` | 같은 lemma/POS 결과가 gold 어절과 겹치지 않음 |
| `gold-or-adapter` | 외부 도구도 놓치거나 source 정렬이 의심됨 |

embedded와 full-POS 원인을 분리하고, 분류용 추가 compile·검색은 성능 측정 구간에서 제외한다.

## 남은 검증

Korean-Kaist·KSL dev의 지정사 gold-aligned lattice candidate는 1,007건이며 accept 957건,
reject 50건이다. 오거부 원인은
[지정사 lattice dev gold 진단](2026-07-13-copula-dev-diagnosis.md)을 기준으로 한다.

1. PUD source adapter로 밀봉된 fixture digest를 검증한 뒤 `copula-lattice`를 한 번 평가한다.
2. `-기` 명사형 뒤 조사 continuation은 독립 규칙과 hard-negative로 검증한다.
