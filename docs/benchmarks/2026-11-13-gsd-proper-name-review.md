# GSD 조인성의 고유 명사 주석

`test-s782`는 배우 조인성의 입대와 활동을 다룹니다. 원본의 `조인+성+은`
분석을 `조인성+은`으로, 표제어를 `조인성`으로 교정했습니다. 정답 범위
`[0, 12)`와 원문은 유지했습니다.
[주석 판정](../../tools/morph-compare/gsd-proper-name-review.tsv)

동일한 full POS `smart` 실행에서 양성 UTF-8 범위를 비교했습니다. 행렬은 TP, FP,
TN, FN, 비율은 precision, recall, F1 순서입니다. 계약 검토는 두 입력 모두 0건입니다.

| GSD 양성 9,332건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 파생 접미사 범위 교정 | 8962, 0, 0, 370 | 100.00, 96.04, 97.98 | 8962, 0, 0, 370 | 100.00, 96.04, 97.98 |
| 인명 주석 교정 | 8963, 0, 0, 369 | 100.00, 96.05, 97.98 | 8963, 0, 0, 369 | 100.00, 96.05, 97.98 |

교정한 질의가 `[0, 12)`에 정확히 일치했습니다. 나머지 9,331건의 반환 범위는
같았습니다. 교정 후 부분 일치는 175건이고 겹치는 반환이 없는 질의는 194건입니다.

```sh
node tools/morph-compare/apply-gsd-proper-name-review.mjs "$suffix_review_input" > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `dfe68b16383e6dca96829cd30ce9051e939029f67b16479f1254eadf8a864e6a`입니다.
[이전 교정 입력](2026-11-13-gsd-nominal-suffix-review.md)
