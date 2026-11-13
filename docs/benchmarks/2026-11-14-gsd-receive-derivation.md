# GSD 피동 접미사 앞 명사 범위

GSD·PUD 원문에서 명사와 `받`이 같은 토큰에 분리 주석된 행을
검색했습니다. GSD 8행을 확인했고 PUD의 대응 주석은 0행이었습니다.
문장에서 피동으로 쓰인 `지적·평가·구박·숭배·대접·사랑` 7행은
`받`의 품사를 동사에서 파생 접미사로 바꾸고, 명사 질의의 정답 끝을
접미사 앞으로 옮겼습니다. `구박받는` 한 문장이 두 ID에 있습니다.
`도움받았다고`는 접미사와 조사가 생략된 목적어·동사 분석을 구별할
근거가 충분하지 않아 기존 주석을 유지했습니다.
[국립국어원 피동 접미사 설명](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=332456) ·
[대접받다의 구성](https://korean.go.kr/front/onlineQna/onlineQnaView.do?qna_seq=326872) ·
[한국어기초사전 도움](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=64542) ·
[교정 목록](../../tools/morph-compare/gsd-receive-derivation-annotations.tsv)

같은 `b21595a649826f4bda47da2adae68f2b9b13aabc` full POS 검색
바이너리로 교정 전후를 평가했습니다. 9,314행에서 일곱 행의 품사와
정답 범위만 바뀌었으며 전체 반환 범위는 동일합니다. 행렬은 TP, FP, TN,
FN 순서이고 비율은 precision, recall, F1입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9151, 0, 0, 163 | 100.00, 98.25, 99.12 | 9151, 0, 0, 163 | 100.00, 98.25, 99.12 |
| smart | 교정 | 9158, 0, 0, 156 | 100.00, 98.33, 99.16 | 9158, 0, 0, 156 | 100.00, 98.33, 99.16 |
| any | 기준선 | 8967, 0, 0, 347 | 100.00, 96.27, 98.10 | 8967, 0, 0, 347 | 100.00, 96.27, 98.10 |
| any | 교정 | 8974, 0, 0, 340 | 100.00, 96.35, 98.14 | 8974, 0, 0, 340 | 100.00, 96.35, 98.14 |

#421 불일치 1,113행에서 교정 후 `smart`는 정확 971행, 부분 65행,
누락 77행입니다. `any`는 정확 858행, 부분 236행, 누락 19행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --receive-derivation > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `2766c3fc291cab4c945c4d57fa3ed94c117a2df0d78d90612158249c77459985`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
