# GSD 어휘 분절과 조사 범위 교정

남은 명사 질의의 표제어, 원문 토큰, 형태 주석과 반환 범위를 비교했습니다.
`NNG+VV`, `NNG+VA`, `NNG+ETM`으로 시작하는 명사 질의 7행을
모두 확인했습니다. 이 중 `협심해서`, `세묜`, `주관사는`, `로켓포탄`,
`사먹습니다` 5행의 분절과 품사를 고쳤습니다. `도움받았다고`의 기존 분석과
`맛있긴`의 명사 질의는 이번 교정의 평가에 그대로 포함했습니다.

다른 명사 불일치에서는 음식명 `구이`, 야구 장비 `안전구`, 여격 조사
`에게`가 붙은 `사람들에게`를 추가로 교정했습니다. `사먹습니다`는 두
본용언을 띄어 쓰고 기존 한 행을 `사다`의 연결형 `사`로 정렬했습니다.
행 수는 9,314개로 같습니다. [한국어기초사전 협심하다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=90618) ·
[한국어기초사전 구이](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=35747) ·
[예술의전당 지휘자 표기](https://www.sac.or.kr/site/main/show/show_view?SN=49258) ·
[교정 목록](../../tools/morph-compare/gsd-missegmented-lexemes-annotations.tsv)

동일한 `b21595a649826f4bda47da2adae68f2b9b13aabc` full POS 바이너리로
재평가했습니다. 행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall,
F1입니다. 계약 검토는 양쪽 모두 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9158, 0, 0, 156 | 100.00, 98.33, 99.16 | 9158, 0, 0, 156 | 100.00, 98.33, 99.16 |
| smart | 교정 | 9166, 0, 0, 148 | 100.00, 98.41, 99.20 | 9166, 0, 0, 148 | 100.00, 98.41, 99.20 |
| any | 기준선 | 8974, 0, 0, 340 | 100.00, 96.35, 98.14 | 8974, 0, 0, 340 | 100.00, 96.35, 98.14 |
| any | 교정 | 8982, 0, 0, 332 | 100.00, 96.44, 98.19 | 8982, 0, 0, 332 | 100.00, 96.44, 98.19 |

#421 불일치 1,113행 중 교정 후 `smart` 정확 일치는 978행, 부분 일치는
61행, 누락은 74행입니다. `any`에서는 각각 865행, 229행, 19행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --missegmented-lexemes > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `6184892a41099170a8a1085ec1b619f96765a60cdf64cef20b00540864626b8e`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
