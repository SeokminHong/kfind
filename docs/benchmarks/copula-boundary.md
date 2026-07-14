# copula smart-boundary 계약

## 제품 동작

`이다/아니다` 계열의 `smart` branch는 앞 체언에 붙는 활용을 검색한다. `학생일 가능성이 있다`와
`책일 가능성이 있다`의 `학생일`, `책일`은 사전 표제어가 아니라 체언 host와 `이다`의 관형형
표면 `일`이 결합한 어절이다.

검색 결과는 기본적으로 생성 가능한 분석의 homonym union이다. 다만 query와 독립적인 같은 줄의
bounded 구조가 하나의 분석만 증명하면 그 분석으로 좁힌다. 부정 지정사 연결형, 유일한
`체언 + VCP+ETM` 분해, 의존명사가 연속하는 구조는 현재 token의 체언과 지정사를 선택한다.
NFC가 같은 token의 인접 반복과 exact `MAG` 분석이 함께 있으면 whole-token 부사를 선택한다.

`독수리가 아니라 매일 것 같아`에서는 `매`와 `이다`가 각각 `매`, `일`을 찾고 whole-token
`매일` 명사·부사는 찾지 않는다. `매일 매일 보고 싶어`에서는 부사 `매일`만 두 번 찾는다.
`그는 집념으로 매일을 보내고 있었다.`에서는 `n:매일`이 조사까지 소비한 `매일을`을 찾는다.
구조가 모호하거나 범위·resource 검증에 실패하면 homonym union을 유지한다. corpus-side lattice
비용, 단어 denylist, fixture 전용 branch와 공개 disambiguation 옵션은 사용하지 않는다.

## 검증 범위

- `학생일`, `책일`은 형태 조합 회귀 fixture로 유지한다.
- 지정사 구조, 반복 부사와 조사 결합 명사를 같은 query 행렬로 검증한다.
- `smart`, `token`, `any`와 NFC/NFD에서 정상 부착형과 음성을 검증한다.
- 기존 VCP/VCN positive와 `--boundary any`의 substring 계약을 유지한다.

copula 전용 lattice 후보는 밀봉 평가에서 false positive 35개와 함께 true positive 32개를
제거해 recall gate를 통과하지 못했다. 실행 코드와 fixture는 유지하지 않으며 판정 근거만
[2026-07-13 copula lattice 제품 판정](2026-07-13-copula-unseen-evaluation.md)에 보존한다.
