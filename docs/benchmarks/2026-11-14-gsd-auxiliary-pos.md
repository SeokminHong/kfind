# GSD 보조 용언의 품사와 연결 어미 교정

GSD 평가 입력에서 첫 형태 주석이 `VX`인 행 중 `싶다` 8행,
`않다` 36행, `하다` 42행, `못하다` 7행, `보다` 3행의 문맥을
대조했습니다. `싶다` 8행, 형용사를 부정하는 `않다` 10행,
형용사 또는 서술격 조사를 강조하는 `하다` 2행을 형용사로
교정했습니다. `먹어 본`, `맺지 못하도록`, `내긴 하지만`,
`사용하지 않은` 등은 동사로 유지했습니다.
[한국어기초사전 싶다](https://krdict.korean.go.kr/eng/dicMarinerSearch/search?mainSearchWord=%EC%8B%B6%EB%8B%A4) ·
[한국어기초사전 아니하다](https://krdict.korean.go.kr/eng/dicMarinerSearch/search?mainSearchWord=%EC%95%84%EB%8B%88) ·
[국립국어원 보조 형용사 하다](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=324942)

동사로 분류된 `-은데/-ㄴ데` 3행을 모두 확인했습니다. `싶은데`와
`한데`는 보조 형용사로 조회합니다. `차이가 많이 나지 않은데`는
앞말 `나다`를 부정하는 보조 동사이므로 `않는데`로 교정했습니다.
[국립국어원 연결 어미 -은데와 -는데](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=306033) ·
[원문 교정 목록](../../tools/morph-compare/gsd-auxiliary-pos-edits.tsv) ·
[주석 교정 목록](../../tools/morph-compare/gsd-auxiliary-pos-annotations.tsv)

동일한 `e78327229c00466f50de6b527a10c431f6db9cd9` full POS
바이너리로 9,311행을 평가했습니다. 교정한 주석 21행은 두 경계 모두
정확히 일치합니다. 행렬 순서는 TP, FP, TN, FN이며 비율은 precision,
recall, F1입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9221, 0, 0, 90 | 100.00, 99.03, 99.51 | 9221, 0, 0, 90 | 100.00, 99.03, 99.51 |
| smart | 교정 | 9224, 0, 0, 87 | 100.00, 99.07, 99.53 | 9224, 0, 0, 87 | 100.00, 99.07, 99.53 |
| any | 기준선 | 9028, 0, 0, 283 | 100.00, 96.96, 98.46 | 9028, 0, 0, 283 | 100.00, 96.96, 98.46 |
| any | 교정 | 9031, 0, 0, 280 | 100.00, 96.99, 98.47 | 9031, 0, 0, 280 | 100.00, 96.99, 98.47 |

#421의 불일치 1,113행 중 교정 후 `smart`는 정확 1,036행,
부분 26행, 누락 51행입니다. `any`는 각각 916행, 182행, 15행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --auxiliary-pos > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `6f7c07b7d8ff5a568fb9741ba0559c35cda2e111d6de13d1f3d525c53d89ee5f`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
