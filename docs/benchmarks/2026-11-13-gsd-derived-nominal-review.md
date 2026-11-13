# GSD 경기명과 분야명의 명사 주석

GSD 양성 9,314건에서 앞선 검토 이후 남은 `NNP+NNG` 주석 5건과
형태소 `전`이 있는 17건을 확인했습니다. 경기 의미의 접미사 `-전`이 붙은
`대만전`, `일본전`, `플레이오프전`은 전체 파생 명사를 질의합니다.
`사무전`은 작품 속 학교 이름이며, 시간 관계를 나타내는 `전` 13건은
대조군으로 유지했습니다. [국립국어원 접미사 풀이](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=88970&nation=eng) ·
[작품의 사무전 표기](https://tv.apple.com/kr/episode/%EB%B8%8C%EB%A5%98-%EC%9F%81%ED%83%88%EC%A0%84-%EA%B2%A9%EB%8F%8C-%EC%82%AC%EB%AC%B4%EC%A0%84-%EB%8C%80-%EC%95%84%EB%9D%BC%ED%81%AC%EB%85%B8%ED%8F%AC%EB%B9%84%EC%95%84/umc.cmc.572wra6i4nsrjw2z0s3fwd2f8)

`유럽여자프로골프`는 뒤따르는 대회를 공동 승인한 투어를 가리킵니다.
`대구광역시교육감상`은 상 이름, `현대미술`은 미술 분야를 가리킵니다.
7건의 질의와 주석을 교정하고 원문 표기를 보존했습니다.
[교정 목록](../../tools/morph-compare/gsd-derived-nominal-annotations.tsv) ·
[대회 주최 측 안내](https://www.ladiesmasters.com/latest-press.php) ·
[대구광역시교육청 시상 안내](https://www.dge.go.kr/dme/na/ntt/selectNttInfo.do?mi=7637&nttSn=2218001) ·
[국립현대미술관의 현대미술 용례](https://www.mmca.go.kr/upload/m/data/artinfo/2011/12/27/20111227111103.pdf)

기준선과 교정본에 같은 full POS 실행 파일 `8581bac664494819874fd78381ac0030879e71c5`를
사용했습니다. 행렬은 TP, FP, TN, FN 순이며 비율은 precision, recall, F1입니다.
계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9082, 0, 0, 232 | 100.00, 97.51, 98.74 | 9082, 0, 0, 232 | 100.00, 97.51, 98.74 |
| smart | 교정 | 9087, 0, 0, 227 | 100.00, 97.56, 98.77 | 9087, 0, 0, 227 | 100.00, 97.56, 98.77 |
| any | 기준선 | 8923, 0, 0, 391 | 100.00, 95.80, 97.86 | 8923, 0, 0, 391 | 100.00, 95.80, 97.86 |
| any | 교정 | 8928, 0, 0, 386 | 100.00, 95.86, 97.88 | 8928, 0, 0, 386 | 100.00, 95.86, 97.88 |

두 경계에서 교정한 7건이 모두 정확히 일치했고 나머지 9,307건의 반환 범위는
동일했습니다. 같은 제품 검색 코드를 사용해 성능 측정을 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --derived-nominals > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `e09e933e865128f1f2bfd937a5d192e26d5250967800a82e4e786108c5554ff3`입니다.
[이전 입력 생성과 실행 환경](2026-11-13-gsd-nominal-modifier-review.md)
