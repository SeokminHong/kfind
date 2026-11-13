# GSD 수량 표현과 위치 의문문의 주석

GSD 양성 9,314건에서 `키로/킬로`로 시작하는 주석 3건, 수 관형사와 순서를
나타내는 표현이 함께 나오는 문장 2건, 대명사 뒤 보조사로 주석된 `어디야` 2건을
검토했습니다. 두 `몇 키로야`는 `몇 킬로야`로 교정하고 거리 단위 `킬로`에
정답 범위를 맞췄습니다. `네 해째`의 `네`는 수 관형사로, 빵집과 주차장 위치를
묻는 `어디야`는 대명사와 지정사 종결형으로 판정했습니다.
[국립국어원 한국어기초사전의 거리 단위 킬로](https://krdict.korean.go.kr/m/eng/searchResultView?ParaWordNo=72175&nation=eng) ·
[문장 교정](../../tools/morph-compare/gsd-context-annotation-edits.tsv) ·
[주석과 범위](../../tools/morph-compare/gsd-context-annotations.tsv)

제품 코드 리비전은 `4492e75d71cb77cb43b37de3805145c12d99232c`입니다.
양쪽에 같은 full POS 실행 파일을 사용했습니다. 행렬은 TP, FP, TN, FN 순이며
비율은 precision, recall, F1입니다. 양성 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9003, 0, 0, 311 | 100.00, 96.66, 98.30 | 9003, 0, 0, 311 | 100.00, 96.66, 98.30 |
| smart | 교정 | 9008, 0, 0, 306 | 100.00, 96.71, 98.33 | 9008, 0, 0, 306 | 100.00, 96.71, 98.33 |
| any | 기준선 | 8848, 0, 0, 466 | 100.00, 95.00, 97.43 | 8848, 0, 0, 466 | 100.00, 95.00, 97.43 |
| any | 교정 | 8852, 0, 0, 462 | 100.00, 95.04, 97.46 | 8852, 0, 0, 462 | 100.00, 95.04, 97.46 |

일치 여부가 달라진 행은 교정한 주석에 한정됐습니다. `any`에서 `네`는 대명사
질의에도 같은 범위로 반환되어 교정 전후에 정확 일치로 집계됐습니다.
제품 코드가 같아 성능 비교는 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --context-annotations > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `b7a0726517a62c72866525ff882bab0c9dda293a705e9ae886e15bd334e2f617`입니다.
[이전 입력 생성](2026-11-13-gsd-pronoun-copula-span-review.md)
