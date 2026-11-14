# 병나다와 부정 지정사 명사형의 정답 범위

고정 사전에서 `병나/병났/병날/병난`으로 시작하는 2건과
`-ㅁ` 용례 4건을 대조했다. `병날라`와 `병나겠네`는 사전에
등재된 동사 `병나다`의 활용이므로 전체 어절을 표적으로 유지했다.
`수사 결과, 김 씨가 범인이 아님이 밝혀졌다`에서는 명사형 어미와
주격 조사가 붙은 `아님이`를 `아니다` 질의의 표적으로 삼았다.
[한국어기초사전 병나다](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=59012) ·
[한국어기초사전 -ㅁ](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=78527)

## 품질

행렬은 TP, FP, TN, FN 순서다. 비율은 precision, recall, F1 순서다.
`005c9169`의 실행 코드에서 원래 입력과
[기존 보정 73건](2026-11-15-copula-honorific-adnominal.md)에 3건을
더한 입력을 비교했다. 추가 보정 3건에서만 반환 범위가 바뀌었다.

| 입력 | 경계 | Raw 행렬 / 비율 (%) | 보정 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 보정 대상 3건 | Smart | 0, 0, 0, 3 / 0.00, 0.00, 0.00 | 3, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 보정 대상 3건 | Any | 1, 0, 0, 2 / 100.00, 33.33, 50.00 | 3, 0, 0, 0 / 100.00, 100.00, 100.00 |
| 사전 4,458건 | Smart | 4372, 0, 0, 86 / 100.00, 98.07, 99.03 | 4436, 0, 0, 22 / 100.00, 99.51, 99.75 |
| 사전 4,458건 | Any | 4263, 0, 0, 195 / 100.00, 95.63, 97.76 | 4329, 0, 0, 129 / 100.00, 97.11, 98.53 |

평가 주석 변경으로 성능 측정을 생략했다.

## 재현

```sh
node tools/nikl-lexicon/apply-dictionary-nada-nominal-review.mjs "$honorific_reviewed" > "$adjusted"
"$runner" boundary full-pos "$boundary" "$raw" "$raw_output"
"$runner" boundary full-pos "$boundary" "$adjusted" "$adjusted_output"
```

입력 SHA-256은 `6cf0042bcc8ca58e6729c40e67306023fded20039742357711d0d4c619c84e31`,
출력은 `23a523049cc090b3dd6394dcbde7c2cd3bbec2c02cecfcd850f4d2866767c34d`다.
