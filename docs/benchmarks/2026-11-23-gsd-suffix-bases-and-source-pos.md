# GSD 접미사 앞 명사 범위와 원문 품사

원문 GSD 계약 미검출 548행 가운데 등록 교정문과 본문·질의·품사가
같고 정답 좌표만 다른 218행을 찾았다. 그중 원문 형태가 접미사
`XSN`을 포함하는 네 행을 모두 검토했다. 문장·질의·품사·좌표까지
같은 세 행은 교정 평가에 있던 품사 근거를 원문과 대조했다.
[GSD r2.18 원문](https://github.com/UniversalDependencies/UD_Korean-GSD/blob/r2.18/ko_gsd-ud-test.conllu)

| 원문 GSD 행 | 원문 범위 | 판정 |
| --- | --- | --- |
| s202:9 | 예수님과 68..80 | 높임 접미사 앞의 예수 68..74 |
| s366:17 | 요드제의 177..189 | 원문의 해당 어절을 제외하고 교정문 요오드 177..186을 검사 |
| s558:9 | 대구은행장상 106..124 | 상을 수여하는 직함 대구은행장 106..121 |
| s794:1 | 이슬람권에서는 0..21 | 범위를 뜻하는 접미사 앞의 이슬람 0..9 |
| s220:8 | 만으로는 45..57 | 제품명을 한정하는 보조사 만 |
| s757:5 | 더 37..40 | 작품명에 들어간 영어 정관사 |
| s779:1 | 이 0..3 | 때문 앞에 쓰인 지시 대명사 |

`예수님`은 한국어기초사전에서 높임 접미사 `-님`의 용례로 확인했다.
`대구은행장상`은 대구은행 보도자료의 수상 명칭이다.
`이슬람권`의 `-권`은 특정 범위에 속하는 지역을 나타낸다.
[높임 접미사 -님](https://krdict.korean.go.kr/rus/dicSearch/SearchView?ParaWordNo=88852)
· [대구은행 사생대회 보도자료](https://www.newswire.co.kr/newsRead.php?no=547775)
· [국립국어원 접미사 -권](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=&pageIndex=1&qna_seq=331102&searchCondition=&searchKeyword=)

국립국어원의 남북한 외래어 비교 자료에서 `요드`는 북한 표기다.
등록 교정문에는 남한 표준 표기인 `요오드`를 사용했다. 원문 `요드`의
반환 177..183과 교정문 `요오드`의 반환 177..186을 같은 러너에서
확인했다. 교정문의 명사 `제` 질의가 접미사와 조사 `제의 [186, 192)`를
반환하는 별도 오탐도 확인했다. 동일 어절의 약을 뜻하는 `-제(劑)`와
수량 뒤에서 쓰는 의존 명사 `제(劑)`의 결합 조건이 다르다.
이 오탐은 교정 양성에 없는 진단 질의에서 발견됐다.
[요오드 표기](https://www.korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=8342&mn_id=217&pageIndex=4)
· [남북한 외래어 비교](https://www.korean.go.kr/common/download.do?c_file_name=0757a5e9-da30-4a2c-bde7-8c6cf719f168_0.pdf&file_path=reportData)
· [한국어기초사전 제와 -제](https://krdict.korean.go.kr/eng/dicMarinerSearch/search?mainSearchWord=%EC%A0%9C)

`'두잇'만으로는`의 `만`은 인용한 제품명을 한정한다.
`더 클로저`는 Television Academy 수상 기록의 `The Closer`다.
`이 때문에`에서 앞 내용의 원인을 가리키는 `이`는 사전 문형에 따라
대명사다.
[명사·한정 보조사·격조사 결합](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=261&pageIndex=1&qna_seq=325335)
· [Television Academy 수상 기록](https://www.televisionacademy.com/awards/nominees-winners/2010/outstanding-lead-actress-in-a-drama-series)
· [국립국어원 지시 대명사와 때문](https://m.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=313417&searchCondition=&searchKeyword=)

## 평가

`2a7786ae`와 동일한 제품 트리의 Smart full-pos release에서 같은
리소스를 사용했다. 교정문 9,298행 중 s366의 18행에 교정을 반영했다.
반환 좌표가 바뀐 네 행은 표기 교정과 그 뒤 어절의 바이트 이동을
대조했다. 새 교정 입력 SHA-256은
`0a6ae6694670697f41786fe3040024c865df670908ebee9e6af2cd7f260013cf`다.
[입력 재현](../../tools/morph-compare/README.md#gsdpud-고정-감사-입력)
· [러너와 리소스](2026-11-23-degree-adverb-before-possibility.md#성능)

행렬 순서는 TP, FP, TN, FN이며 뒤의 세 값은 정밀도·재현율·F1(%)이다.

| 입력 | 원본 행렬 및 지표 | 계약 행렬 및 지표 | FPc / FNc | 검토 / 제외 |
| --- | --- | --- | ---: | ---: |
| 원문 GSD 기존 | 8,225, 0, 0, 1,110 / 100, 88.11, 93.68 | 8,648, 9, 92, 548 / 99.90, 94.04, 96.88 | 20 / 548 | 612 / 38 |
| 원문 GSD 후보 | 8,225, 0, 0, 1,110 / 100, 88.11, 93.68 | 8,651, 9, 95, 541 / 99.90, 94.11, 96.92 | 20 / 541 | 619 / 39 |
| 교정 GSD 양쪽 | 9,286, 0, 0, 12 / 100, 99.87, 99.94 | 9,285, 2, 11, 0 / 99.98, 100, 99.99 | 6 / 0 | 149 → 153 / 0 |
| 고정 음성 양쪽 | 0, 3,026, 50,282, 0 / 0, 0, 0 | 1,430, 1,481, 50,248, 0 / 49.12, 100, 65.88 | 1,483 / 0 | 1,589 / 149 |

원문 FNc 7건 감소는 정답 범위 확정 3건, 품사 판정 3건, 비표준
범위 제외 1건의 결과다. 원문 양성의 추가 반환 미판정은 515행
573범위에서 511행 569범위로 줄었다. 교정 양성은 86행 92범위다.
