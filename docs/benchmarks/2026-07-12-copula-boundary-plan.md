# VCP 지정사 smart-boundary 계획

## 목표

`smart` 경계에서 어휘 내부 표면형을 `이다` 계열 VCP 지정사 분석으로 인정하는 case를
다룬다. 정상 분석 근거는 Korean-Kaist·KSL dev split의 실제 VCP/VCN 어절을 사용하고,
`--boundary any`의 substring 계약은 유지한다.

`학생일 가능성이 있다`, `책일 가능성이 있다`의 `학생일`, `책일`은 사전 표제어로 가정하지
않는다. 체언 `학생`, `책`에 VCP 관형형 표면 `일`이 붙는 조합을 검증하는 어절 fixture다.

## 현재 제약

`매일` 같은 어휘 내부 `일`과 `학생일`, `책일`의 VCP 지정사 후보 `일`은 현재 query-side matcher가 갖는 anchor,
왼쪽 token 문자, 오른쪽 경계만으로는 구분할 수 없다. 현재 스펙은 문맥상 다른 표제어라도
생성 가능한 표면형이면 검색 결과에 포함하는 homonym union을 정의한다.

## 정책 결정

기존 homonym union 계약을 유지한다. 따라서 다음 선택지 중 1번을 적용하고 matcher는
변경하지 않는다.

1. homonym union을 유지하고 해당 case를 `gold-or-adapter`로 분류한다.
2. VCP/VCN host의 형태 분석을 새 제품 범위로 정의한다.
3. 문맥 품사 분석을 런타임 검증에 추가하는 별도 설계를 진행한다.

특정 어휘만 막는 denylist나 현재 corpus 문장에 맞춘 예외 branch는 선택지에
포함하지 않는다.

독립 benchmark의 어휘 내부 hit는 현재 제품 범위에서 문맥 분석 없이 제거할 수 없는 동형
표면형이다. 이 항목은 matcher 회귀가 아니라 알려진 정책 한계로 기록한다.

## 실행 단계

1. `이다`의 생성 branch별 anchor·core mapping·continuation·경계 증거를 fixture로 고정한다.
2. `학생일`, `책일`은 형태 조합 회귀 fixture로 유지하고, 품질 판정은 UD dev corpus의
   VCP/VCN 분석, 어휘 내부 substring, 한 음절 왼쪽·오른쪽 경계를 별도 slice로 측정한다.
3. 제품 정책을 변경할 때만 스펙을 먼저 변경한 뒤 query plan과 matcher 경계를 수정한다.
4. `smart`, `token`, `any`와 NFC/NFD에서 정상 부착형·음성 case를 모두 검증한다.

## 완료 게이트

- 표면형 모호성을 어떤 제품 정책으로 풀지 스펙에 명시되어 있다.
- corpus 고유 예외 없이 독립적인 hard-negative slice로 정밀도를 보고한다.
- 조합 fixture `학생일`, `책일`의 VCP 관형형 경로를 보존한다.
- 기존 VCP/VCN positive와 `--boundary any` 계약을 유지한다.
