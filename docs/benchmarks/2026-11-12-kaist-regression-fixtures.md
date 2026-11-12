# KAIST 구현 대상 회귀와 맞춤법 교정

기준 실행기는 `23b2349ba189059596d1c72458bcee89eebd53be`입니다. KAIST test에서
구현 대상으로 검토한 양성 14건을 원문과 회귀 fixture의 교정문에 같은 full-POS
실행기로 평가했습니다. 교정문은 `개체호(個體戶)라고 불리는`, `1,800만 개`,
`16.6 %나`로 표기하고 표적 범위를 다시 계산했습니다.
[회귀 fixture](../../data/fixtures/ud_morphology_cases.tsv) ·
[원문 및 출처](../../data/fixtures/README.md) ·
[‘부르다’의 피동 표현](https://www.korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=6448&mn_id=159) ·
[괄호 뒤 조사](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=&pageIndex=1&qna_seq=313703) ·
[한국어 어문 규범](https://www.korean.go.kr/kornorms/m/m_regltn.do?regltn_code=0001)

양성은 예측 범위가 원문의 표적 어절 UTF-8 범위에 겹치면 일치로 셉니다.
원문은 14건 모두 strict 기대값을 유지한 계약 검토가 있으며, 교정문에는 같은
판정을 적용했습니다. 두 입력 모두 계약 보정값이 raw와 같습니다.

| 입력 | 모드 | Raw TP, FP, TN, FN | Raw precision, recall, F1 (%) | 계약 보정 TP, FP, TN, FN | 계약 보정 precision, recall, F1 (%) |
| --- | --- | --- | --- | --- | --- |
| 원문 | smart | 14, 0, 0, 0 | 100.00, 100.00, 100.00 | 14, 0, 0, 0 | 100.00, 100.00, 100.00 |
| 교정문 | smart | 14, 0, 0, 0 | 100.00, 100.00, 100.00 | 14, 0, 0, 0 | 100.00, 100.00, 100.00 |
| 원문 | any | 13, 0, 0, 1 | 100.00, 92.86, 96.30 | 13, 0, 0, 1 | 100.00, 92.86, 96.30 |
| 교정문 | any | 13, 0, 0, 1 | 100.00, 92.86, 96.30 | 13, 0, 0, 1 | 100.00, 92.86, 96.30 |

`any`의 FN 1건은 `걸까`에서 생략된 지정사입니다. 회귀에서는 `smart`의 정확한
반환 범위를 검사합니다.

원문 입력은 [계약 검토 목록](../../tools/morph-compare/query-matrix-contract-reviews.tsv)의
`explicit-pos/test/implementation-target` 14건을 KAIST test에서 읽어 구성합니다.
교정문은 같은 사례의 `text`, `gold_byte_start`, `gold_byte_end`를 회귀 fixture의 값으로
바꿉니다. 원문 입력 SHA-256은 `1b0c8e939187ed608b4acc2413d711a3a1e9383eacff004d4f9a08a572d7c238`,
교정 입력은 `8a231b575e29a8dabebef54b70880053007e763897f5ab42eb49cbf683c817ac`입니다.

```sh
cargo build --release --locked --manifest-path tools/morph-compare/runner/Cargo.toml
export KFIND_COMPONENT_RESOURCE="$resource/morphology-component-compact.kfc"
export KFIND_FULL_POS_LEXICON="$resource/lexicon.bin"
export KFIND_ENRICHED_PREDICATES="$resource/predicates.enriched.tsv"
"$runner" boundary full-pos "$boundary" "$input" "$output"
cargo run -p kfind-testkit --bin verify-gold -- "$KFIND_FULL_POS_LEXICON" "$KFIND_COMPONENT_RESOURCE"
```
