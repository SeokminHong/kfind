# copula smart-boundary 계약

## 제품 동작

`이다/아니다` 계열의 `smart` branch는 앞 체언에 붙는 활용을 검색한다. `학생일 가능성이 있다`와
`책일 가능성이 있다`의 `학생일`, `책일`은 사전 표제어가 아니라 체언 host와 `이다`의 관형형
표면 `일`이 결합한 어절이다.

검색 결과는 생성 가능한 분석의 homonym union이다. `매일`의 어휘 내부 `일`과 copula 후보를
query anchor와 인접 문자만으로 구분하지 않으며 corpus-side lattice 비용으로 필터링하지 않는다.
corpus 단어 denylist, fixture 전용 branch와 공개 disambiguation 옵션을 두지 않는다.

## 검증 범위

- `학생일`, `책일`은 형태 조합 회귀 fixture로 유지한다.
- `smart`, `token`, `any`와 NFC/NFD에서 정상 부착형과 음성을 검증한다.
- 기존 VCP/VCN positive와 `--boundary any`의 substring 계약을 유지한다.

copula 전용 lattice 후보는 밀봉 평가에서 false positive 35개와 함께 true positive 32개를
제거해 recall gate를 통과하지 못했다. 실행 코드와 fixture는 유지하지 않으며 판정 근거만
[2026-07-13 copula lattice 제품 판정](2026-07-13-copula-unseen-evaluation.md)에 보존한다.
