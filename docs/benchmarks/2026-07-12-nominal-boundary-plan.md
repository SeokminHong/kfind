# 명사 smart-boundary 계획

2026-07-13 결정: 검증된 형태 분석의 완전한 component span은 `smart` 검색 결과로 인정한다.
query가 component의 prefix·suffix·내부 어디에 있든 양쪽 경계를 형태 분석으로 증명한다.
`사용자권한 → 권한`도 positive다. 현재 기본 결과는 shadow 검증 전까지 유지한다.

## 근거

dev에서 embedded 명사 FN 70개 중 64개가 `boundary-rejected`다. full-POS profile 전체
FN에서도 boundary 거부가 97개로 가장 크다. 표제어 추가로 해결할 수 있는 문제가 아니다.

주요 형태는 다음과 같다.

- 합성어 구성 성분: `문학작품`의 `문학`, `중국요리`의 `요리`
- 붙여 쓴 단위: `2014년`, `요코씨`, `반친구`
- 생산 접미 결합: `고집스럽다`의 `고집`
- 문장 오류 또는 gold span 불일치

## 제품 선택지

선택: 외부 사전과 productive lattice가 증명한 완전한 구성 성분을 기본 `smart`에서 허용한다.
단순 substring과 component 경계를 가로지르는 span은 허용하지 않는다.

기존 `compound-substring` 중 `사용자권한 → 권한`은 positive로 전환한다. `대학교 → 학교`처럼
source component 근거가 없는 case와 component 경계를 가로지르는 substring을 precision
negative로 사용한다.

## 다음 구현 순서

1. dev boundary FN을 붙여쓰기, 복합어, 파생어, gold 오류로 분리한다.
2. 각 slice에 독립 positive/negative fixture를 만든다.
3. component-aware `smart`의 resource 비용을 측정한다.
4. 제품 결과를 바꾸기 전에 스펙과 CLI·library resource 계약을 먼저 변경한다.

## 완료 게이트

- component-aware `smart`와 `any`의 차이가 스펙에 명시된다.
- 합성어 component positive와 경계-crossing negative를 같은 fixture에서 평가한다.
- 특정 corpus 단어 denylist를 사용하지 않는다.
