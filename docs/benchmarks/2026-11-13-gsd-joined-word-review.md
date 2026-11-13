# GSD 단일 어휘의 분리 주석 교정

교정 문장에 붙여 쓴 `이곳`, `그때`, `올여름` 등을 원래 두 질의로
유지한 16개 위치를 한 표제어의 질의로 합쳤습니다. 한 단어의 뜻으로 쓰인 문맥과
연속 UTF-8 범위를 확인했습니다. 조사까지 원래 두 번째 주석에 있으면 새 범위에
포함했습니다. 원문 추출본과 앞 단계 교정본은 보존했습니다.
[위치별 판정](../../tools/morph-compare/gsd-joined-word-review.tsv) ·
[국립국어원 바른 국어 생활](https://www.korean.go.kr/common/download.do?c_file_name=5c1ecd30-6519-4f83-bd36-dd215e699601_0.pdf&file_path=reportData&o_file_name=%EB%B0%94%EB%A5%B8+%EA%B5%AD%EC%96%B4+%EC%83%9D%ED%99%9C.pdf)

동일한 full POS `smart` 실행 파일로 정확한 UTF-8 범위를 비교했습니다. 행렬은
TP, FP, TN, FN 순서이며 비율은 precision, recall, F1입니다. 계약 검토는 양쪽 모두
0건입니다.

| GSD 양성 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- |
| 이전 교정 9,332건 | 8963, 0, 0, 369 | 100.00, 96.05, 97.98 | 8963, 0, 0, 369 | 100.00, 96.05, 97.98 |
| 단일 어휘 주석 교정 9,316건 | 8964, 0, 0, 352 | 100.00, 96.22, 98.07 | 8964, 0, 0, 352 | 100.00, 96.22, 98.07 |

이전 32개 질의는 정확 15건과 겹침 없음 17건이었고, 새 16개 질의는 모두
정확 일치했습니다. 나머지 9,300건의 반환 범위는 같았습니다. 교정 후 부분 일치는
175건, 겹치는 반환이 없는 질의는 177건입니다. 검색 실행 파일이 같아 기존 고정
입력의 성능 비교를 사용합니다.

```sh
node tools/morph-compare/apply-gsd-joined-word-review.mjs "$name_review_input" > "$corrected_input"
"$runner" boundary full-pos smart "$corrected_input" "$output"
```

교정 입력 SHA-256은 `16efe1b8b9eb19be9bf81bfa7a505362fe77fa6337653db88574ccf45d7abcf1`입니다.
[이전 교정 입력](2026-11-13-gsd-proper-name-review.md)
