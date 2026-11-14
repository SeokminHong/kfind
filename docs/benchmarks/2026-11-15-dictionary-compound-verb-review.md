# 사전 예문의 합성·파생 동사 표제어

고정 사전에서 `먹고살다`, `눈치채다`, `도와주다`, `말씀드리다`,
`그만두다`, `갈아입다`, `다녀오다`의 활용 표면을 조사했다.
같은 표면으로 시작하는 25건 중 13건에서 원래 표적이 한 어절 전체였지만
질의는 그 일부에만 정렬돼 있었다. 올바른 표제어가 이미 지정된 예문과
`병나다`처럼 띄어쓰기 검토가 필요한 예문은 그대로 사용했다.
[먹고살다](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=331631) ·
[눈치채다](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=92522) ·
[도와주다](https://stdict.korean.go.kr/search/searchView.do?word_no=84994&searchKeywordTo=3) ·
[말씀드리다](https://korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=332243) ·
[그만두다](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=62318)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
원래 입력, 기존 표제어·품사 보정 입력과 신규 보정 입력을 모두
`bf35d570`과 동일한 실행 코드에서 평가했다.

| 입력 | 경계 | Raw 행렬 / 비율 (%) | 표제어·품사 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 새 보정 대상 13건 | smart | 3, 0, 0, 10 / 100.00, 23.08, 37.50 | 13, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 새 보정 대상 13건 | any | 0, 0, 0, 13 / 0.00, 0.00, 0.00 | 13, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 사전 4,458건 | smart | 4364, 0, 0, 94 / 100.00, 97.89, 98.93 | 4392, 0, 0, 66 / 100.00, 98.52, 99.25 |
| 사전 4,458건 | any | 4249, 0, 0, 209 / 100.00, 95.31, 97.60 | 4280, 0, 0, 178 / 100.00, 96.01, 97.96 |

전체 사전의 보정 열은 아래 세 도구로 재현한 표제어·품사 보정 40건을
적용한다. 개별 어미 보고서에서 별도로 검토한 지정사 범위 등의 보정은
각 보고서의 품질 표에 기록돼 있으며 이 집계에는 포함되지 않는다.

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-whole-predicate-review.mjs "$raw" > "$whole"
node tools/nikl-lexicon/apply-dictionary-anhta-pos-review.mjs "$whole" > "$pos"
node tools/nikl-lexicon/apply-dictionary-compound-verb-review.mjs "$pos" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$raw" "$raw_output"
"$runner" boundary full-pos "$boundary" "$adjusted" "$adjusted_output"
```

입력 SHA-256은 `dd2ca7f70d6f369aa67edfe0d6d5437e4500436548c92294836780c1c9d6eccc`,
출력은 `caddc70312825f862c5f16c3c121a2aeb739999b399a9cd331090eabb2d7934d`다.
실행 리소스와 환경은 [높임 뒤 종결형 보고서](2026-11-15-honorific-final-ji.md#성능)와 같다.
