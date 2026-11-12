# GSD의 고요와 더라고요 표기 교정

`맛있구요`, `괜찮더라구요`, `좋더라구요`를 표준 표기로 교정하고
반환 범위를 다시 정렬했습니다.
[국립국어원 말뭉치 전사 지침](https://www.korean.go.kr/common/download.do?c_file_name=bde4951e-ba07-4305-8f48-2175038d3299.pdf&file_path=etcData&o_file_name=2023%EB%85%84+%EC%9D%BC%EC%83%81+%EB%8C%80%ED%99%94+%EB%A7%90%EB%AD%89%EC%B9%98+%EA%B5%AC%EC%B6%95.pdf) ·
[UD 회귀](../../data/fixtures/ud_morphology_cases.tsv)

같은 `9abbbbf4a8622d019d2a22405cb8497767718317` 실행 파일로 명시적 품사
`full-pos smart`의 정확한 범위를 채점했습니다. 행렬 순서는 TP, FP, TN, FN이며
비율은 precision, recall, F1 순서입니다. 계약 검토는 두 입력 모두 0건입니다.

| 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| GSD 원문 3행 | 0, 0, 0, 3 | 0.00, 0.00, 0.00 | 0, 0, 0, 3 | 0.00, 0.00, 0.00 |
| GSD 교정 3행 | 3, 0, 0, 0 | 100.00, 100.00, 100.00 | 3, 0, 0, 0 | 100.00, 100.00, 100.00 |

| 입력 | SHA-256 |
| --- | --- |
| 원문 JSONL | `dcef6d679edb4967a64e69dfcf385f2dd7215184bbed33af2235e9504413e516` |
| 교정 JSONL | `1dfa9e08a5262be0397784d13984f7891f1a9b1774bce729a89d71de9affc6d1` |
