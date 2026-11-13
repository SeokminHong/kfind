# GSD 익명 표현의 품사 주석

장소 띄어쓰기를 교정한 GSD 9,300행에서 표면형이 `모`인
2행을 검토했다. `이 모 씨`는 성 뒤에서 이름을 대신하는
대명사로, `모 경제지`는 뒤의 명사를 꾸미는 관형사로 교정했다.
원문과 UTF-8 범위는 유지했다.
[국립국어원 품사 설명](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=27&pageIndex=1&qna_seq=335953) ·
[교정 주석](../../tools/morph-compare/gsd-anonymous-mo-annotations.tsv)

같은 full POS 실행 파일을 사용했다. 계약 검토는 0건이다.
행렬은 TP, FP, TN, FN, 비율은 precision, recall, F1 순서다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9259, 0, 0, 41 | 100.00, 99.56, 99.78 | 9259, 0, 0, 41 | 100.00, 99.56, 99.78 |
| smart | 교정 후 | 9260, 0, 0, 40 | 100.00, 99.57, 99.78 | 9260, 0, 0, 40 | 100.00, 99.57, 99.78 |
| any | 양쪽 | 9063, 0, 0, 237 | 100.00, 97.45, 98.71 | 9063, 0, 0, 237 | 100.00, 97.45, 98.71 |

#421 비교군 1,113행에서 교정 후 `smart`는 정확 1,073행,
부분 15행, 누락 25행이다. `any`는 정확 948행, 부분 158행,
누락 7행이다. 검색 코드 변경이 없어 성능 측정을 생략했다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --anonymous-mo > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

입력 SHA-256은 `776e8c66af108cfef3aad202ed57455ca3736c26c76e15592dedf4929ddff416`,
교정 SHA-256은 `569a6f9f9d66793d920603fe872ddf64df0be409430d72a42758430c1e99fe8a`다.
