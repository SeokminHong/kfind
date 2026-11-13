# GSD 문장 표기와 주석 교정

고정 GSD 양성 입력의 고유 문장 989개를 읽고 맞춤법·문법 교정 후보를 확인했습니다.
원문의 의도적인 축약과 구어체는 유지합니다. `내비 키고`에서는 기기를 작동한다는 뜻의
`켜다` 활용만 교정했습니다.
[국립국어원 어문 규범](https://www.korean.go.kr/kornorms/m/m_regltn.do?regltn_code=0001)

| 입력 | SHA-256 |
| --- | --- |
| 이전 교정 | `d34735e79fad865a62685522859f2664e56d9f37330a1a11d17614aab82cda1e` |
| 문장·주석 교정 후보 | `e561b8522b58fd625805186a4f868e2078019fb1cfb12c9bd2e7a7f66dbed132` |
| 문장·주석 교정 및 계약 검토 | `5491013e680698c880a2b5667edb1fb9220cc95a47d4b1c6920e3286a4e738bf` |

같은 full POS `smart` 실행 파일과 리소스로 UTF-8 정확 일치를 평가했습니다.
행렬은 TP, FP, TN, FN 순서, 비율은 precision, recall, F1 순서입니다.
이 중간 후보의 두 입력은 계약 검토 0건으로 raw와 계약 보정 수치가 같습니다.

| 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 이전 교정 9,335건 | 8809, 0, 0, 526 | 100.00, 94.37, 97.10 | 8809, 0, 0, 526 | 100.00, 94.37, 97.10 |
| 문장·주석 교정 후보 9,334건 | 8821, 0, 0, 513 | 100.00, 94.50, 97.17 | 8821, 0, 0, 513 | 100.00, 94.50, 97.17 |
| 문장·주석 교정 및 계약 검토 | 8821, 0, 0, 513 | 100.00, 94.50, 97.17 | 8768, 0, 0, 511 | 100.00, 94.49, 97.17 |

`한결 같아요`의 기존 두 주석을 `한결같아요`의 한 주석으로 합쳤습니다.
교정 후 부분 일치는 326건, 겹치는 반환이 없는 사례는 187건입니다.

누락된 원문을 추정해야 하는 3개 문장(989개 중 0.30%)에 속한 질의 55건
(9,334건 중 0.59%)은 `nonstandard-input`으로 계약 평가에서 제외했습니다.
계약 검토 55건은 모두 제외이고 재분류 0건입니다. 계약 평가 분모는 9,279건입니다.
원문과 raw 결과는 보존했습니다.

| 문장 ID | 질의 | 제외 사유 |
| --- | ---: | --- |
| `test-s188` | 29 | 작품명 부호와 작품 관계를 연결하는 구문 누락 |
| `test-s398` | 7 | 주어와 술어의 관계가 성립하지 않아 뜻을 확정하기 어려움 |
| `test-s661` | 19 | 가타카나 인용 표기 누락 |

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$previous_input" --review-contract > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$corrected_output"
```

[문장 교정 근거](../../tools/morph-compare/gsd-sentence-edits.tsv) ·
[교정 주석](../../tools/morph-compare/gsd-sentence-edit-annotations.tsv) ·
[계약 제외 근거와 대조 예문](../../tools/morph-compare/gsd-nonstandard-sentences.tsv) ·
[이전 평가와 실행 환경](2026-11-13-gsd-test-s501-spelling.md)
