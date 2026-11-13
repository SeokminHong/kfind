# GSD 띄어쓰기 교정과 주석 정렬

기존 띄어쓰기 교정표에서 공백만 삭제한 39건을 대조했다.
붙여 쓴 위치에 서로 맞닿은 옛 주석이 남아 있던
`전화번호별로`, `역모죄로`, `붉은빛을`, `창밖의`,
`달간의`, `모래주머니` 6곳을 확인했다. 각 위치를 한 행으로
정렬해 중복된 6행을 병합했다. 문장 표기는 유지했다.
`달간의`에서 기간 접미사 `-간` 앞의 명사 범위를 유지했다.
[국립국어원 기간 접미사 설명](https://korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=311730) ·
[교정 주석](../../tools/morph-compare/gsd-proofreading-join-annotations.tsv)

같은 구어 조건형 후보 실행 파일로 비교했다. 교정 전 지표는
병합 후 남은 ID 9,300행으로 맞췄다. 이 비교의 계약 검토는
0건이며 행렬은 TP, FP, TN, FN, 비율은 precision, recall, F1
순서다.

| 경계 | 주석 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9256, 0, 0, 44 | 100.00, 99.53, 99.76 | 9256, 0, 0, 44 | 100.00, 99.53, 99.76 |
| smart | 교정 후 | 9257, 0, 0, 43 | 100.00, 99.54, 99.77 | 9257, 0, 0, 43 | 100.00, 99.54, 99.77 |
| any | 양쪽 | 9062, 0, 0, 238 | 100.00, 97.44, 98.70 | 9062, 0, 0, 238 | 100.00, 97.44, 98.70 |

교정한 6행 중 `smart` 5행과 `any` 6행의 범위가 정확히
일치했다. `smart`에서 `달간의`의 `달`은 누락됐다.
#421 비교군 1,113행의 ID는 유지됐다. 교정 후 `smart`는
정확 1,071행, 부분 15행, 누락 27행이다. `any`는 정확 947행,
부분 159행, 누락 7행이다. 검색 코드 변경이 없어 성능 측정을
생략했다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --proofreading-joins > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

입력 SHA-256은 `bc9e38af4f43f07f27e10d10d483afbfdda1c4d7b4236ddf28b4519df64747ae`,
교정 SHA-256은 `a928472937bc4f14f40b16d24446f3b5a94a1748ad38c561680d979bfaafbed5`다.
