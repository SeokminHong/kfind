# GSD 교정 문장의 독립 재검토

[UD Korean-GSD 원문](https://github.com/UniversalDependencies/UD_Korean-GSD/blob/02c343e4e1e3180069f637e68a791ec6b96dd33a/ko_gsd-ud-test.conllu)과
기준 입력의 문장 989개를 ID로 대조했습니다. 앞선 교정으로 본문이 달라진 187개 문장은
전체를 읽고, GSD test 회귀 예문 199행의 표기도 대조했습니다. 43개 문장과 회귀
5행을 교정했습니다. 본문이 바뀐 질의는 649건이고, UTF-8 정답 범위가 바뀐 질의는
288건입니다. 허용된 보조 용언 표기와 `내비`, `많진` 등 구어와 준말은 보존했습니다.
[문장별 근거](../../tools/morph-compare/gsd-independent-proofreading-edits.tsv) ·
[형태 주석](../../tools/morph-compare/gsd-independent-proofreading-annotations.tsv)

- `test-s188`은 제목을 감싼 ASCII `< >`를 유지하고 `천재 수학자`를 띄어 썼습니다.
  [국립국어원 문장 부호 설명](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=27&pageIndex=1&qna_seq=328219) ·
  [연극 제작발표회 기사](https://m.playdb.co.kr/Artist/News/1738)
- `test-s398`의 원문은 “인간미 없는 사람의 음식은 음식에 느낌을 떨어뜨린다.”입니다.
  기존 교정의 `음식의 느낌을 떨어뜨린다`는 조사 교체 후 목적어와 서술어가 성립합니다.
  `떨어뜨린다`의 표기는 맞으며, 원문의 의미를 달리 확정할 출처를 찾지 못해 추가 변경하지
  않았습니다.
- `test-s661`에서 빠진 두 표기는
  [같은 문장이 있는 이타샤 문서의 어원 절](https://ko.wikipedia.org/w/index.php?title=%EC%9D%B4%ED%83%80%EC%83%A4&oldid=42011838#어원)에서 확인했습니다.
  두 표기를 각각 작은따옴표로 감쌌습니다. 뒤의 `'한심하다', '부끄럽다', '가엾다' 등의
  뜻으로 사용된다.`는 기존 공백·부호 교정이 맞습니다. 문장 전체를 평가에 포함했습니다.

`들어와 있음을`, `기울여 왔다`, `노력해 주길`의 본용언 질의는 띄어 쓴 보조 용언
앞에서 정답 범위를 닫았습니다. `미용사분`과 `간호사분들`의 `-분`은 높임 접미사이므로
독립 명사 질의 2건을 제거하고 직업명 질의는 유지했습니다. 이는 주석 교정으로,
문장·질의의 계약 제외와 구별합니다. 양쪽 모두 계약 검토는 0건입니다.

같은 full POS `smart` 실행 파일로 질의별 UTF-8 범위를 비교했습니다. 행렬은
TP, FP, TN, FN 순서이고 비율은 precision, recall, F1입니다. 양성 입력이므로
precision 수치는 별도 오탐 검증을 뜻하지 않습니다.

| 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) | 부분 일치 | 겹치는 반환 없음 |
| --- | --- | --- | --- | --- | ---: | ---: |
| 문맥 품사 교정 (9,316건) | 8969, 0, 0, 347 | 100.00, 96.28, 98.10 | 8969, 0, 0, 347 | 100.00, 96.28, 98.10 | 174 | 173 |
| 독립 재검토 (9,314건) | 8975, 0, 0, 339 | 100.00, 96.36, 98.15 | 8975, 0, 0, 339 | 100.00, 96.36, 98.15 | 169 | 170 |

공통 질의에서는 부분 일치 5건과 겹치는 반환이 없던 2건이 정확 일치로 바뀌었습니다.
제거한 두 질의는 이전에 정확 일치 1건, 겹치는 반환 없음 1건이었습니다.
`verify-gold`에서는 기준과 교정본에서 같은 기존 불일치 12건이 재현됐습니다.

`e77548d43132b391cdd00cb72bf9b3afaec05905`의 검색 코드로 빌드한 release 실행 파일을
사용했습니다. 환경은 Apple M1 Max, macOS 26.6.2, Rust 1.97.0, Node.js 26.11.1입니다.
검색 코드와 리소스가 같으므로 실행 성능은 비교하지 않았습니다.

| 항목 | SHA-256 |
| --- | --- |
| UD 원문 | `3d1df99bda4800235e14bcfd915baf706eafa1a3935a75ffd32420a51e57f5aa` |
| 문맥 품사 교정 | `120ad5000b84b3d61fef05214b6d703e0f573eb55330f0741cf455583806dce9` |
| 독립 재검토 | `f32a177315f52acff2c691341d527b9191590050b3c1dff28f57ca5fdf17e1ed` |
| 검색 실행 파일 | `8cd0d069a3215d8f87623017322c788e953fc0a269971c018a49a9fde65b53f5` |
| full POS lexicon | `47bb7a3a31d6985f0b31dab5632b0d0a07feefa7cd2d94f220ec5d5483ac735f` |
| compact component | `eb59a212200441a7b37554d334541332b6dfc386a77a513c0f63c9660ba9ec42` |
| enriched predicate | `67e99dc49c8d74d7584b370d1ca26e2e07e30ad5dce9391a7615734fadcc668e` |

```sh
cargo build --release --manifest-path tools/morph-compare/runner/Cargo.toml --locked
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$context_pos_input" \
  --independent-proofreading > "$corrected_input"
"$runner" boundary full-pos smart "$context_pos_input" "$baseline_result"
"$runner" boundary full-pos smart "$corrected_input" "$corrected_result"
```

[기준 입력](2026-11-13-gsd-context-pos-review.md) ·
[이전 문장 교정](2026-11-13-gsd-full-proofreading.md)
