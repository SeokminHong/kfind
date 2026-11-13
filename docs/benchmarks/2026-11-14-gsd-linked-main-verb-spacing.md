# GSD에서 연결된 본용언의 띄어쓰기

교정 GSD 9,306행에서 품사열 `VV+EC+VV` 2행과
`VV+EC+VX` 102행을 검토했다. 앞뒤 동사가 독립된 동작을
나타내는 `골라 먹을`, `불러 모아`, `날려 보냈다`,
`비벼 먹으면`, `시켜 먹어요`를 띄어 썼다. 문장 5개의
58행을 정렬하고 첫 동사에 해당하는 UTF-8 정답 범위 5개를
다시 계산했다. [한글 맞춤법](https://www.korean.go.kr/kornorms/m/m_regltn.do) ·
[날려 보내다](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=314048) ·
[원문과 교정 근거](../../tools/morph-compare/gsd-linked-main-verb-spacing-edits.tsv)

검색 실행 파일은 `f74262cbfeced1458e6c0fd0a45a7b9a8cae9493`과
소스가 같다. 양성 검토는 0건이며 계약 보정 값은 raw와 같다.
행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall, F1이다.

| full POS 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9237, 0, 0, 69 | 100.00, 99.26, 99.63 | 9237, 0, 0, 69 | 100.00, 99.26, 99.63 |
| smart | 교정 후 | 9240, 0, 0, 66 | 100.00, 99.29, 99.64 | 9240, 0, 0, 66 | 100.00, 99.29, 99.64 |
| any | 교정 전 | 9040, 0, 0, 266 | 100.00, 97.14, 98.55 | 9040, 0, 0, 266 | 100.00, 97.14, 98.55 |
| any | 교정 후 | 9045, 0, 0, 261 | 100.00, 97.20, 98.58 | 9045, 0, 0, 261 | 100.00, 97.20, 98.58 |

#421의 비교군 1,113행에서 `smart`는 정확 1,053행, 부분
22행, 누락 38행이며 `any`는 정확 932행, 부분 169행, 누락
12행이다. 검색 코드 변경이 없어 성능 측정을 생략했다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --linked-main-verb-spacing > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

입력 SHA-256은 `e3a6c7a8d26b07420e45c11c70ba9ff154122536b5caae8fac0acc18a4d21896`,
교정 SHA-256은 `da4d9c6691f3cbd8e9eccaaddbe0ebc061cfca843cedd656ef9ddfa70865e3a7`이다.
