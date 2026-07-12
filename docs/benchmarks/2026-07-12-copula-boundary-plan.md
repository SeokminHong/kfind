# Copula smart-boundary 계획

## 목표

`smart` 경계에서 `매일`의 `일`을 `이다`의 관형형으로 인정하는 case를 다룬다.
`학생일`, `책일`과 같은 정상적인 부착형과 `--boundary any`의 substring 계약은
유지한다.

## 현재 제약

`매일`의 끝 `일`과 `학생일`의 끝 `일`은 현재 query-side matcher가 갖는 anchor,
왼쪽 token 문자, 오른쪽 경계만으로는 구분할 수 없다. 현재 스펙은 문맥상 다른
표제어라도 생성 가능한 표면형이면 검색 결과에 포함하는 homonym union을
정의한다.

## 선행 결정

다음 중 하나를 제품 정책으로 확정하기 전에는 matcher를 변경하지 않는다.

1. homonym union을 유지하고 해당 case를 `gold-or-adapter`로 분류한다.
2. 지정사 부착 host의 어휘 분석을 새 제품 범위로 정의한다.
3. 문맥 품사 분석을 런타임 검증에 추가하는 별도 설계를 진행한다.

특정 어휘만 막는 denylist나 현재 corpus 문장에 맞춘 예외 branch는 선택지에
포함하지 않는다.

## 실행 단계

1. `이다`의 생성 branch별 anchor·core mapping·continuation·경계 증거를 fixture로 고정한다.
2. 정상 부착형, 어휘 내부 substring, 한 음절 왼쪽·오른쪽 경계를 별도 slice로 만든다.
3. 선행 결정에 따라 스펙을 먼저 변경한 뒤 query plan과 matcher 경계를 수정한다.
4. `smart`, `token`, `any`와 NFC/NFD에서 정상 부착형·음성 case를 모두 검증한다.

## 완료 게이트

- 표면형 모호성을 어떤 제품 정책으로 풀지 스펙에 명시되어 있다.
- corpus 고유 예외 없이 독립적인 hard-negative slice로 정밀도를 보고한다.
- 기존 copula positive와 `--boundary any` 계약을 유지한다.
