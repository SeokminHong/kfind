# GSD 인명과 명사형 지정사의 정답 범위

인명 주석을 교정한 GSD 9,314건에서 명사 뒤에 지정사가 주석되고 정답 범위가
어절 전체인 21건을 검토했습니다. `사마월이었고`, `두뇌임이`, `최악임`의 정답
끝을 각각 `사마월`, `두뇌`, `최악`의 끝으로 옮겼습니다. 원문과 형태 주석은
보존했습니다. `공화춘이란`과 `달이면`은 조사 해석이 가능한 기존 판정을 유지했습니다.
[범위 판정](../../tools/morph-compare/gsd-copula-boundary-audit.tsv) ·
[이전 지정사 범위 판정](2026-11-13-gsd-copula-span-review.md)

같은 full POS `smart` 실행 파일로 UTF-8 범위를 비교했습니다. 행렬은 TP, FP, TN, FN
순서이며 비율은 precision, recall, F1입니다. 계약 검토는 양쪽 모두 0건입니다.

| GSD 양성 9,314건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 인명 주석 교정 | 8992, 0, 0, 322 | 100.00, 96.54, 98.24 | 8992, 0, 0, 322 | 100.00, 96.54, 98.24 |
| 지정사 범위 교정 | 8995, 0, 0, 319 | 100.00, 96.58, 98.26 | 8995, 0, 0, 319 | 100.00, 96.58, 98.26 |

교정한 세 건이 부분 일치에서 정확 일치로 바뀌었습니다. 제품 반환 범위는 양쪽에서
같았으며, 검색 실행 파일과 리소스가 같아 성능 비교는 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-copula-span-review.mjs "$name_input" --after-name-review > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `7441ca03d27db97675c7e713ebaa57bd478e29aa5da404a70eabdbdc4fe566e3`입니다.
[실행 파일과 환경](2026-11-13-dependent-noun-particle-contraction.md)
