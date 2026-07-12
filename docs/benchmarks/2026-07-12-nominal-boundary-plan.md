# 명사 smart-boundary 계획

## 근거

dev에서 embedded 명사 FN 70개 중 64개가 `boundary-rejected`다. full-POS profile 전체
FN에서도 boundary 거부가 97개로 가장 크다. 표제어 추가로 해결할 수 있는 문제가 아니다.

주요 형태는 다음과 같다.

- 합성어 구성 성분: `문학작품`의 `문학`, `중국요리`의 `요리`
- 붙여 쓴 단위: `2014년`, `요코씨`, `반친구`
- 생산 접미 결합: `고집스럽다`의 `고집`
- 문장 오류 또는 gold span 불일치

## 제품 선택지

1. 기본 `smart`가 합성어 내부를 거부하는 현재 계약을 유지한다.
2. 합성어 구성 성분 검색을 위한 별도 boundary mode를 추가한다.
3. 외부 사전의 복합어 구조를 resource에 넣고 검증된 구성 성분만 허용한다.

기본 `smart` 완화는 `compound-substring` hard-negative와 충돌하므로 적용하지 않는다.

## 다음 구현 순서

1. dev boundary FN을 붙여쓰기, 복합어, 파생어, gold 오류로 분리한다.
2. 각 slice에 독립 positive/negative fixture를 만든다.
3. 별도 mode의 사용자 가치와 resource 비용을 측정한다.
4. 제품 mode를 추가하기로 결정하면 스펙과 CLI를 먼저 변경한다.

## 완료 게이트

- 기본 `smart`와 `any` 계약이 변하지 않는다.
- 합성어 positive와 substring negative를 같은 fixture에서 평가한다.
- 특정 corpus 단어 denylist를 사용하지 않는다.
