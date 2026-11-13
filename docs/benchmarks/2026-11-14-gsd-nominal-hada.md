# GSD 하다 활용형과 동형인 명사 주석 교정

GSD의 동사 질의 중 표제어가 `하다`로 끝나고 표면형이 `하는`,
`하고`, `함`으로 끝나는 119행을 문맥과 대조했습니다. 함명
`천안함` 2행, 명사와 보조사가 결합한 `은하는`과 `나하는`,
비교 조사가 결합한 `예전하고`를 교정했습니다. `공격하고`,
`공식을 마련하는` 등은 동사로 유지했습니다. 원문과 바이트
범위는 보존했습니다.
[한국어기초사전 은하](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=71816) ·
[한국어기초사전 조사 하고](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=78635) ·
[교정 목록](../../tools/morph-compare/gsd-nominal-hada-annotations.tsv)

동일한 `e78327229c00466f50de6b527a10c431f6db9cd9` full POS
바이너리로 9,311행을 평가했습니다. `예전하고`는 두 경계에서
`예전`까지만 반환해 부분 일치입니다. 나머지 교정 4행은 정확히
일치했습니다. 행렬 순서는 TP, FP, TN, FN이며 비율은 precision,
recall, F1입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9226, 0, 0, 85 | 100.00, 99.09, 99.54 | 9226, 0, 0, 85 | 100.00, 99.09, 99.54 |
| smart | 교정 | 9225, 0, 0, 86 | 100.00, 99.08, 99.54 | 9225, 0, 0, 86 | 100.00, 99.08, 99.54 |
| any | 기준선 | 9033, 0, 0, 278 | 100.00, 97.01, 98.48 | 9033, 0, 0, 278 | 100.00, 97.01, 98.48 |
| any | 교정 | 9032, 0, 0, 279 | 100.00, 97.00, 98.48 | 9032, 0, 0, 279 | 100.00, 97.00, 98.48 |

#421의 불일치 1,113행의 판정은 두 경계 모두 동일합니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --nominal-hada > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `a45e484712d290120c639911809d2ddba426ea41452df7b9590c55ad74283ade`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
