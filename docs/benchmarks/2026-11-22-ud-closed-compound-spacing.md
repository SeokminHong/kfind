# GSD·PUD 합성어 내부의 띄어쓰기

UD 2.18 GSD·PUD에서 `이날·그날·이때·그때·이곳·올여름·한눈·모래주머니`와
내부에 공백을 둔 표기를 함께 찾았다. `이 때문에`처럼 다른 단어이거나 공백이
합성어 경계에 해당하지 않는 경우를 가려 GSD 31위치와 PUD 5위치를 확인했다.
고정 음성에서 오탐이 합성어 내부의 공백에 걸린 경우를 원문과 대조했다.
[국립국어원 이날의 표기](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=313732)
· [국립국어원 이때·그때의 표기](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=&pageIndex=1&qna_seq=320357)

GSD 8문장과 PUD 3문장에서 합성어 한 곳씩을 확인했다. 지시 대상인 `이날·이때·그때`,
계절인 `올여름`, 한 번 보는 뜻인 `한눈`, 차단벽 재료를 가리키는 `모래주머니`다.
그 자리의 공백만 지운 입력에서 15개 기존 질의는 반환이 없었고, 짝지은 표준
표기의 질의 15개는 각각 지정한 전체 범위만 반환했다. 원본 평가는 유지하고
이 15행을 계약 평가에서 제외했다.
[국립국어원 올여름의 표기](https://www.korean.go.kr/common/download.do?c_file_name=54ccc574-cfad-4ee0-b766-c62987dc369b_0.pdf&file_path=etcData&o_file_name=%ED%95%9C%EB%88%88%EC%97%90+%EC%95%8C%EC%95%84%EB%B3%B4%EB%8A%94+%EC%8B%A0%EB%AC%B8%EC%96%B8%EC%96%B4+%EB%B0%94%EB%A1%9C+%EC%93%B0%EA%B8%B0.pdf)
· [국립국어원 한눈의 쓰임](https://www.korean.go.kr/nkview/nklife/2006_4/2006_0413.pdf)
· [국립국어원 모래주머니](https://www.korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=8626&mn_id=62&pageIndex=43)

## 품질

`0367d61e`와 동일한 제품 코드의 고정 11세트에서 FNc는 0, FPc는
1,704→1,689다. 고정 음성 53,308행의 원본 반환 범위는 동일하다.

| 등록부 | 원본 TP, FP, TN, FN / precision, recall, F1 (%) | 계약 TP, FP, TN, FN / precision, recall, F1 (%) | 계약 제외 |
| --- | --- | --- | ---: |
| 기준선 | 0, 3,227, 50,081, 0 / 0, —, 0 | 1,432, 1,704, 50,054, 0 / 45.66, 100, 62.70 | 118 |
| 검토 후 | 0, 3,227, 50,081, 0 / 0, —, 0 | 1,432, 1,689, 50,054, 0 / 45.88, 100, 62.90 | 133 |

제품 코드와 사전 리소스는 동일하므로 기존 성능 기준선을 사용한다.
