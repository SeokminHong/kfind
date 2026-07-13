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

## 제품 후보 계약

- 정책 이름은 `copula-lattice`, 기본값은 `union`이다.
- `copula-lattice`는 `smart` VCP contextual origin에만 적용하고 기존 compact resource를
  사용한다. resource가 필요한 plan의 초기화 오류는 fallback하지 않는다.
- lattice `reject`만 contextual origin에서 제거한다. `accept`, `ambiguous`, 상한·평가 오류인
  `unresolved`는 유지하며 추가 비용 threshold는 없다.
- JSON과 `--explain-match`는 반환된 contextual origin의 outcome·비용·margin과
  `unresolved` reason을 보존한다. 거부 경로는 benchmark 보고서에 남긴다.
- UD Korean-PUD unseen gate를 통과하기 전에는 공개 제품 옵션으로 노출하지 않는다.

## 검증 범위

- Korean-Kaist·KSL dev의 VCP/VCN gold occurrence와 어휘 내부 음성을 전수 fixture로 사용한다.
- `학생일`, `책일`은 형태 조합 회귀 fixture로 유지한다.
- `smart`, `token`, `any`와 NFC/NFD에서 정상 부착형과 음성을 검증한다.
- Korean-GSD fixture는 regression baseline으로만 사용한다.
- UD Korean-PUD r2.18 test는 밀봉된 제품 판정용 unseen fixture로 사용한다.

[지정사 lattice 독립 평가](2026-07-13-copula-blind-evaluation.md)에는 source 정렬 불일치 2건을
제외하고 정상 VCP gold reject 13개가 남아 있다.

[지정사 lattice dev gold 진단](2026-07-13-copula-dev-diagnosis.md)의 gold-aligned candidate
1,007건 중 reject는 50건이다. segmented nominal competitor 33건, whole-window competitor
13건, segmented other 3건, segmented predicate 1건이며 두 lexicon profile의 결과는 같다.

## 후속 작업

1. PUD source adapter와 fixture 생성을 구현해 스펙의 expected digest를 검증한다.
2. `copula-lattice` 투영을 benchmark에 연결하고 밀봉된 fixture를 한 번 평가한다.
3. 모든 gate를 통과하면 opt-in 제품 정책을 구현하고, 실패하면 union을 유지한다.

Kaist·KSL test, Korean-GSD 및 PUD 결과에 맞춰 비용·threshold·fixture 선택을 변경하지 않는다.
