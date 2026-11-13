# GSD 명사 표제어와 파생 접미사 범위

`대학생과`의 `대학`, `입법권을`의 `입법`, `유동성`의 `유동`처럼
주석의 명사 표제어에 파생 접미사가 붙은 54건을 문장별로 확인했습니다. 표제어 질의의
정답 끝을 접미사 앞으로 옮겼습니다. 배우 이름 `조인성`처럼 같은 글자로 끝나도
별개의 고유명사인 사례는 이 범위 교정에 포함하지 않았습니다.
[문장별 판정](../../tools/morph-compare/gsd-nominal-suffix-review.tsv) ·
[국립국어원 손과 손님의 구성](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=315948) ·
[검색 사양](../../specs/matching.md)

동일한 `3d1c243dc87d11de865b592cd1653ca4f39bbad1` full POS `smart` 실행 파일로
정확한 UTF-8 범위를 채점했습니다. 행렬은 TP, FP, TN, FN, 비율은 precision, recall,
F1 순서입니다. 양쪽 입력에 계약 검토가 0건입니다.

| GSD 양성 9,332건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 접미사 적까지 교정 | 8908, 0, 0, 424 | 100.00, 95.46, 97.68 | 8908, 0, 0, 424 | 100.00, 95.46, 97.68 |
| 명사 파생 접미사 범위 교정 | 8962, 0, 0, 370 | 100.00, 96.04, 97.98 | 8962, 0, 0, 370 | 100.00, 96.04, 97.98 |

54건이 부분 일치에서 정확 일치로 바뀌었습니다. 전체 반환 범위가 같았습니다.
부분 일치는 176건이고 겹치는 반환이 없는 질의는 194건입니다.
제품 실행 파일이 같아 성능 비교에는 기존 고정 입력 측정을 사용합니다.

```sh
node tools/morph-compare/apply-gsd-nominal-suffix-review.mjs "$jeok_review_input" > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `6606de93e41ce3cd475b5469c8d6673c087877e73549218e6c64c10d369a615a`입니다.
[이전 교정 입력](2026-11-13-gsd-jeok-span-review.md)
