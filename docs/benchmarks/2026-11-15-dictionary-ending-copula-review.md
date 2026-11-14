# 사전 예문의 지정사 시작 범위

고정 사전에서 지정사 `이다`의 어간 또는 받침 융합형 앞까지 체언을
포함한 16건을 검토했다. 개별 검증 보고서에서 지정사의 시작을 확인한
15건과 `약골이기로선들`에서 체언 `약골`을 분리한 1건이다.
예문과 원래 범위는 보존했다.
[판정 예시](2026-11-14-final-rieuljira.md#품질) ·
[지정사 검색 범위](copula-boundary.md) ·
[표준국어대사전 -기로선들](https://stdict.korean.go.kr/search/contentViewOne.do?word_no=49070)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
`6a1f24b2` 실행 코드에서 [기존 보정 43건](2026-11-15-dictionary-desiderative-review.md)에
지정사 16건을 추가했다.

| 입력 | 경계 | Raw 행렬 / 비율 (%) | 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 지정사 범위 검토 16건 | Smart·Any | 0, 0, 0, 16 / 0.00, 0.00, 0.00 | 15, 0, 0, 1 / 100.00, 93.75, 96.77 |
| 사전 4,458건 | Smart | 4369, 0, 0, 89 / 100.00, 98.00, 98.99 | 4415, 0, 0, 43 / 100.00, 99.04, 99.52 |
| 사전 4,458건 | Any | 4257, 0, 0, 201 / 100.00, 95.49, 97.69 | 4306, 0, 0, 152 / 100.00, 96.59, 98.27 |

`약골이기로선들`은 `이기로선들`이 정답이지만 두 경계 모두
`이기로`까지만 반환하므로 보정 후에도 누락으로 집계했다.
범위 보정 전후 4,458건의 반환은 동일했다. 평가 주석 변경으로
성능 측정은 생략했다.

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-ending-copula-review.mjs "$desiderative_reviewed" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$raw" "$raw_output"
"$runner" boundary full-pos "$boundary" "$adjusted" "$adjusted_output"
```

입력 SHA-256은 `1c27f7dd2195d37d2e44d009b8e99215d4a2ee348c4c05283ff7e136d09acd59`,
출력은 `b71de3ffffcb86a27c668093bdd5615a17beb39091389a005c22d3cd6aa29bcd`다.
