# GSD 지정사 앞 명사 정답 범위 재검토

전수 교정 GSD에서 명사 표제어 뒤 지정사와 어미까지 정답에 포함된 22건을
문장별로 확인해 명사의 끝으로 옮겼습니다. `정도여서`의 `정도`와
`마일인가요`의 `마일`이 여기에 해당합니다.
[문장별 판정](../../tools/morph-compare/gsd-copula-span-review.tsv) ·
[검색 사양](../../specs/matching.md)

`공화춘이란 이름`은 화제 보조사 해석과 관형형 해석이 겹칩니다.
`한 달이면 되겠지`는 조건 조사와 지정사 해석이 겹칩니다. 두 사례의 기존 범위는
유지했습니다. `센트랄`, `안전구`, `가실라면`에는 명사 뒤 지정사 범위 교정을
적용하지 않았습니다.

같은 full POS `smart` 실행 파일(`ca2d8dc62cf42e8e9f6004b6a8cd8e0f783753c7`)에서
9,332건의 반환 범위가 모두 동일했습니다. 행렬은 TP, FP, TN, FN, 비율은 precision,
recall, F1 순서입니다. 두 입력 모두 계약 검토 0건입니다.

| GSD 양성 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 전수 교정 입력 | 8834, 0, 0, 498 | 100.00, 94.66, 97.26 | 8834, 0, 0, 498 | 100.00, 94.66, 97.26 |
| 지정사 앞 범위 교정 | 8855, 0, 0, 477 | 100.00, 94.89, 97.38 | 8855, 0, 0, 477 | 100.00, 94.89, 97.38 |

교정한 22건 중 21건이 정확 일치했습니다. `금리를 동결할 것이란 시장 예상`의
`것`은 지정사까지 반환해 부분 일치입니다. 전체 부분 일치는 282건이고
겹치는 반환이 없는 경우는 195건입니다.

```sh
node tools/morph-compare/apply-gsd-copula-span-review.mjs "$proofread_input" > "$corrected_input"
"$runner" boundary full-pos smart "$proofread_input" "$previous_output"
"$runner" boundary full-pos smart "$corrected_input" "$corrected_output"
```

교정 입력 SHA-256은 `ad32efd10519f4c4100ab3107e3910074c68b28c67f48b14a15716a9da90d3ce`입니다.
[전수 교정 입력](2026-11-13-gsd-full-proofreading.md) ·
[실행 환경과 성능](2026-11-13-directed-prospective.md)
