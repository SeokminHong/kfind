# GSD 복합 고유명사 주석

GSD 양성 9,314건에서 `NNP+NNG`로 시작하는 44건을 전수 검토했습니다.
`인천공항`, `운서역`, `한국도예협회` 등 이름 전체가 하나의 장소나 단체를
가리키는 28건의 질의를 해당 이름으로 정렬했습니다. `민사과에`의 `민사과`는
법원 부서를 가리키는 일반명사로 교정했습니다. 원문과 UTF-8 정답 범위는
동일합니다. [사례별 주석](../../tools/morph-compare/gsd-compound-name-review.tsv) ·
[철도 명칭과 역명](https://encykorea.aks.ac.kr/Article/E0081076) ·
[한국도예협회 명칭](https://www.ikfa.or.kr/data/pdf/world/world_03.pdf) ·
[대한팔경 용례](https://encykorea.aks.ac.kr/Article/E0024209)

나머지 15건에서는 `중국시장`의 `중국`, `이명박정권은`의 `이명박`처럼
앞말이 뒷말을 수식하거나 `현대미술은`처럼 일반명사가 고유명사로 주석된 경우를
확인했습니다. 수식어와 조사 범위, 품사, 띄어쓰기를 문장별로 판정할 대상으로
남겼습니다.

기준선과 교정본에 같은 full POS 실행 파일 `8581bac664494819874fd78381ac0030879e71c5`를
사용했습니다. 행렬은 TP, FP, TN, FN 순이며 비율은 precision, recall, F1입니다.
계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9043, 0, 0, 271 | 100.00, 97.09, 98.52 | 9043, 0, 0, 271 | 100.00, 97.09, 98.52 |
| smart | 교정 | 9072, 0, 0, 242 | 100.00, 97.40, 98.68 | 9072, 0, 0, 242 | 100.00, 97.40, 98.68 |
| any | 기준선 | 8884, 0, 0, 430 | 100.00, 95.38, 97.64 | 8884, 0, 0, 430 | 100.00, 95.38, 97.64 |
| any | 교정 | 8913, 0, 0, 401 | 100.00, 95.69, 97.80 | 8913, 0, 0, 401 | 100.00, 95.69, 97.80 |

두 경계에서 교정한 29건이 모두 정확히 일치했고 나머지 9,285건의 반환 범위는
동일했습니다. 제품 검색 코드가 동일해 성능 측정을 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-proper-name-review.mjs "$input" --compound-names > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `1d8380f3c6c6f892fab50f510b0013ad8db55ea1105901ac24f15508510c4bb2`입니다.
[이전 입력 생성과 실행 환경](2026-11-13-gsd-numeral-compound-review.md)
