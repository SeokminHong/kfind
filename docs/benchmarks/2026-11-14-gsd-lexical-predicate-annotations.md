# GSD의 합성 용언 주석

교정 GSD 9,306행에서 표면형이 `끝내`, `맛있`, `맛없`,
`멋있`, `멋없`으로 시작하는 45행의 문맥과 주석을 검토했다.
`끝내주네요`와 `끝내줍니다`는 맛에 대한 평가이므로 동사
`끝내주다`로, `맛있긴`은 형용사 `맛있다`로 교정했다.
41행은 이미 `맛있다`, 1행은 이미 `멋있다`로 주석되어 있었다.
문장, 표면형, UTF-8 정답 범위는 보존했다.
[끝내주다](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=38397) ·
[한글 맞춤법](https://www.korean.go.kr/kornorms/m/m_regltn.do?regltn_code=0002) ·
[교정 주석](../../tools/morph-compare/gsd-lexical-predicate-annotations.tsv)

검색 실행 파일은 938c01c7 소스에서 빌드했다. 양성 검토는
0건이며 계약 보정 값은 raw와 같다. 행렬은 TP, FP, TN, FN
순서이며 비율은 precision, recall, F1이다.

| full POS 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9246, 0, 0, 60 | 100.00, 99.36, 99.68 | 9246, 0, 0, 60 | 100.00, 99.36, 99.68 |
| smart | 교정 후 | 9248, 0, 0, 58 | 100.00, 99.38, 99.69 | 9248, 0, 0, 58 | 100.00, 99.38, 99.69 |
| any | 교정 전 | 9047, 0, 0, 259 | 100.00, 97.22, 98.59 | 9047, 0, 0, 259 | 100.00, 97.22, 98.59 |
| any | 교정 후 | 9049, 0, 0, 257 | 100.00, 97.24, 98.60 | 9049, 0, 0, 257 | 100.00, 97.24, 98.60 |

#421 비교군 1,113행에서 교정 후 `smart`는 정확 1,060행,
부분 19행, 누락 34행이다. `any`는 정확 935행, 부분 165행,
누락 13행이며 교정한 `맛있긴` 1행이 누락에 포함된다.
검색 코드 변경이 없어 성능 측정을 생략했다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --lexical-predicates > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

입력 SHA-256은 `da4d9c6691f3cbd8e9eccaaddbe0ebc061cfca843cedd656ef9ddfa70865e3a7`,
교정 SHA-256은 `206289e8e002a81ad1fb460c321a8e14e7f1b4668f6684abe434f881aa947217`이다.
