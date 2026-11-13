# GSD 명사와 접미사 적의 정답 범위

원문과 주석에서 명사 표제어 뒤에 접미사 `-적(的)`이 붙은 52건의 정답을
표제어 범위로 맞췄습니다. `투기적`의 `투기`, `경쟁적으로`의 `경쟁`,
`구조적인`의 `구조`가 여기에 해당합니다. `해적이`의 `적(賊)`은 다른
형태소입니다. 이 사례는 명사 표제어를 `해적`으로 교정하고 주격 조사까지의
정답 범위를 보존했습니다.
[국립국어원 접미사 풀이](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=333318) ·
[한국어기초사전 해적](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=84783) ·
[회귀 예문](../../data/fixtures/ud_morphology_cases.tsv)

같은 `3d1c243dc87d11de865b592cd1653ca4f39bbad1` full POS `smart` 실행 파일로
UTF-8 정확 일치를 채점했습니다. 행렬은 TP, FP, TN, FN, 비율은 precision, recall,
F1 순서입니다. 양쪽 입력의 계약 검토는 0건입니다.

| GSD 양성 9,332건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 지정사 범위 교정 입력 | 8855, 0, 0, 477 | 100.00, 94.89, 97.38 | 8855, 0, 0, 477 | 100.00, 94.89, 97.38 |
| 접미사 범위와 표제어 교정 | 8908, 0, 0, 424 | 100.00, 95.46, 97.68 | 8908, 0, 0, 424 | 100.00, 95.46, 97.68 |

범위 교정 52건과 표제어 교정 1건이 정확 일치했습니다. 나머지 반환 범위는
같았습니다. 부분 일치 230건, 겹치는 반환이 없는 질의 194건입니다.
`verify-gold`의 기존 불일치 15건은 동일했습니다. 제품 실행 파일이 같으므로
성능 비교에는 기존 고정 입력 측정을 사용합니다.

```sh
node tools/morph-compare/apply-gsd-jeok-span-review.mjs "$copula_review_input" > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `3161e0bb04c405518322600dce3188f0fd94f50f85b5ca05f2975360379b3919`입니다.
[이전 교정 입력](2026-11-13-gsd-copula-span-review.md)
