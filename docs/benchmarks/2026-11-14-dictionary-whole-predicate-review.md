# 사전 예문의 잘하다·잘되다·잘생기다 표제어 보정

고정 사전 4,458건에서 표면형이 `잘하-`, `잘되-/잘됐-`, `잘생기-/잘생겼-`로
시작하는 19건을 전수 검토했다. 기존 표제어가 맞는 7건을 확인했고,
표제어가 `하다`, `되다`, `생기다`로 축소된 12건에 전체 표제어를 지정했다.
능숙·성공·외모의 어휘 의미와 붙여 쓴 원문, 사전 등재를 함께 대조했다.
[국립국어원 띄어쓰기 해설](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=44&pageIndex=1&qna_seq=335078) ·
[한국어기초사전 잘생기다](https://krdict.korean.go.kr/mon/dicSearch/SearchView?ParaWordNo=24881&nation=mon)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
Raw는 기존 고정 사전으로 채점하고 계약 보정은 선행 검토와 이 표제어
보정을 적용했다. 12건의 문장·원래 범위와 사전 출처를 보존했다.

| 입력 | 경계 | Raw 행렬 / 비율 (%) | 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 표제어 보정 대상 12건 | smart·any | 0, 0, 0, 12 / 0.00, 0.00, 0.00 | 12, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 전체 사전 4,458건 | smart | 4301, 0, 0, 157 / 100.00, 96.48, 98.21 | 4324, 0, 0, 134 / 100.00, 96.99, 98.47 |
| 전체 사전 4,458건 | any | 4168, 0, 0, 290 / 100.00, 93.49, 96.64 | 4192, 0, 0, 266 / 100.00, 94.03, 96.92 |

전체 보정은 이 12건과 [선행 보정 10건](2026-11-14-honorific-request-sipsa.md#품질),
[열거형의 지정사 범위 2건](2026-11-14-copula-enumerative-yo.md#품질)을 적용했다.
나머지 4,446건의 질의와 검색 결과는 동일했다.

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-whole-predicate-review.mjs "$raw" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$adjusted" "$output"
```

입력 SHA-256은 `747e7e60df2c779d0cd6b79c27c088e283d5e81bf613988832c9a1b8233bf884`,
출력은 `9d41ef2cac5673fed7e37c9a9ac851fcc0d779cf0d4c11913bd5e087121db235`다.
측정 빌드는 `919a0962`다.
