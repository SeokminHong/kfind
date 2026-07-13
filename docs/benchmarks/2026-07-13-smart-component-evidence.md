# smart component 검색 근거

측정일: 2026-07-13

기준 보고서: `target/morph-benchmark-after/report.json`

## 결정

`smart`는 query가 문자열 token의 바깥 경계에 있을 때뿐 아니라, 검증된 형태 분석의 완전한
component span과 일치할 때도 검색 결과로 인정한다. 이 증명은 query component의 왼쪽과
오른쪽 내부 경계를 모두 다룬다.

따라서 다음은 positive다.

- `중국요리`의 `요리`
- `문학작품`의 `문학`
- `사용자권한`과 `권한관리`의 `권한`
- `산길`의 `길`

`대학교`의 `학교`처럼 source 분석의 component를 가로지르거나, `매일`의 `일`처럼 query
품사와 일치하는 component 근거가 없는 substring은 계속 거부한다.

## dev evidence

현재 dev의 명사 FN 70개 중 64개가 `boundary-rejected`다. gold 어절 안 query 위치는 다음과
같다.

| 위치 | case |
| --- | ---: |
| gold 어절 prefix | 49 |
| gold 어절 내부 | 13 |
| gold 어절 suffix | 2 |
| 합계 | 64 |

prefix에는 `문학작품`, `고집스러운`, `환경보호`처럼 오른쪽 component 경계가 필요한 case가
포함된다. 내부에는 `기록도구의`, `요코씨는`, `금속활자는`처럼 양쪽 component 경계와 뒤의
조사까지 함께 검증해야 하는 case가 있다. suffix에는 `2014년`, `중국요리`가 있다.

같은 lemma/POS와 gold span 기준으로 외부 분석기 결과를 교차 확인했다.

| Kiwi | Lindera | case |
| --- | --- | ---: |
| match | match | 57 |
| match | no-match | 4 |
| no-match | match | 3 |
| no-match | no-match | 0 |

64개 모두 적어도 한 분석기가 찾았고 57개는 두 분석기가 함께 찾았다. 이는 단순 표제어
추가보다 corpus-side component 분석이 recall 상한을 높일 가능성이 크다는 근거다. 제품
판정은 외부 분석기 출력이 아니라 고정 morphology resource의 source 분석으로 다시 측정한다.

## fixture 변경 방향

현재 `사용자권한 → 권한`을 no-match로 둔 fixture와 hard-negative는 승인된 계약과 충돌한다.
제품 결과를 바꾸는 구현 단위에서 positive로 전환한다. `대학교 → 학교`는 source component
근거가 없을 때 거부하는 경계-crossing negative로 유지하고, `역사과목 → 사과` 같은
component 경계 교차 case를 추가한다.

## 다음 계측

1. smart boundary에서 거부된 명사 anchor와 전체 Unicode token window를 수집한다.
2. morphology resource로 complete path를 구성한다.
3. query lemma/POS와 정확히 같은 node span을 포함하는 path와 제외하는 path를 함께 기록한다.
4. component evidence가 있는 dev FN 수와 revised hard-negative 오수용 수를 report에 추가한다.
5. 검색 결과는 shadow gate를 통과하기 전까지 변경하지 않는다.
