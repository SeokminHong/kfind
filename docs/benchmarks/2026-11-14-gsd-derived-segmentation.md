# GSD 용언의 어절 내부 분절과 품사 교정

같은 어절 안에서 명사와 동사로 나뉜 GSD 주석을 인접 행의
UTF-8 범위로 찾아 5쌍을 모두 대조했다. `업그레이드된`,
`교육되었겠지`, `거부당했네요`는 피동 접미사가 붙은 동사,
`듯했는데`와 `재미있는`은 형용사 질의 하나로 합쳤다.
`하고` 조사 주석 2행에서는 비교를 나타내는 `예전하고`를
유지하고, `고혹적인`과 병렬로 `매력`을 꾸미는
`럭셔리하고`를 형용사로 교정했다. 원문은 보존했다.
[국립국어원 피동 접미사](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=336219) ·
[한국어기초사전 거부당하다 용례](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=88911) ·
[한국어기초사전 재미있다](https://krdict.korean.go.kr/m/eng/searchResult?mainSearchWord=%EC%9E%AC%EB%AF%B8%EC%9E%88%EB%8B%A4&nation=eng) ·
[국립국어원 외래어 결합과 접미사 하다](https://korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=327362) ·
[교정 목록](../../tools/morph-compare/gsd-derived-segmentation-annotations.tsv)

동일한 `b840213a1f870330d84363b0a3b332abf6960640` full POS
바이너리에서 교정한 6행의 전체 범위를 두 경계 모두 반환했다.
행렬은 TP, FP, TN, FN 순서이며, 비율은 precision, recall, F1
순서다. 계약 검토는 0건이다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9227, 0, 0, 84 | 100.00, 99.10, 99.55 | 9227, 0, 0, 84 | 100.00, 99.10, 99.55 |
| smart | 교정 | 9226, 0, 0, 80 | 100.00, 99.14, 99.57 | 9226, 0, 0, 80 | 100.00, 99.14, 99.57 |
| any | 기준선 | 9034, 0, 0, 277 | 100.00, 97.03, 98.49 | 9034, 0, 0, 277 | 100.00, 97.03, 98.49 |
| any | 교정 | 9029, 0, 0, 277 | 100.00, 97.02, 98.49 | 9029, 0, 0, 277 | 100.00, 97.02, 98.49 |

#421의 불일치 1,113행 중 교정 후 `smart`는 정확 1,039행,
부분 25행, 누락 49행이다. `any`는 각각 919행, 179행, 15행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --derived-segmentation > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `f181be59898bb2729e4549acf04279d98ba27e90389551762c686de7201eb096`이다.
검색 코드 변경이 없어 성능 측정을 생략했다.
