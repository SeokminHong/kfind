# 고정 GSD 입력 맞춤법과 문법 재점검

UD Korean-GSD test 원문 989개와 고정 양성 입력의 문장 ID 989개를 대조했습니다.
GSD 출처가 표시된 회귀 fixture의 227행(고유 텍스트 194개)도 따로 읽었습니다.
맞춤법·문법이 분명한 129개 문장에 교정을 적용했습니다. 질의 1,577건의 본문,
840건의 UTF-8 범위와 71건의 형태 주석이 바뀌었습니다. 회귀 fixture 7행을
교정했고, 전체 양성 질의 ID와 9,332건의 분모를 유지했습니다.

조사·의존 명사·문장 부호는 [국립국어원 어문 규범](https://korean.go.kr/kornorms/m/m_regltn.do?regltn_code=0001)에
따랐습니다. `쓰여진`은 [피동사 용례](https://korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=6154&mn_id=217&pageIndex=1)에
따라 `쓰인`으로, `다음날인 6일`은 [이튿날을 나타내는 띄어쓰기](https://korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=311632)에
따라 `다음 날인 6일`로 고쳤습니다. `<7광구`처럼 문장 안에서 경계가 확인되는
제목은 닫는 부호를 복원했습니다.

`넘넘`, `맘`, `내비`처럼 의도적으로 줄인 말과 문체는 보존했습니다. 어문 규범에서
별도로 정하지 않은 백분율 기호의 띄어쓰기, 허용된 보조 용언·전문 용어 표기,
그리고 `test-s174`의 한자 자리처럼 누락된 내용을 이 입력만으로 확정할 수 없는
표기도 유지했습니다. 문장 0/989개, 질의 0/9,332건을 계약 평가에서 제외했습니다.

같은 full POS `smart` 실행 파일과 사전으로 UTF-8 정확 일치를 평가했습니다. 행렬은
TP, FP, TN, FN 순서, 비율은 precision, recall, F1 순서입니다. 양성 입력에 한정된
평가이므로 precision은 별도의 오탐 검증을 뜻하지 않습니다. 계약 검토는 두 입력 모두
0건이며 raw와 계약 보정 수치가 같습니다.

| 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) | 부분 일치 | 겹치는 반환 없음 |
| --- | --- | --- | --- | --- | ---: | ---: |
| 복원 문장 교정 | 8822, 0, 0, 510 | 100.00, 94.53, 97.19 | 8822, 0, 0, 510 | 100.00, 94.53, 97.19 | 324 | 186 |
| 전체 재점검 교정 | 8832, 0, 0, 500 | 100.00, 94.64, 97.25 | 8832, 0, 0, 500 | 100.00, 94.64, 97.25 | 299 | 201 |

질의 ID로 대응시켰을 때 부분 일치 28건과 겹치는 반환이 없던 14건이 정확 일치로
바뀌었고, 정확 일치 32건이 부분 일치 또는 겹치는 반환 없음으로 바뀌었습니다.
이는 동일한 실행 파일에서 문장과 정답 범위를 고쳐 얻은 차이입니다.
`verify-gold`에서는 기존 불일치 15건이 동일하게 재현됐습니다.

| 항목 | SHA-256 |
| --- | --- |
| UD 원문 | `3d1df99bda4800235e14bcfd915baf706eafa1a3935a75ffd32420a51e57f5aa` |
| 복원 문장 교정 | `b29c500d8f0ebd1795b1ddf27975cfcd40c97b2f14ad12e9e8e89ee4f71ee0dc` |
| 전체 재점검 교정 | `0ff79e6d0a67dd778017c7425bf27bd0a71f0ade30b8e12613635464067f8d98` |
| 검색 실행 파일 | `f5d1dd409e4f8dea88b249e5ac987deed61a87195dd52a40c156460b134fa37a` |
| full POS lexicon | `47bb7a3a31d6985f0b31dab5632b0d0a07feefa7cd2d94f220ec5d5483ac735f` |
| compact component | `eb59a212200441a7b37554d334541332b6dfc386a77a513c0f63c9660ba9ec42` |
| enriched predicate | `67e99dc49c8d74d7584b370d1ca26e2e07e30ad5dce9391a7615734fadcc668e` |

복원 문장 교정의 저장소 기준은 `6719186cf8dbbed2d62363743f09347d33b5bf5b`입니다.
Node.js v26.11.1로 입력을 만들고, 두 입력에 같은 검색 실행 파일을 사용했습니다.
검색 코드에는 변경이 없어 실행 성능은 비교하지 않았습니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$previous_input" \
  --review-contract --align-shared-sentences --restore-source-passages > "$restored_input"
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$previous_input" \
  --review-contract --align-shared-sentences --restore-source-passages --proofread > "$proofread_input"
"$runner" boundary full-pos smart "$restored_input" "$restored_result"
"$runner" boundary full-pos smart "$proofread_input" "$proofread_result"
```

[문장별 교정 근거](../../tools/morph-compare/gsd-proofreading-edits.tsv) ·
[형태 주석 변경](../../tools/morph-compare/gsd-proofreading-annotations.tsv) ·
[복원 문장 교정](2026-11-13-gsd-sentence-review.md)
