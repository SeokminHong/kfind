# 기원 종결 어미 예문의 표제어

고정 사전의 `-아지이다/-어지이다/-여지이다` 용례 5건을 전수 검토했다.
사전의 결합 설명과 용례를 따라 `나아지이다`, `이루어지이다`,
`깊어지이다`의 질의를 각각 `낫다`, `이루다`, `깊다`로 보정했다.
일반 활용 `나아지려나`, `나아졌습디다`의 주석과 기원형 5건의
원문·정답 범위는 유지했다.
[표준국어대사전 -아지이다](https://stdict.korean.go.kr/search/contentViewOne.do?word_no=454940) ·
[표준국어대사전 -어지이다](https://stdict.korean.go.kr/search/contentViewOne.do?word_no=221967)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
`6a1f24b2` 실행 코드에서 원래 질의와
[기존 표제어·품사 보정 40건](2026-11-15-dictionary-compound-verb-review.md)에
이번 3건을 더한 질의를 비교했다.

| 입력 | 경계 | Raw 행렬 / 비율 (%) | 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 기원형 5건 | Smart·Any | 2, 0, 0, 3 / 100.00, 40.00, 57.14 | 5, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 사전 4,458건 | Smart | 4369, 0, 0, 89 / 100.00, 98.00, 98.99 | 4400, 0, 0, 58 / 100.00, 98.70, 99.35 |
| 사전 4,458건 | Any | 4257, 0, 0, 201 / 100.00, 95.49, 97.69 | 4291, 0, 0, 167 / 100.00, 96.25, 98.09 |

보정 대상 3건만 양쪽 경계에서 전체 범위 일치로 바뀌었다.
평가 주석 변경으로 성능 측정은 생략했다.

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-desiderative-review.mjs "$compound_reviewed" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$raw" "$raw_output"
"$runner" boundary full-pos "$boundary" "$adjusted" "$adjusted_output"
```

입력 SHA-256은 `caddc70312825f862c5f16c3c121a2aeb739999b399a9cd331090eabb2d7934d`,
출력은 `1c27f7dd2195d37d2e44d009b8e99215d4a2ee348c4c05283ff7e136d09acd59`다.
