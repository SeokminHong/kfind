# local lattice 비용 실패 분석

대상: 지정사 local-context 2,916건의 schema 4 shadow report

## 결론

현재 lattice가 모든 candidate를 `reject`한 원인은 threshold가 아니라 morphology resource의
분석 행 누락이다. query-side 품사 집합을 corpus-side resource 필터로 재사용하면서 어미,
접사, 숫자와 복합 `Inflect` 분석이 제거되었다. 연결 비용 행렬의 방향과 합산은 MeCab 계약과
일치한다.

P3 판정 적용은 보류한다. 다음 resource schema를 정할 때 query 품사와 corpus 분석 품사를
분리하고, 축약형의 복합 분석이 query를 포함하는 조건을 먼저 확정해야 한다.

## `NoCompletePath`

- 오류 candidate는 26개, 고유 분석 어절은 16개다.
- 모든 어절이 숫자를 포함한다. 예: `1주일`, `1박2일`, `68억이다`, `12명이고`.
- lattice는 HANGUL 미등록어 node만 만든다. source의 `NUMERIC` unknown class와 `SN` 분석은
  사용하지 않는다.
- 숫자 byte 구간을 덮는 node가 없으므로 BOS에서 EOS까지 완전 경로가 만들어지지 않는다.

## resource 손실

| 항목 | source 행 | 현재 보존 | 제외 |
| --- | ---: | ---: | ---: |
| 전체 | 816,283 | 758,360 | 57,923 |
| `Inflect` | 44,820 | 723 | 44,097 |

`EC` 2,547개, `EF` 1,821개, `EP` 51개, `ETM` 133개, `ETN` 14개는 모두 제외된다.
`XR`, `XSN`, `NNBC`도 query 품사 집합에 없으므로 경로 node가 되지 않는다.

이 누락 때문에 실제 어미 경로 대신 동음이의 품사로 어절을 완성한다. `것이다`의 현재
최저 미포함 경로는 `것/NNB + 이/JKS + 다/MAG`이고, 최저 포함 경로는
`것/NNB + 이/VCP + 다/JX`다. source의 `다/EF`는 어느 경로에도 없다.

## 비용 근거

candidate target과 fixture gold span을 대조해 case label 안의 다른 anchor를 분리했다.

| target 종류 | 평가 candidate | total margin 중앙값 | word delta 중앙값 | connection delta 중앙값 |
| --- | ---: | ---: | ---: | ---: |
| gold target | 925 | 4,744 | 343 | 2,551 |
| non-gold target | 695 | 5,091 | 3,294 | 927 |

gold target에서도 모두 미포함 경로가 낮다. 실제 지정사 target의 reject는 주로 누락된
어미·복합 분석 때문에 잘못된 연결을 선택한 결과다.

동일 source 비용으로 대표 경로를 다시 계산하면 다음과 같다.

| 표면형 | 현재 경로 | source 분석 경로 | 비용 차이 |
| --- | ---: | ---: | ---: |
| `것이다` | 포함 7,342 / 미포함 3,157 | `것/NNB + 이/VCP + 다/EF` 2,001 | source 경로가 미포함보다 1,156 낮음 |
| `인` | atomic `VCP` tail 6,626 | `VCP+ETM` tail 1,958 | 4,668 감소 |
| `일` | atomic `VCP` tail 6,031 | `VCP+ETM` tail 2,821 | 3,210 감소 |

`인`과 `일`의 복합 행은 표면형 하나에 지정사와 관형형 어미가 함께 들어간다. 현재 schema는
단일 `DataFinePos`만 저장하므로 이 분석을 표현할 수 없다.

## 다음 계약 선택

권장안은 corpus-side morphology schema를 source 분석 보존형으로 바꾸는 것이다.

1. query tag의 `DataFinePos`와 corpus node 품사를 분리한다.
2. 기본 어미·접사·단위 명사와 모든 source unknown class를 보존한다.
3. `Inflect`의 품사열과 expression을 보존한다.
4. 축약된 한 음절 node가 query component를 포함하는 조건을 스펙에 정의한다.
5. resource를 다시 생성한 뒤 같은 2,916건에서 판별력을 재측정한다.

최소 품사 몇 개와 VCP 행만 추가하는 방식은 다른 활용에서 같은 누락을 반복하고 source 비용의
의미를 다시 훼손하므로 권장하지 않는다.

MeCab의 CSV와 연결 행렬 형식은 [공식 사전 구조 문서](https://taku910.github.io/mecab/dic-detail.html)를
기준으로 대조했다.
