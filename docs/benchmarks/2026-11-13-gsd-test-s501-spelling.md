# GSD test-s501 명사형 표기 교정

GSD `test-s501`의 원문은 “선생님들 너무 친절하시고 가격도 괜찮고 무척 맘에 듬!”입니다.
`들다`의 어간 `들-`에 명사형 어미 `-ㅁ`이 붙으므로 `듦`으로 교정했습니다.
`맘`은 `마음`의 준말이므로 유지했습니다.
[원본 GSD](https://github.com/UniversalDependencies/UD_Korean-GSD/blob/02c343e4e1e3180069f637e68a791ec6b96dd33a/ko_gsd-ud-test.conllu) ·
[국립국어원 명사형 표기](https://www.korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=8703)

같은 문장을 사용하는 평가 질의 8건에 교정을 적용했습니다. 마지막 질의의 표제어·품사를
`듬/adverb`에서 `들다/verb`로, GSD 분석을 `듬/MAG`에서 `들+ㅁ/VV+ETN`으로
교정했습니다. 양쪽 표기가 각각 UTF-8 3바이트이므로 마지막 표적은 `[70, 73)`이고,
나머지 7건의 정답 범위도 동일합니다. `선생`과 `들다`의 정확 일치를 회귀 예문에 기록했습니다.
[회귀 예문](../../data/fixtures/ud_morphology_cases.tsv)

같은 `fda2a4b5` full POS `smart` 실행 파일, 사전, 구성 성분 리소스와 보강 용언 사전을
사용했습니다. 행렬은 TP, FP, TN, FN, 비율은 precision, recall, F1 순서입니다.
양쪽 입력에 계약 검토 0건이므로 raw와 계약 보정 수치가 같습니다.

| GSD 양성 9,335건 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 교정 전 | 8809, 0, 0, 526 | 100.00, 94.37, 97.10 | 8809, 0, 0, 526 | 100.00, 94.37, 97.10 |
| 교정 후 | 8809, 0, 0, 526 | 100.00, 94.37, 97.10 | 8809, 0, 0, 526 | 100.00, 94.37, 97.10 |

교정한 8건은 양쪽에서 정확 일치했고, 나머지 9,327건은 입력과 반환 범위가 같았습니다.
교정 후 부분 일치는 328건, 겹치는 반환이 없는 사례는 198건입니다.
`verify-gold`에서는 새 회귀 예문이 통과했고 기존 불일치 15건이 재현됐습니다.
제품 검색 코드는 동일하므로 성능 비교는 [기존 고정 입력과 측정](2026-11-13-compound-noun-annotation-spans.md)을 사용합니다.

```sh
node tools/morph-compare/apply-gsd-positive-corrections.mjs "$fixed_input" \
  --explicit-copula-noun-spans --fused-copula-noun-spans \
  --honorific-noun-spans --compound-noun-spans --reviewed-spelling > "$corrected_input"
"$runner" boundary full-pos smart "$previous_input" "$previous_output"
"$runner" boundary full-pos smart "$corrected_input" "$corrected_output"
```

| 입력 | SHA-256 |
| --- | --- |
| 고정 추출본 | `a6955363dbf837f16b5ec4ef6465eabf8e6611df43c591a21588aa638efde129` |
| 교정 전 | `b884984b52c3049c51dccf731436521c54a9f3b0746159fc621ad8bd7c7fdf24` |
| 교정 후 | `d34735e79fad865a62685522859f2664e56d9f37330a1a11d17614aab82cda1e` |

실행 환경은 Apple M1 Max, macOS 26.6.2, Rust 1.97.0, Node 26.11.1입니다.
실행 파일 SHA-256은 `f5d1dd409e4f8dea88b249e5ac987deed61a87195dd52a40c156460b134fa37a`입니다.
이 비교는 `test-s501` 교정만 다룹니다.
