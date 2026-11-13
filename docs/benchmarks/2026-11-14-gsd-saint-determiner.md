# GSD 성인 이름 앞 관형사 주석

교정 GSD 9,298행에서 표면형 또는 표제어가 `성`인 7행을
확인했다. `성 파트리치오`의 `성`은 성인 이름 앞에 쓰는
관형사이므로 `NNG`를 `MM`으로 바꿨다. 문장과 UTF-8 범위는
같다. [한국어기초사전](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=63034) ·
[교정 주석](../../tools/morph-compare/gsd-saint-determiner-annotations.tsv)

45a3b236의 full POS 실행 파일로 두 입력을 비교했다. 행렬 순서는
TP, FP, TN, FN이며 양성의 정확 범위를 채점했다. 계약 검토는
0건이다. 검색 코드 변경이 없어 성능 측정을 생략했다.

| 경계 | 입력 | Raw 행렬 | Raw P/R/F1 (%) | 계약 보정 행렬 | 보정 P/R/F1 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9296, 0, 0, 2 | 100.00, 99.98, 99.99 | 9296, 0, 0, 2 | 100.00, 99.98, 99.99 |
| smart | 교정 후 | 9297, 0, 0, 1 | 100.00, 99.99, 99.99 | 9297, 0, 0, 1 | 100.00, 99.99, 99.99 |
| any | 양쪽 | 9084, 0, 0, 214 | 100.00, 97.70, 98.84 | 9084, 0, 0, 214 | 100.00, 97.70, 98.84 |

#421 비교군 1,111행의 `smart` 정확 일치는 1,109행에서 1,110행으로
늘었다. 남은 누락은 `화보 속`의 `속`이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --saint-determiner > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 전 SHA-256은 `03dcf54bdc8abcf5a79a5526ef0da77596edb9f59992df4b1e14e84794fe4c03`,
교정 후는 `1040ed9371fa5ca3a223dfacba83f07765c99b901b41a675701d851a8c130c21`이다.
