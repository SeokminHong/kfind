# 선택적 국소 형태 추론 계약

## 적용 범위

기본 검색은 query-side 형태 컴파일과 anchor 주변 verifier를 사용한다. corpus-side lattice는
추가 근거가 필요한 branch의 후보 위치에서만 실행한다.

| context requirement | 적용 대상 | 제품 결과 |
| --- | --- | --- |
| `None` | literal과 일반 형태 branch | 기존 verifier 결과 |
| `EojeolLattice` | 앞 host에 붙는 VCP 지정사 branch | shadow 계측만 수행 |
| `NominalComponent` | token 내부 명사 component 후보 | `accept`만 match로 복구 |

문자열 경계와 형태 분석 선택은 별도 축이다. 지정사는 homonym union을 유지하고, 명사
component만 검증된 corpus-side 근거로 `smart` 경계를 확장한다.

## 실행 계약

```text
byte scan
  -> anchor hit
  -> morphology/boundary verifier
  -> context requirement 확인
  -> bounded Unicode token 추출
  -> local lattice 비용 비교
  -> branch 정책 적용
```

- anchor가 없는 파일과 줄에서는 lattice를 실행하지 않는다.
- literal, `token`, `any`와 context가 필요 없는 branch는 resource를 읽지 않는다.
- 원문 범위는 256 bytes, NFC 문자열은 64 Unicode scalar, lattice는 4,096 node로 제한한다.
- NFC 안정 경계와 원문 byte offset을 양방향으로 보존한다.
- fixture 전용 비용, corpus 단어 denylist와 결과별 예외를 사용하지 않는다.
- resource 오류와 상한 초과를 조용히 다른 판정으로 바꾸지 않는다.

## resource 계약

query-side full POS와 corpus-side morphology resource는 같은 고정 source snapshot에서 생성하지만
별도 artifact다. full POS는 표제어·품사를, corpus-side resource는 원본 표면형의 모든 분석,
연결 ID, 단어 비용, matrix와 unknown 정의를 보존한다.

제품 `NominalComponent`는 compact schema 1을 사용한다. full morphology schema 3은 benchmark와
지정사 shadow 분석에 사용한다. compact/full 비교는 exact/common-prefix hit, scoring checksum,
candidate decision, 비용, node와 path provenance가 모두 일치해야 한다.

## benchmark 계약

- `EojeolLattice`는 raw anchor, verifier 통과, 대상 hit, 고유 어절과 N-best 경로를 성능 측정
  구간 밖에서 기록한다.
- `NominalComponent`는 기존 경계 reject, resource lookup, accept/reject와 경로 provenance를
  지정사 지표와 분리한다.
- component candidate가 있는데 resource가 없거나 검증에 실패하면 benchmark를 실패시킨다.
- 고정 test, dev, hard-negative와 PUD unseen fixture의 역할을 섞지 않는다.

## 후속 작업

지정사 결과 필터링은 제품 범위가 아니다. `copula-lattice` 후보는 UD Korean-PUD r2.18의
밀봉된 921-case fixture에서 recall 65.37%로 제품 gate를 통과하지 못했다. 지정사 검색은
homonym union을 유지하고 lattice 판정은 benchmark shadow 계측에만 남긴다. 다음 작업은
[형태소 검색 개선 핸드오프](morphology-handoff.md)를 따른다.
