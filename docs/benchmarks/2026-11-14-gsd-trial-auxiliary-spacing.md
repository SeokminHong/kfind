# GSD 보조 용언 보다의 띄어쓰기

교정 GSD 9,298행에서 `보` 형태와 보조 용언 주석이 함께 있는
15행을 검토했다. 세 음절인 파생 본용언에 보조 용언이 붙은
`기대해봄직하다`와 `전화해보니`에 공백을 넣었다. 본용언 질의의
정답 범위를 각각 `기대해`와 `전화해`로 교정하고 뒤 표적의
UTF-8 오프셋을 다시 계산했다.
[한글 맞춤법 제47항](https://korean.go.kr/kornorms/m/m_regltn.do?regltn_code=0001) ·
[원문 교정](../../tools/morph-compare/gsd-trial-auxiliary-spacing-edits.tsv) ·
[교정 주석](../../tools/morph-compare/gsd-trial-auxiliary-spacing-annotations.tsv)

418f04d6과 검색 코드가 같은 full POS 실행 파일을 사용했다.
양성 정확 범위 기준이며 행렬은 TP, FP, TN, FN 순서다.
계약 검토는 0건이다. 검색 코드 변경이 없어 성능 측정을 생략했다.

| 경계 | 입력 | Raw 행렬 | Raw P/R/F1 (%) | 계약 보정 행렬 | 보정 P/R/F1 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9295, 0, 0, 3 | 100.00, 99.97, 99.98 | 9295, 0, 0, 3 | 100.00, 99.97, 99.98 |
| smart | 교정 후 | 9296, 0, 0, 2 | 100.00, 99.98, 99.99 | 9296, 0, 0, 2 | 100.00, 99.98, 99.99 |
| any | 교정 전 | 9083, 0, 0, 215 | 100.00, 97.69, 98.83 | 9083, 0, 0, 215 | 100.00, 97.69, 98.83 |
| any | 교정 후 | 9084, 0, 0, 214 | 100.00, 97.70, 98.84 | 9084, 0, 0, 214 | 100.00, 97.70, 98.84 |

#421 비교군 1,111행에서 교정 후 `smart`는 정확 1,109행,
부분 0행, 누락 2행이며 `any`는 정확 969행, 부분 137행,
누락 5행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --trial-auxiliary-spacing > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 전 SHA-256은 `ccda1da94c85419e3a4f160a14f2de50c8935556f6d9cdcab9e1e30786a10dc1`,
교정 후는 `03dcf54bdc8abcf5a79a5526ef0da77596edb9f59992df4b1e14e84794fe4c03`이다.
