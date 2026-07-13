# VCP 지정사 smart-boundary 계약

## 제품 동작

`smart`의 VCP 지정사 branch는 체언 host에 붙는 활용을 검색한다. `학생일 가능성이 있다`와
`책일 가능성이 있다`의 `학생일`, `책일`은 사전 표제어가 아니라 체언 host와 VCP 관형형
표면 `일`의 결합이다.

지정사 검색은 생성 가능한 분석의 homonym union을 유지한다. `매일`의 어휘 내부 `일`과 VCP
관형형 후보는 query anchor와 인접 문자만으로 구분하지 않는다. VCP branch의
`EojeolLattice`는 benchmark shadow 계측에만 사용하며 제품 결과를 필터링하지 않는다.

corpus 단어 denylist와 fixture 전용 branch는 허용하지 않는다. `--boundary any`의 substring
계약과 기존 VCP/VCN positive는 유지한다.

## 검증 범위

- Korean-Kaist·KSL dev의 VCP/VCN gold occurrence와 어휘 내부 음성을 전수 fixture로 사용한다.
- `학생일`, `책일`은 형태 조합 회귀 fixture로 유지한다.
- `smart`, `token`, `any`와 NFC/NFD에서 정상 부착형과 음성을 검증한다.
- Korean-GSD fixture는 regression baseline으로만 사용한다.

[지정사 lattice 독립 평가](2026-07-13-copula-blind-evaluation.md)에는 source 정렬 불일치 2건을
제외하고 정상 VCP gold reject 13개가 남아 있다.

[지정사 lattice dev gold 진단](2026-07-13-copula-dev-diagnosis.md)의 gold-aligned candidate
1,007건 중 reject는 50건이다. segmented nominal competitor 33건, whole-window competitor
13건, segmented other 3건, segmented predicate 1건이며 두 lexicon profile의 결과는 같다.

## 후속 작업

1. 결과 필터링 정책과 resource 오류, 상한 초과, JSON/explain 계약을 스펙에 정의한다.
2. 별도 unseen source와 fixture를 결과 확인 전에 고정한다.
3. dev에서 정한 정책을 unseen source로 검증한 뒤 제품 적용 여부를 판단한다.

Korean-GSD 결과에 맞춰 비용, threshold와 fixture 가중치를 변경하지 않는다.
