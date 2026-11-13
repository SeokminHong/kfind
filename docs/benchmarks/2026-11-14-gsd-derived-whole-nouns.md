# GSD 파생 명사와 건수의 주석

교정 GSD 9,300행에서 `여+`, `총+`, `극+`, `건+`으로
시작하는 형태소 주석 4행을 검토했다. `여회사원`과
`총동창회장`의 첫 형태소는 접두사이며 `극빈곤층`의 `극`은
정도를 더하는 어근이다. `건수는`은 사건의 수를 뜻하는 사전
표제어 `건수`와 조사 `는`으로 분석했다. 네 행은 어절 안의
첫 음절을 독립 명사로 질의하던 주석을 명사 전체의 질의로
교정했다. 원문과 UTF-8 정답 범위는 보존했다.
[한국어기초사전 여-](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=72505) ·
[경기도교육청 공문서 작성 자료](https://www.goe.go.kr/resource/old/BBSMSTR_000000030179/BBS_202501161040039850.pdf) ·
[국립국어원 건수 답변](https://korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=&pageIndex=1&qna_seq=316076) ·
[교정 주석](../../tools/morph-compare/gsd-derived-whole-nouns-annotations.tsv)

52e91fe6의 동일한 full POS 러너로 교정 전후를 비교했다.
행렬은 TP, FP, TN, FN 순서다. 계약 검토는 0건이다.
검색 실행 코드 변경이 없어 성능 측정을 생략했다.

| 경계 | 입력 | Raw 행렬 | Raw P/R/F1 (%) | 계약 보정 행렬 | 보정 P/R/F1 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9282, 0, 0, 18 | 100.00, 99.81, 99.90 | 9282, 0, 0, 18 | 100.00, 99.81, 99.90 |
| smart | 교정 후 | 9286, 0, 0, 14 | 100.00, 99.85, 99.92 | 9286, 0, 0, 14 | 100.00, 99.85, 99.92 |
| any | 교정 전 | 9077, 0, 0, 223 | 100.00, 97.60, 98.79 | 9077, 0, 0, 223 | 100.00, 97.60, 98.79 |
| any | 교정 후 | 9081, 0, 0, 219 | 100.00, 97.65, 98.81 | 9081, 0, 0, 219 | 100.00, 97.65, 98.81 |

#421 비교군 1,113행에서 교정 후 `smart`는 정확 1,099행,
부분 0행, 누락 14행이다. `any`는 정확 966행, 부분 140행,
누락 7행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --derived-whole-nouns > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 전 SHA-256은 `88b7817096118165f11f9640a4ef4c14f84e5b5ef3ed1e9f81754671c114cb96`,
교정 후는 `31926841e4cd7b7bb456ffc56a3972b9b3d2c03e23c9e155771dbdb04a508032`다.
