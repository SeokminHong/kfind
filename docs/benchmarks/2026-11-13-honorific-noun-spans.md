# 높임 접미사 앞 명사 정답 범위

`fda2a4b5`의 같은 full POS `smart` 실행 파일로 GSD 입력 두 버전을
평가했습니다. GSD 주석과 원문에서 두 음절 이상인 일반 명사·고유 명사 뒤에
`-님`이 바로 결합하고, 복수 접미사 `-들`이나 조사가 이어지는 사례의
정답 끝을 높임 접미사 앞으로 옮겼습니다. 한 음절 어근과 중간에 다른 명사가
놓이는 합성어는 이번 교정 범위에서 제외했습니다.
[검색 사양](../../specs/matching.md) ·
[한국어기초사전 -님](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=88852&nation=eng) ·
[범위 회귀](../../data/fixtures/ud_morphology_cases.tsv)

행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall, F1 순서입니다.
양성은 UTF-8 시작과 끝 바이트의 일치로 채점했습니다. 계약 검토는 두 입력 모두
0건이며 raw와 계약 보정 수치는 같습니다.

| GSD 양성 9,335건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 이전 교정 | 8443, 0, 0, 892 | 100.00, 90.44, 94.98 | 8443, 0, 0, 892 | 100.00, 90.44, 94.98 |
| 높임 접미사 앞 범위 교정 | 8464, 0, 0, 871 | 100.00, 90.67, 95.11 | 8464, 0, 0, 871 | 100.00, 90.67, 95.11 |

정답 끝을 교정한 21건이 모두 정확 일치로 바뀌었습니다. 전체 반환 범위는
동일했습니다. 교정 후 부분 일치는 673건, 겹치는 반환이 없는 사례는 198건입니다.
`verify-gold`에는 기존 불일치 15건이 남았습니다.

```sh
node tools/morph-compare/apply-gsd-positive-corrections.mjs "$fixed_input" \
  --explicit-copula-noun-spans --fused-copula-noun-spans --honorific-noun-spans > "$noun_span_input"
```

교정 입력 SHA-256은 `969d43824e9627bb23627c29fadeecf302841ee593e147b02ce4d3745608d905`입니다.
제품 성능 비교에는 기존 고정 입력과 동일한 실행 파일을 사용합니다.
[이전 입력과 평가 방식](2026-11-13-fused-copula-noun-spans.md)
