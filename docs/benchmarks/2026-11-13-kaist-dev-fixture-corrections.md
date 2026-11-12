# KAIST dev 표기와 주석 교정

제품 리비전은 `937cf238`입니다. KAIST dev 질의 행렬 2,532건의 원본과
교정 진단 입력을 같은 실행기로 채점했습니다.

| KAIST 문장 ID | 교정 |
| --- | --- |
| `M2TA_069-s25` | `착하다보니 → 착하다 보니`, `착하다`와 `보다`의 범위를 분리 |
| `MH2_0169-s386` | `안나오면 → 안 나오면`, 동사 `안나다 → 나오다` |
| `M2TA_089-s62` | `삼십년을 → 삼십 년을`, 수사 `삼십`과 단위 `년`의 범위를 분리 |
| `MH2_0169-s738` | `1,500여년 간 → 1,500여 년간`, 명사 `년`의 범위와 접미사 `-간`의 품사를 정정 |
| `MH2_0069-s112` | `수천가지가 → 수천 가지가`, 수사 `수천`의 범위를 정정 |
| `MH2_0159-s26` | `문학 비평에는 격식 있는 글 읽기가 요구된다.`, `있다`·`비평`·`읽다`의 범위를 정정 |
| `MH2_0069-s34` | 허용 표기 `첫번째`에서 `첫`을 관형사로 정정 |
| `MH2_0169-s640`, `MH2_0209-s100`, `MH2_0169-s259` | `그 혼란`, `저 밑바닥`, `이 주장`의 첫 어절을 관형사로 정정 |
| `MH2_0159-s86` | `적잖은`을 동사 `적다`에서 형용사 `적잖다`로 정정 |
| `MH2_0069-s271` | `주 가치`의 `주`를 관형사로 정정 |

[UD 회귀](../../data/fixtures/ud_morphology_cases.tsv) ·
[국립국어원 ‘하다 보니’](https://korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=6469&mn_id=217&pageIndex=1) ·
[국립국어원 ‘40여 년’](https://www.korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=9217&mn_id=217&pageIndex=2) ·
[국립국어원 ‘첫번째’](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=332762) ·
[한국어기초사전 ‘첫’](https://krdict.korean.go.kr/m/eng/searchResultView?ParaWordNo=77610&nation=eng)

교정 진단 입력은 문장 ID별로 텍스트를 교체하고, 바뀐 표적의 질의·품사·UTF-8 범위와
뒤에 오는 범위를 수정했습니다. 원본과 같은 질의 ID와 음성 질의를 보존했습니다.
문장·품사가 달라진 원본 계약 검토 5건은 교정 입력에서 제거했습니다. 접미사 `-간`은
교정한 품사 기준 음성, 제품 계약의 단위 내부 성분 기준 양성으로 채점했습니다.
원본과 교정본의 차이는 입력·주석 교정 효과입니다.
[원본 계약 검토](../../tools/morph-compare/query-matrix-contract-reviews.tsv) ·
[단위 내부 성분 계약](../../specs/matching.md)

양성과 범위가 지정된 음성은 표적과의 겹침으로 채점했습니다. 표적이 없는 음성은
반환 여부로 채점했습니다. 행렬 순서는 TP, FP, TN, FN이고 비율 순서는 precision,
recall, F1입니다.

| 입력 | 모드 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) | 검토 |
| --- | --- | --- | --- | --- | --- | ---: |
| 원본 | smart | 1245, 0, 1266, 21 | 100.00, 98.34, 99.16 | 1244, 0, 1269, 17 | 100.00, 98.65, 99.32 | 5 |
| 교정 | smart | 1255, 1, 1266, 10 | 99.92, 99.21, 99.56 | 1256, 0, 1266, 10 | 100.00, 99.21, 99.60 | 1 |
| 원본 | any | 1257, 20, 1246, 9 | 98.43, 99.29, 98.86 | 1252, 23, 1246, 9 | 98.20, 99.29, 98.74 | 5 |
| 교정 | any | 1258, 21, 1246, 7 | 98.36, 99.45, 98.90 | 1259, 20, 1246, 7 | 98.44, 99.45, 98.94 | 1 |

```sh
cargo build --release --locked --manifest-path tools/morph-compare/runner/Cargo.toml
"$runner" boundary full-pos "$boundary" "$input" "$output"
```

| 입력 | SHA-256 |
| --- | --- |
| KAIST dev 원본 | `cc6b67f87fde082bbf6d55de7f20576da824088c2f841a37c58df73ee84f79e7` |
| KAIST dev 교정 진단 | `b9f4e75c10dd436389e8e05633c8cd09d6e145a7eafd4bf590d0d016af929bd9` |

제품 코드와 리소스가 같아 성능은 측정하지 않았습니다.
[공통 리소스 checksum](2026-11-13-exclamative-endings.md#성능)
