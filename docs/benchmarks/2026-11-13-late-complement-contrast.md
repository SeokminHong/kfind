# 늦다의 일정 보어와 시간 부사어

한국어기초사전의 동사 `늦다`에는 일정 보어를 받는 문형이 있습니다.
형용사 `늦다`는 시각이나 속도의 상태를 나타냅니다. `면접에 늦죠`,
`마감에 늦죠`, `회의에 자주 늦죠`의 `에`는 일정을 나타내지만
`해가 겨울에 늦죠`, `시계가 아침에 늦죠`에서는 시간 부사어를 만듭니다.
조사와 표면 품사가 같은 두 문형은 보어와 부사어를 구분할 근거가 필요합니다.
[동사 문형](https://krdict.korean.go.kr/m/eng/searchResultView?ParaSenseSeq=&ParaWordNo=61181&fileNo=&imgCount=&multiMediaSeq=&nation=eng&searchKind=&searchKindValue=&shortenUrl=&studySeq=) ·
[형용사 문형](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=64526)

GSD 양성 9,314건, PUD 양성 798건, GSD·PUD 음성 53,308건, 사전
예문 4,458건 전체에서 `늦지요`·`늦죠`를 검색한 결과는 0건입니다.
별도 회귀 입력에서는 `늦다`의 11개 질의에서 기존 형용사 오탐 3건을
재현했습니다. 과거 #402의 구현은 일정 명사 목록에 의존했고 #415에서
제거됐습니다. 이번 대조 입력에는 다른 일정 명사와 부사가 사이에 오는
경우를 포함했습니다. 일정 보어의 형용사 오탐은 그대로 집계했습니다.
[기존 회귀](../../data/fixtures/jiyo_span_cases.tsv) ·
[대조 입력](../../tools/morph-compare/late-complement-contrast-cases.jsonl)

기준선 `e0b590fc7413051044d2c0127f2eaf3b85afac81`과 검색 코드가 동일한
`8581bac664494819874fd78381ac0030879e71c5` 실행 파일을 사용했습니다.
행렬은 TP, FP, TN, FN이며 비율은 precision, recall, F1입니다.
계약 검토는 0건입니다.

| 입력 | 경계 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| 일정 보어·시간 부사어 9건 | smart | 6, 3, 0, 0 | 66.67, 100.00, 80.00 | 6, 3, 0, 0 | 66.67, 100.00, 80.00 |
| 일정 보어·시간 부사어 9건 | any | 6, 3, 0, 0 | 66.67, 100.00, 80.00 | 6, 3, 0, 0 | 66.67, 100.00, 80.00 |

형용사 오탐을 줄이려면 사전 문형이나 이에 준하는 구조 근거를 검색기가
사용할 수 있어야 합니다. 현재 리소스는 보어와 부사어의 역할을 표시하지
않습니다. 검색 코드를 수정하지 않아 성능 측정을 생략했습니다.

```sh
"$runner" boundary full-pos "$boundary" tools/morph-compare/late-complement-contrast-cases.jsonl "$output"
```

입력 SHA-256은 `04a42d65ca7983245582b36cc792bbb0a6f9079945fdbdfa24c63838ab23b990`입니다.
