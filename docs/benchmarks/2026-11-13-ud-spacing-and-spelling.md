# GSD 표기와 어절 범위 교정

GSD 15문장의 표기와 띄어쓰기를 교정하고 16개 질의의 UTF-8 범위를 다시 정렬했습니다.
`원하는 대로`, `되나 봐요`, `촉구해 왔다` 등은 본용언의 어절을 표적으로 삼았습니다.
`평평한 곳 잇어서`와 `내비 키고`는 표기와 함께 표제어도 각각 `있다`와 `켜다`로
교정했습니다. [한글 맞춤법](https://korean.go.kr/kornorms/regltn/regltnView.do) ·
[국립국어원 켜다 용례](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=305514) ·
[UD 회귀](../../data/fixtures/ud_morphology_cases.tsv)

두 입력은 같은 `8cd83687b36e61ca52796e112df7e3b506bc1732` 실행 파일에서
`boundary full-pos smart`로 비교했습니다. 양성은 정확한 범위로 채점했습니다.
행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall, F1입니다.
계약 검토는 두 입력 모두 0건입니다.

| 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| GSD 원문 16행 | 0, 0, 0, 16 | 0.00, 0.00, 0.00 | 0, 0, 0, 16 | 0.00, 0.00, 0.00 |
| GSD 교정 16행 | 16, 0, 0, 0 | 100.00, 100.00, 100.00 | 16, 0, 0, 0 | 100.00, 100.00, 100.00 |

두 입력을 원문·교정 순서로 번갈아 담은 JSONL의 SHA-256은
`12e40a5afd49d289adca7138455b70120cad4cddc45f407211a92aa8bb942971`입니다.
[공통 리소스 SHA-256](2026-11-13-grammatical-nominal-suffixes.md#성능)
