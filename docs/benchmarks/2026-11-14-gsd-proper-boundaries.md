# GSD 고유명과 행정 단위 주석

GSD `smart`의 남은 명사 불일치에서 `NNP`가 포함된 20행을 모두
검토했습니다. 같은 어휘가 등장하는 행을 추가로 대조해 총 15행의
표제어·품사·범위를 교정했습니다. 일본 옛 지명 `지쿠고·지쿠젠·부젠`과
`영인면`은 문맥이 지시하는 지명으로, 시설·정당·기업은 원문에서
해당 개체를 가리키는 이름 전체로 정렬했습니다. `북측`은 일반 명사
전체로, 행정 단위 `주·구` 세 행은 일반 명사로 교정했습니다.
고유명 표기는 원문을 따랐습니다. `백석이의`의 호칭과 `이슬람권에서는`의
파생 범위는 별도 검토 대상으로 남았습니다.
[은평구민체육센터 시설 안내](https://sports.seoul.go.kr/main/facilities/facilities_view.do?ft_idx=1139) ·
[서울시 정부중앙청사 표기](https://opengov.seoul.go.kr/og/com/download.php?dname=%EA%B2%B0%EC%9E%AC%EB%AC%B8%EC%84%9C%EB%B3%B8%EB%AC%B8.pdf&uri=%2Ffiles%2Fdcdata%2F100001%2F20201221%2FF0000081881063.pdf) ·
[교정 목록](../../tools/morph-compare/gsd-proper-boundaries-annotations.tsv)

동일한 `b21595a649826f4bda47da2adae68f2b9b13aabc` full POS 바이너리로
9,314행을 재평가했습니다. 두 경계 모두 정확 일치가 11행 늘었습니다.
행정 단위 `주·구` 세 행은 `any`에서 정확히 일치하며 `smart`에서
여전히 누락됩니다. 행렬은 TP, FP, TN, FN 순서이고 비율은 precision,
recall, F1입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9166, 0, 0, 148 | 100.00, 98.41, 99.20 | 9166, 0, 0, 148 | 100.00, 98.41, 99.20 |
| smart | 교정 | 9177, 0, 0, 137 | 100.00, 98.53, 99.26 | 9177, 0, 0, 137 | 100.00, 98.53, 99.26 |
| any | 기준선 | 8982, 0, 0, 332 | 100.00, 96.44, 98.19 | 8982, 0, 0, 332 | 100.00, 96.44, 98.19 |
| any | 교정 | 8993, 0, 0, 321 | 100.00, 96.55, 98.25 | 8993, 0, 0, 321 | 100.00, 96.55, 98.25 |

#421 불일치 1,113행 중 교정 후 `smart`는 정확 일치 989행, 부분
일치 53행, 누락 71행입니다. `any`에서는 각각 876행, 219행, 18행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --proper-boundaries > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `05de72d018f39d9a03e2787f95f36c2fcf052e0d82313e8bf7c1714135e6f39d`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
