# GSD 문맥에 따른 품사 주석 교정

문장에서 대명사·관형사·부사·동사로 쓰인 표면을 명사로 주석한 9개 위치의
질의와 품사를 교정했습니다. 원문, 이전 질의, UTF-8 정답 범위를 보존했습니다.
[위치별 판정](../../tools/morph-compare/gsd-context-pos-review.tsv)

같은 full POS `smart` 실행 파일을 사용했습니다. 행렬은 TP, FP, TN, FN 순서이며
비율은 precision, recall, F1입니다. 계약 검토는 양쪽 모두 0건입니다.

| GSD 양성 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 이전 교정 9,316건 | 8964, 0, 0, 352 | 100.00, 96.22, 98.07 | 8964, 0, 0, 352 | 100.00, 96.22, 98.07 |
| 품사 주석 교정 9,316건 | 8968, 0, 0, 348 | 100.00, 96.26, 98.10 | 8968, 0, 0, 348 | 100.00, 96.26, 98.10 |

바뀐 9건 중 5건이 정확히 일치했습니다. 나머지 4건은 `우리`의 축약형 `울`과
`저는`의 축약형 `전`을 검색하는 대명사 질의이며, 겹치는 반환이 없었습니다.
교정 대상 외 9,307건의 반환은 동일했습니다. 검색 실행 파일이 같아 기존 고정 입력의
성능 비교를 사용합니다.

```sh
node tools/morph-compare/apply-gsd-context-pos-review.mjs "$joined_word_input" > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `120ad5000b84b3d61fef05214b6d703e0f573eb55330f0741cf455583806dce9`입니다.
[이전 교정 입력](2026-11-13-gsd-joined-word-review.md)
