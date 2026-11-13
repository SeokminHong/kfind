# GSD 문맥에 맞춘 표제어와 방송 제목

교정 GSD 양성 9,314행에서 `나름가수다`, `박칼린`, `묵은지`,
`맛나-`, `지어내-`의 출현과 기존 표제어가 겹치는 행을 대조했습니다.
`맛나네`는 형용사 `맛나다`, `지어내서`는 동사 `지어내다`의
활용이며 `묵은지라`와 `묵은지와`의 명사는 `묵은지`입니다.
인명 `박칼린`의 범위를 고쳤고, 두 문장의 방송 제목은 MBC 표기대로
`나름 가수다`로 띄어 썼습니다. 정답 주석 7행과 제목이 있는 문장 2개의
UTF-8 범위를 교정했습니다.
[MBC 방송 제목](https://playvod.imbc.com/vod/vodplay?broadcastid=1000786100361100000) ·
[국립국어원 `맛나다` 풀이](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=333966) ·
[`지어내다` 사전 문형](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=76850) ·
[교정 목록](../../tools/morph-compare/gsd-context-lexemes-annotations.tsv)

기준선 `7a0f9188`의 검색 코드와 동일한 실행 파일을 사용했습니다. 행렬은
TP, FP, TN, FN 순서이며 비율은 precision, recall, F1입니다. 계약 검토는
0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9135, 0, 0, 179 | 100.00, 98.08, 99.03 | 9135, 0, 0, 179 | 100.00, 98.08, 99.03 |
| smart | 교정 | 9140, 0, 0, 174 | 100.00, 98.13, 99.06 | 9140, 0, 0, 174 | 100.00, 98.13, 99.06 |
| any | 기준선 | 8956, 0, 0, 358 | 100.00, 96.16, 98.04 | 8956, 0, 0, 358 | 100.00, 96.16, 98.04 |
| any | 교정 | 8961, 0, 0, 353 | 100.00, 96.21, 98.07 | 8961, 0, 0, 353 | 100.00, 96.21, 98.07 |

`나름 가수다`는 두 경계에서 구의 두 원자 범위로 반환됩니다. 고정
평가의 단일 연속 범위 기준에서는 부분 일치 두 건으로 남습니다.
#421의 불일치 1,113행 중 `smart` 정확 일치는 953행, 부분 일치는
77행, 겹치는 반환 없음은 83행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --context-lexemes > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `03ef9ff20b40e0f6756a45dab2488e357ad814c08faf904d2954ebda2d24f006`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
