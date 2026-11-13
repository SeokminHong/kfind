# GSD 용언의 표제어·품사 교정

GSD `smart`에서 정확히 일치하지 않은 동사·형용사 질의 34행을
문장과 형태 주석으로 대조했습니다. `난방식`, `내쉬와`는 각각 일반
명사와 인명·조사로, 두 `그래도`는 부사로 정렬했습니다. `분양하다`는
동사, `필요하다`는 형용사로 교정했습니다. `두말할`과
`불어넣었다는`은 어절당 하나의 용언 질의로 합쳤습니다.
`따라한`은 `따라 한`으로 띄어 쓰고 기존 질의를 `따르다`의
연결형에 정렬했습니다. 관련 주석 11행 중 2행은 앞 행에 합쳐졌습니다.
[국립국어원 따라 하다](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=335337) ·
[한국어기초사전 분양하다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=60376) ·
[교정 목록](../../tools/morph-compare/gsd-predicate-reanalysis-annotations.tsv)

동일한 `e78327229c00466f50de6b527a10c431f6db9cd9` full POS 바이너리로
교정 전 9,313행과 교정 후 9,311행을 평가했습니다. 변경된 9개 질의는
두 경계 모두 정확히 일치합니다. 행렬 순서는 TP, FP, TN, FN이며
비율은 precision, recall, F1입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9214, 0, 0, 99 | 100.00, 98.94, 99.47 | 9214, 0, 0, 99 | 100.00, 98.94, 99.47 |
| smart | 교정 | 9221, 0, 0, 90 | 100.00, 99.03, 99.51 | 9221, 0, 0, 90 | 100.00, 99.03, 99.51 |
| any | 기준선 | 9023, 0, 0, 290 | 100.00, 96.89, 98.42 | 9023, 0, 0, 290 | 100.00, 96.89, 98.42 |
| any | 교정 | 9028, 0, 0, 283 | 100.00, 96.96, 98.46 | 9028, 0, 0, 283 | 100.00, 96.96, 98.46 |

#421의 불일치 1,113행 중 교정 후 `smart`는 정확 1,033행,
부분 28행, 누락 52행입니다. `any`는 각각 913행, 185행, 15행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --predicate-reanalysis > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `354cec7eeb316c1c210caf58015c23371f81dedbbba6f75dad8e5ee53da66753`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
