# GSD 수사로 분리된 명사 주석

GSD 양성 9,314건에서 `NR/SN/MM` 바로 뒤에 `NNB/NNBC/NNG`가 붙은
주석을 검색했습니다. 해당하는 `일회용` 2건과 `일주일에` 1건의 질의를
사전에 등재된 명사 `일회용`과 `일주일`로 교정했습니다. 국립국어원 형태 분석
지침은 `일회용`을 `일회/NNG + 용/XSN`으로 분석하며, 검색 질의는 파생 명사
전체를 대상으로 합니다. `일주일에`의 부사격 조사는 보존했습니다.
같은 `일`이 독립 수사인 대조군 1건, 원문, UTF-8 정답 범위는 동일합니다.
[사례별 주석](../../tools/morph-compare/gsd-numeral-compound-review.tsv) ·
[국립국어원 형태 분석 지침](https://korean.go.kr/common/download.do?c_file_name=313036ed-dca8-473e-9ab9-e0bd7bf07565_0.pdf&file_path=reportData&o_file_name=%EC%B5%9C%EC%A2%85+%EB%B3%B4%EA%B3%A0%EC%84%9C_%ED%98%95%ED%83%9C+%EB%B6%84%EC%84%9D+%EB%A7%90%EB%AD%89%EC%B9%98+%EA%B5%AC%EC%B6%95.pdf#page=44) ·
[국립국어원 일주일 표제어 안내](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=314450)

기준선과 교정본에 같은 full POS 실행 파일 `8581bac664494819874fd78381ac0030879e71c5`를
사용했습니다. 행렬은 TP, FP, TN, FN 순이며 비율은 precision, recall, F1입니다.
계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9040, 0, 0, 274 | 100.00, 97.06, 98.51 | 9040, 0, 0, 274 | 100.00, 97.06, 98.51 |
| smart | 교정 | 9043, 0, 0, 271 | 100.00, 97.09, 98.52 | 9043, 0, 0, 271 | 100.00, 97.09, 98.52 |
| any | 기준선 | 8881, 0, 0, 433 | 100.00, 95.35, 97.62 | 8881, 0, 0, 433 | 100.00, 95.35, 97.62 |
| any | 교정 | 8884, 0, 0, 430 | 100.00, 95.38, 97.64 | 8884, 0, 0, 430 | 100.00, 95.38, 97.64 |

교정한 3건의 정확 일치가 회복됐고 나머지 9,311건의 반환 범위는 동일합니다.
제품 검색 코드가 동일해 성능 측정을 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-proper-name-review.mjs "$input" --numeral-compounds > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `7499b197f9f01116b4d8ffc6f70ceb2da723fa0daf348cc48c3b317eca4147eb`입니다.
[이전 입력 생성과 실행 환경](2026-11-13-gsd-numeral-copula-span-review.md)
