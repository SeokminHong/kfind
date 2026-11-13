# GSD 명사 표제어와 인명 주석 교정

`우리나라` 네 곳과 인명 `한산동` 한 곳의 질의 및 품사를 교정했습니다.
문장과 UTF-8 정답 범위는 보존했습니다.
[위치별 판정](../../tools/morph-compare/gsd-whole-noun-context-review.tsv) ·
[국사편찬위원회 한산동 기록](https://contents.history.go.kr/mobile/kc/view.do?code=kc_age_20&levelId=kc_i201230)

같은 full POS `smart` 실행 파일로 정확한 UTF-8 범위를 비교했습니다.
행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall, F1입니다. 계약 검토는
양쪽 모두 0건입니다.

| GSD 양성 9,314건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 대명사 문맥 교정 | 8976, 0, 0, 338 | 100.00, 96.37, 98.15 | 8976, 0, 0, 338 | 100.00, 96.37, 98.15 |
| 명사·인명 주석 교정 | 8981, 0, 0, 333 | 100.00, 96.42, 98.18 | 8981, 0, 0, 333 | 100.00, 96.42, 98.18 |

부분 일치 4건과 겹치는 반환이 없던 1건이 모두 정확 일치로 바뀌었습니다.
교정 후 부분 일치는 165건, 겹치는 반환 없음은 168건입니다. 검색 실행 파일과
리소스가 같아 성능 비교는 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-context-pos-review.mjs "$pronoun_input" --whole-nouns > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `149aa2f31b2ccaf769bea4afe09ec213b080c47dd9ca37f447cd8177cf2487aa`입니다.
[이전 입력](2026-11-13-gsd-pronoun-context-review.md) ·
[실행 환경](2026-11-13-gsd-independent-proofreading.md)
