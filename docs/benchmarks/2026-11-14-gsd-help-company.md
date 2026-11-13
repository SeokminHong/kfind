# GSD 도움의 띄어쓰기와 회사명 경계

체언 범위를 교정한 GSD 9,300행의 `smart` 부분 일치 3행을
검토했다. `도움받았다고`에서 일반 명사 `도움`과 동사 `받다`를
띄고, 조사 생략과 나머지 원문을 보존했다. 같은 조건의
`도움+받` 주석은 한 행이었다.
[국립국어원 띄어쓰기 답변](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=261&pageIndex=1&qna_seq=331893)

`간+의` 주석 2행을 검토했다. `모간키간의`의 `모간키간`은
케빈 기디스가 채권 거래를 담당한 회사 Morgan Keegan의 음역으로
판정했다. 회사명 전체를 고유 명사로 주석하고 관형격 조사 `의`를
유지했다. 기간을 나타내는 `달간의`의 주석은 유지했다.
[미국 증권거래위원회 자료](https://www.sec.gov/files/litigation/complaints/2009/comp21143.pdf) ·
[띄어쓰기 교정](../../tools/morph-compare/gsd-help-company-edits.tsv) ·
[교정 주석](../../tools/morph-compare/gsd-help-company-annotations.tsv)

7ca3b9a8의 동일한 full POS 러너로 교정 전후를 비교했다.
행렬은 TP, FP, TN, FN 순서다. 계약 검토는 0건이며 검색 실행
코드 변경이 없어 성능 측정을 생략했다.

| 경계 | 입력 | Raw 행렬 | Raw P/R/F1 (%) | 계약 보정 행렬 | 보정 P/R/F1 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 교정 전 | 9278, 0, 0, 22 | 100.00, 99.76, 99.88 | 9278, 0, 0, 22 | 100.00, 99.76, 99.88 |
| smart | 교정 후 | 9280, 0, 0, 20 | 100.00, 99.78, 99.89 | 9280, 0, 0, 20 | 100.00, 99.78, 99.89 |
| any | 교정 전 | 9075, 0, 0, 225 | 100.00, 97.58, 98.78 | 9075, 0, 0, 225 | 100.00, 97.58, 98.78 |
| any | 교정 후 | 9077, 0, 0, 223 | 100.00, 97.60, 98.79 | 9077, 0, 0, 223 | 100.00, 97.60, 98.79 |

#421 비교군 1,113행에서 교정 후 `smart`는 정확 1,093행,
부분 1행, 누락 19행이다. `any`는 정확 962행, 부분 144행,
누락 7행이다. `smart` 부분 일치는 `백석이의` 한 행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --help-company > "$corrected"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 전 SHA-256은 `cd25113ad12ef849cac7e549005cecf410588223cc306119ee626db78a68c8bb`,
교정 후는 `88b7817096118165f11f9640a4ef4c14f84e5b5ef3ed1e9f81754671c114cb96`다.
