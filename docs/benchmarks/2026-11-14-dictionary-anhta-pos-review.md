# 사전 예문의 보조 용언 않다 품사 검토

고정 사전에서 `않다`를 질의한 55건을 전수 검토했다. `-지 않다`에서
바로 앞 용언이 나타내는 상태를 부정하면 보조 형용사, 동작을 부정하면
보조 동사로 분석했다. `싶지 않다`에서는 앞선 동작보다 `싶다`가
`않다`에 직접 연결된다. 원문의 질의 품사를 확인한 뒤 15건에서
계약 보정의 품사를 형용사로 지정했다. 동작을 부정하는
`책을 읽지 않으면서`와 `학교에 가지 않는다지` 등은 동사로 유지했다.
[국립국어원 않다의 품사](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=73&pageIndex=1&qna_seq=334688) ·
[국립국어원 보조 용언의 직접 결합](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=73&pageIndex=1&qna_seq=326432)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
앞선 보정과 신규 품사 보정을 모두 적용한 값을 원래 고정 판정과 비교했다.
다른 4,443건의 검색 범위와 15건의 원문·표적 범위는 동일했다.
`않으니라`는 사전 예문의 앞 용언 품사와 해당 어미의 결합 설명을
추가 대조해야 한다. `않을레라`는 동사 질의에서 누락으로 남았다.

| 대상 | 경계 | Raw 행렬 / 비율 (%) | 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 품사 보정 대상 15건 | smart·any | 9, 0, 0, 6 / 100.00, 60.00, 75.00 | 15, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 전체 사전 4,458건 | smart | 4301, 0, 0, 157 / 100.00, 96.48, 98.21 | 4330, 0, 0, 128 / 100.00, 97.13, 98.54 |
| 전체 사전 4,458건 | any | 4168, 0, 0, 290 / 100.00, 93.49, 96.64 | 4198, 0, 0, 260 / 100.00, 94.17, 97.00 |

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-whole-predicate-review.mjs "$raw" > "$whole"
node tools/nikl-lexicon/apply-dictionary-anhta-pos-review.mjs "$whole" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$adjusted" "$output"
```

[선행 표제어 보정](2026-11-14-dictionary-whole-predicate-review.md#재현) 뒤의
입력 SHA-256은 `9d41ef2cac5673fed7e37c9a9ac851fcc0d779cf0d4c11913bd5e087121db235`,
출력은 `dd2ca7f70d6f369aa67edfe0d6d5437e4500436548c92294836780c1c9d6eccc`다.
측정 런타임은 `9967f0fa`다.
