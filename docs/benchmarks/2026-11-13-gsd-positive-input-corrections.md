# GSD 양성 고정 입력의 표기와 범위 교정

`4bd750d838617b6074f3769cb05b271fb4b687e9`의 같은 full POS `smart` 실행 파일로
고정 입력과 교정 입력을 채점했습니다. [교정 목록](../../tools/morph-compare/gsd-positive-input-corrections.tsv)의
26개 질의는 원래 ID를 유지하고 기존 UD 회귀의 문장과 UTF-8 정답 범위를 사용합니다.
보조 용언의 띄어쓰기는 본용언이 세 음절 이상인 파생 용언 활용형인 경우와 `되나 봐요`처럼
`-나` 뒤에서 띄어 쓰는 경우를 교정했습니다. [국립국어원 제47항 해설](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=27&pageIndex=1&qna_seq=331002)

TP, FP, TN, FN 순서로 표시하며 비율은 precision, recall, F1입니다.
양성 질의의 시작과 끝 바이트가 모두 일치해야 TP입니다. 별도 계약 보정은 0건이며
각 입력의 raw와 계약 보정 수치는 같습니다.

| 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) | 검토 |
| --- | --- | --- | --- | --- | ---: |
| 고정 입력 9,335건 | 8225, 0, 0, 1110 | 100.00, 88.11, 93.68 | 8225, 0, 0, 1110 | 100.00, 88.11, 93.68 | 0 |
| 교정 입력 9,335건 | 8251, 0, 0, 1084 | 100.00, 88.39, 93.84 | 8251, 0, 0, 1084 | 100.00, 88.39, 93.84 | 0 |

교정한 26건은 모두 정확 일치했고 다른 9,309건의 반환 범위는 동일했습니다.
교정 입력의 불일치는 부분 일치 881건과 겹치는 반환이 없는 203건입니다.
이 차이는 입력 교정의 효과입니다.

```sh
node tools/morph-compare/apply-gsd-positive-corrections.mjs "$fixed_input" > "$reviewed_input"
KFIND_FULL_POS_LEXICON="$lexicon" KFIND_COMPONENT_RESOURCE="$components" \
KFIND_ENRICHED_PREDICATES="$enriched" \
  "$runner" boundary full-pos smart "$reviewed_input" "$output"
```

고정 입력 SHA-256은 `a6955363dbf837f16b5ec4ef6465eabf8e6611df43c591a21588aa638efde129`,
교정 입력은 `0109e590c91c8f0373181f41f35831dcd1aa3aacfae9358cfb2ed7fd5fba06dc`입니다.
벤치마크 입력 도구만 변경해 제품 성능 측정에는 기존 고정 입력과 실행 파일을 사용합니다.
[공통 사전 SHA-256](2026-11-13-corpus-rule-audit.md#성능)
