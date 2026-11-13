# GSD 하다 파생 형용사의 품사와 범위 교정

GSD에서 동사로 표시된 `-하다` 표제어 365종을 목록으로 뽑고
상태를 나타내는 용례를 문맥과 대조했습니다. `친절하다` 14행,
`불친절하다` 6행을 포함해 형용사 용례 39행을 교정했습니다.
`검증하다`, `맡기다`의 인용형과 `변하다`의 변화 용례 등은
동사로 유지했습니다. 품사만 바뀐 행은 원문과 바이트 범위를 보존했습니다.
[한국어기초사전 친절하다](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=72157) ·
[한국어기초사전 불친절하다](https://krdict.korean.go.kr/m/eng/searchResult?mainSearchWord=%EB%B6%88%EC%B9%9C%EC%A0%88%ED%95%98%EB%8B%A4&nation=eng&nationCode=6) ·
[국립국어원 성숙하다의 품사 추가](https://www.korean.go.kr/nkview/nklife/2018_4/28_4.pdf)

`데다가`가 포함된 문장은 한 건입니다. 소비 환경의 상태를 나타내는
`견조한` 뒤에 의존 명사 `데`를 띄어 쓰고 기대 범위를 `견조한`으로
바꿨습니다.
[국립국어원 견조하다 용례](https://kli.korean.go.kr/term/trgtWord/indexTrgtWord.do?trgtWordNo=2024322) ·
[국립국어원 데다가 띄어쓰기](https://korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=308194) ·
[원문 교정 목록](../../tools/morph-compare/gsd-derived-predicate-pos-edits.tsv) ·
[주석 교정 목록](../../tools/morph-compare/gsd-derived-predicate-pos-annotations.tsv)

동일한 `e78327229c00466f50de6b527a10c431f6db9cd9` full POS 바이너리로
9,311행을 평가했습니다. 교정한 39행은 두 경계에서 모두 정확히
일치했습니다. 행렬 순서는 TP, FP, TN, FN이며 비율은 precision,
recall, F1입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9224, 0, 0, 87 | 100.00, 99.07, 99.53 | 9224, 0, 0, 87 | 100.00, 99.07, 99.53 |
| smart | 교정 | 9226, 0, 0, 85 | 100.00, 99.09, 99.54 | 9226, 0, 0, 85 | 100.00, 99.09, 99.54 |
| any | 기준선 | 9031, 0, 0, 280 | 100.00, 96.99, 98.47 | 9031, 0, 0, 280 | 100.00, 96.99, 98.47 |
| any | 교정 | 9033, 0, 0, 278 | 100.00, 97.01, 98.48 | 9033, 0, 0, 278 | 100.00, 97.01, 98.48 |

#421의 불일치 1,113행 중 교정 후 `smart`는 정확 1,038행,
부분 26행, 누락 49행입니다. `any`는 각각 918행, 180행, 15행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --derived-predicate-pos > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `e5f6483836dd29d8ebc03afc714dd20fd94547713a7a5015a004f1d0de5e9b0a`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
