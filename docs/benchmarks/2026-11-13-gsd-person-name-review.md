# GSD 인명 표제어 주석 교정

`김남길`, `박하선`, `하그리브스`, `사마월`, `카이라`, `김여진`이
나오는 여덟 곳에서 인명의 표제어와 형태소 주석을 교정했습니다. 문장과 UTF-8
정답 범위는 보존했습니다. [위치별 판정](../../tools/morph-compare/gsd-person-name-review.tsv)

같은 full POS `smart` 실행 파일로 정확한 UTF-8 범위를 비교했습니다.
행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall, F1입니다.
계약 검토는 양쪽 모두 0건입니다.

| GSD 양성 9,314건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 명사 표제어 교정 | 8981, 0, 0, 333 | 100.00, 96.42, 98.18 | 8981, 0, 0, 333 | 100.00, 96.42, 98.18 |
| 인명 주석 교정 | 8988, 0, 0, 326 | 100.00, 96.50, 98.22 | 8988, 0, 0, 326 | 100.00, 96.50, 98.22 |

교정한 8건 중 7건이 정확히 일치했습니다. `사마월이었고`는 부분 일치로 남았습니다.
교정 후 부분 일치와 겹치는 반환이 없는 질의는 각각 163건입니다.
검색 코드와 리소스가 같아 성능 비교를 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-proper-name-review.mjs "$noun_input" --person-names > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `132f6c5dd4efea5000e9a9b0cdaa064da9b058226e0b0eff2af4ff5e42b3d365`입니다.
[이전 입력](2026-11-13-gsd-whole-noun-context-review.md) ·
[실행 환경](2026-11-13-gsd-independent-proofreading.md)
