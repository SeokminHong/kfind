# local lattice 비용 분석

대상: 지정사 local-context 2,916건의 schema 4 shadow report

## 결론

schema 2에서 모든 candidate가 `reject`된 원인은 threshold가 아니라 morphology resource의
분석 행 누락이었다. query-side 품사 집합을 corpus-side 필터로 재사용하면서 어미, 접사,
숫자와 복합 `Inflect` 분석이 제거되었다.

schema 3은 source 분석과 모든 unknown class를 보존해 1,647개 candidate를 모두 평가한다.
gold target recall은 94.65%지만 non-gold target reject 비율은 24.44%다. source 최저 비용은
경로 존재 문제를 해결했지만 제품 판정에 필요한 구분력은 부족하므로 P3 적용은 보류한다.

## schema 2 실패 원인

- 오류 candidate는 26개, 고유 분석 어절은 16개였다.
- 모든 오류 어절이 숫자를 포함했다. 예: `1주일`, `1박2일`, `68억이다`, `12명이고`.
- HANGUL 미등록어 node만 구성해 source의 `NUMERIC` unknown class와 `SN` 분석을 사용하지
  못했다.
- source 816,283개 행 중 57,923개를 제외했고, `Inflect` 44,820개 중 723개만 보존했다.
- `EC`, `EF`, `EP`, `ETM`, `ETN`, `XR`, `XSN`, `NNBC`가 경로에서 누락됐다.

이 누락 때문에 `것이다`는 실제 `것/NNB + 이/VCP + 다/EF` 대신
`것/NNB + 이/JKS + 다/MAG`를 최저 경로로 선택했다. `인`, `일`의 `VCP+ETM` 복합 행도
단일 query 품사만 저장하는 schema로는 표현할 수 없었다.

## schema 3 결과

resource는 NFC 표면형 773,105개와 source 분석 815,725개, 3,822×2,693 연결 비용 행렬을
보존한다. SHA-256은
`50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23`다.

| target 종류 | candidate | accept | reject | accept 비율 |
| --- | ---: | ---: | ---: | ---: |
| gold target | 935 | 885 | 50 | 94.65% |
| non-gold target | 712 | 538 | 174 | 75.56% |

- 1,647개 candidate가 모두 `evaluated`였고 오류와 `ambiguous`는 없었다.
- 전체 판정은 `accept` 1,423개, `reject` 224개다.
- 완전 경로는 candidate마다 최대 4개, node는 최대 175개로 상한 안에 있다.
- report 경로에 `VCP+ETM` 등 복합 품사가 원문 그대로 남는다.
- union 검색 결과, CLI와 timed 성능 경로는 바뀌지 않는다.

case label에는 동일 문장의 다른 anchor도 포함될 수 있어 판별력은 candidate target과 fixture
gold span의 byte 중첩으로 계산했다. 이 기준에서 precision은 62.19%, recall은 94.65%,
specificity는 24.44%다. threshold를 추가하기 전에 별도 blind source에서 비용 분포와
non-gold 오수용 원인을 확인해야 한다.

MeCab의 CSV와 연결 행렬 형식은 [공식 사전 구조 문서](https://taku910.github.io/mecab/dic-detail.html)를
기준으로 대조했다.
