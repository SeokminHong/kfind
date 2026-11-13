# GSD 세다의 활용과 문화 면의 띄어쓰기

교정 GSD 9,300행에서 `쌔서`, `쎄서` 표기와 `+면+에서`,
`+면+에` 주석을 검색했다. 해당하는 두 문장을 검토했다.
`양념이 쌔서`는 `양념이 세서`로 교정하고 형용사 `세다`의
활용으로 주석했다. `정치, 경제, 사회, 문화면에서`에서는 열거의
마지막 항목 `문화`와 측면을 나타내는 명사 `면`을 띄었다.
나머지 원문은 보존하고 뒤 표적의 UTF-8 범위를 다시 계산했다.
[한국어기초사전 세다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=64132) ·
[국립국어원 맞춤법 교정 연구](https://www.korean.go.kr/common/download.do?c_file_name=46270824-a860-478a-96a9-11a5558940a5.pdf&file_path=reportData&o_file_name=2022%EB%85%84+%EB%A7%9E%EC%B6%A4%EB%B2%95+%EA%B5%90%EC%A0%95+%EB%A7%90%EB%AD%89%EC%B9%98+%EC%97%B0%EA%B5%AC+%EB%B6%84%EC%84%9D.pdf) ·
[원문과 교정](../../tools/morph-compare/gsd-nominal-spelling-edits.tsv) ·
[정답 주석](../../tools/morph-compare/gsd-nominal-spelling-annotations.tsv)

4da237a8의 동일한 full POS 러너로 교정 전후를 비교했다.
행렬은 TP, FP, TN, FN 순서이며 계약 검토는 0건이다.
검색 실행 코드 변경이 없어 성능 측정을 생략했다.

| 경계 | 입력 | Raw 행렬 | Raw P/R/F1 (%) | 계약 보정 행렬 | 보정 P/R/F1 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9287, 0, 0, 13 | 100.00, 99.86, 99.93 | 9287, 0, 0, 13 | 100.00, 99.86, 99.93 |
| smart | 교정 후 | 9289, 0, 0, 11 | 100.00, 99.88, 99.94 | 9289, 0, 0, 11 | 100.00, 99.88, 99.94 |
| any | 교정 전 | 9081, 0, 0, 219 | 100.00, 97.65, 98.81 | 9081, 0, 0, 219 | 100.00, 97.65, 98.81 |
| any | 교정 후 | 9082, 0, 0, 218 | 100.00, 97.66, 98.81 | 9082, 0, 0, 218 | 100.00, 97.66, 98.81 |

#421 비교군 1,113행에서 교정 후 `smart`는 정확 1,102행,
부분 0행, 누락 11행이다. `any`는 정확 967행, 부분 139행,
누락 7행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --nominal-spelling > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 전 SHA-256은 `31926841e4cd7b7bb456ffc56a3972b9b3d2c03e23c9e155771dbdb04a508032`,
교정 후는 `40b188ec41c9405a897b786da234f9394b010bf8a03067cf5d738af4d7bc4965`다.
