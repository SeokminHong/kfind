# GSD 보조 용언 오다의 띄어쓰기

GSD에서 `오` 형태와 보조 용언 주석이 함께 있는 5행을
문맥과 대조했다. 4행의 `오`는 보조 용언 `오다`의 어간이고,
`보여주시오`의 `오`는 종결 어미다. 세 음절 이상의 파생 동사
활용형 뒤에 붙어 있던 `발전시켜온`, `수행해왔습니다`를
`발전시켜 온`, `수행해 왔습니다`로 고쳤다.
붙여 쓰기가 허용되는 `해온`과 `있어왔다`는 원문을 보존했다.
[국립국어원 제47항 해설](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&qna_seq=319792) ·
[원문 교정](../../tools/morph-compare/gsd-oda-spacing-edits.tsv) ·
[주석 교정](../../tools/morph-compare/gsd-oda-spacing-annotations.tsv)

동일한 `5ec20bc168f24af3a8ddfdf01f421b5626b1a9bb` full POS
실행 파일로 GSD 9,306행을 비교했다. 두 경계에서 교정한
본용언 활용형 2행의 전체 범위가 정확히 일치했다.
행렬은 TP, FP, TN, FN 순서이고 비율은 precision, recall, F1이다.
계약 검토는 0건이다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9231, 0, 0, 75 | 100.00, 99.19, 99.60 | 9231, 0, 0, 75 | 100.00, 99.19, 99.60 |
| smart | 교정 | 9233, 0, 0, 73 | 100.00, 99.22, 99.61 | 9233, 0, 0, 73 | 100.00, 99.22, 99.61 |
| any | 기준선 | 9034, 0, 0, 272 | 100.00, 97.08, 98.52 | 9034, 0, 0, 272 | 100.00, 97.08, 98.52 |
| any | 교정 | 9036, 0, 0, 270 | 100.00, 97.10, 98.53 | 9036, 0, 0, 270 | 100.00, 97.10, 98.53 |

#421의 불일치 1,113행 중 `smart`는 정확 1,046행, 부분 23행,
누락 44행이다. `any`는 각각 926행, 174행, 13행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --oda-spacing > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `e3a6c7a8d26b07420e45c11c70ba9ff154122536b5caae8fac0acc18a4d21896`이다.
검색 코드 변경이 없어 성능 측정을 생략했다.
