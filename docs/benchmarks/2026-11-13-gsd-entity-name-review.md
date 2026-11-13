# GSD 고유명사 주석

고정 GSD의 불일치 중 `NP` 또는 `NNP`로 시작하고 다른 명사 성분이 이어지는
50건을 찾아 문맥을 대조했습니다. `센트랄`처럼 고유명사에 지정사 주석이 붙은
경우도 확인했습니다. `우리산업`, `윌리엄스버그`, `김경주` 등 한 이름이
여러 성분으로 나뉜 사례를 포함해 12건의 질의와 주석을 교정했습니다.
지역명과 일반 명사가 결합한 `강남지역`, `중국시장` 등은 별도 판정 대상으로
남겼습니다. 문장과 UTF-8 정답 범위는 보존했습니다.
[사례별 주석](../../tools/morph-compare/gsd-entity-name-review.tsv) ·
[오슬로 센트랄 역](https://www.banenor.no/reise-og-trafikk/stasjoner/-o-/oslo-s/) ·
[Ustream 공식 서비스 자료](https://www.ibm.com/docs/en/announcement_archive/ENUS216-436/ENUS216-436.PDF)

`0bec5d43daf130f397c596ce52d5a0a9e6d37930`의 같은 full POS 실행 파일로
측정했습니다. 행렬은 TP, FP, TN, FN 순이며 비율은 precision, recall, F1입니다.
양성 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9010, 0, 0, 304 | 100.00, 96.74, 98.34 | 9010, 0, 0, 304 | 100.00, 96.74, 98.34 |
| smart | 교정 | 9022, 0, 0, 292 | 100.00, 96.86, 98.41 | 9022, 0, 0, 292 | 100.00, 96.86, 98.41 |
| any | 기준선 | 8852, 0, 0, 462 | 100.00, 95.04, 97.46 | 8852, 0, 0, 462 | 100.00, 95.04, 97.46 |
| any | 교정 | 8864, 0, 0, 450 | 100.00, 95.17, 97.52 | 8864, 0, 0, 450 | 100.00, 95.17, 97.52 |

정확 일치 여부는 교정한 12건에서만 달라졌습니다. 검색 코드가 같아 성능 측정을 생략했습니다.

```sh
node tools/morph-compare/apply-gsd-proper-name-review.mjs "$input" --entity-names > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `e9e89fc3fc5e0b385963e6eb346c2fb2db381660d1ccda35f51981afc5504eac`입니다.
[이전 입력 생성](2026-11-13-gsd-context-annotations.md)
