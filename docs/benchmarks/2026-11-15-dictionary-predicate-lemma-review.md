# 사전 예문의 용언 표제어와 품사

고정 사전에서 `숨바꼭질`, `한잔`, `잘하/잘했/잘합`,
`체하/체했`, `그만하/그만했`, `저세`, `않으니라`, `못하/못했`로
시작하는 활용 29건을 대조했다. 전체 용언의 표제어와 품사 오류
10건을 보정하고 원문과 범위를 보존했다. `말리지 못했거든`의
보조 용언 결합 범위는 별도 검토 대상으로 남겼다.
[파생어 예시](https://www.korea.kr/briefing/policyBriefingView.do?newsId=156157539) ·
[절다](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=83396&nation=eng) ·
[잘하다·못하다의 띄어쓰기](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=306073)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
`82c5c34d` 실행 코드에서 원래 질의와
[기존 보정 60건](2026-11-15-respectful-rio.md)에 이번 10건을 추가한 질의를 비교했다.

| 입력 | 경계 | Raw 행렬 / 비율 (%) | 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 새 보정 10건 | Smart·Any | 0, 0, 0, 10 / 0.00, 0.00, 0.00 | 10, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 사전 4,458건 | Smart | 4371, 0, 0, 87 / 100.00, 98.05, 99.01 | 4429, 0, 0, 29 / 100.00, 99.35, 99.67 |
| 사전 4,458건 | Any | 4262, 0, 0, 196 / 100.00, 95.60, 97.75 | 4323, 0, 0, 135 / 100.00, 96.97, 98.46 |

보정한 10건만 전체 범위 일치로 바뀌었다. 평가 주석 변경으로 성능 측정을 생략했다.

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-predicate-lemma-review.mjs "$rio_reviewed" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$raw" "$raw_output"
"$runner" boundary full-pos "$boundary" "$adjusted" "$adjusted_output"
```

입력 SHA-256은 `eea0253d969e5af109b945aa6f95776da44c550cfe9e4ea9cd8a45aa2c529fe2`,
출력은 `4af55d3b080fc129afa8b1dbd9f0f8592bcebde344e4c35fc06a752d6e03e00b`다.
