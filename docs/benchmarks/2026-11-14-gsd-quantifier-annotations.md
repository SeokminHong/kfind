# GSD 수량 관형사와 단위 명사 주석

교정 GSD 양성 9,314행에서 질의가 `수`, `주`, `전`인 87행을 문맥과
대조했습니다. `수 주 전부터`에서 `수`는 수량 관형사이고 `주`는 기간을
세는 단위 의존 명사입니다. `심사 전 과정`의 `전`은 전체를 뜻하는
관형사입니다. 세 행의 품사를 교정하고 문장과 UTF-8 범위를 보존했습니다.
`발매 전에는`처럼 시간 관계를 나타내는 명사는 기존 주석을 유지했습니다.
[`주`의 사전 풀이](https://krdict.korean.go.kr/jpn/dicMarinerSearch/search?mainSearchWord=%EC%A3%BC) ·
[`전 과정`의 품사](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=261&pageIndex=1&qna_seq=318151) ·
[교정 목록](../../tools/morph-compare/gsd-quantifier-annotations.tsv)

검색 코드가 같은 `4af9c526` full POS 실행 파일로 병합된 `e2782989`의
기준 입력과 교정 입력을 비교했습니다. 행렬은 TP, FP, TN, FN 순이고 비율은
precision, recall, F1 순입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9113, 0, 0, 201 | 100.00, 97.84, 98.91 | 9113, 0, 0, 201 | 100.00, 97.84, 98.91 |
| smart | 교정 후 | 9115, 0, 0, 199 | 100.00, 97.86, 98.92 | 9115, 0, 0, 199 | 100.00, 97.86, 98.92 |
| any | 양쪽 | 8938, 0, 0, 376 | 100.00, 95.96, 97.94 | 8938, 0, 0, 376 | 100.00, 95.96, 97.94 |

`smart`에서 두 관형사의 반환 범위만 바뀌었고, 단위 명사 `주`는 계속
누락됩니다. #421 불일치 1,113행의 `smart` 결과는 정확 일치 928행,
부분 일치 86행, 겹치는 반환 없음 99행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --quantifier-annotations > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `13bf24075dd714060f0366e04ddcf17a385bc902cc704a88d1d6cda3457a2152`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
