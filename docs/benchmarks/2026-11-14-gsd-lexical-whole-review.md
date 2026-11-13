# GSD의 고유 명사와 차용어 범위

#421의 불일치 1,113행을 최신 GSD 입력에 연결해 남은 불일치에서
어휘가 무관한 성분으로 분리된 사례를 검토했습니다. 사람·시설·상표
이름과 차용어, 음운 용어 17행을 교정했습니다. 같은 문장에 두 번 나오는
`챌린지`는 두 위치 모두 명사로 맞췄습니다. 원문을 보존하고
정답의 UTF-8 범위를 교정한 어휘와 붙은 조사에 맞췄습니다.
[`챌린지` 프로그램](https://kr.newsroom.ibm.com/announcements?item=122393) ·
[`시크릿`이 포함된 밴드 이름](https://music.apple.com/us/artist/tabithas-secret/129678792) ·
[`파운드리` 용어](https://semiconductor.samsung.com/kr/support/tools-resources/dictionary/semiconductor-glossary-foundry/) ·
[교정 목록](../../tools/morph-compare/gsd-lexical-whole-annotations.tsv)

기준선 `96a1bd7b`의 동일한 full POS 실행 파일로 기준선과 교정 입력을
비교했습니다. 행렬은 TP, FP, TN, FN,
비율은 precision, recall, F1 순서입니다. 계약 검토는 0건입니다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9116, 0, 0, 198 | 100.00, 97.87, 98.93 | 9116, 0, 0, 198 | 100.00, 97.87, 98.93 |
| smart | 교정 | 9133, 0, 0, 181 | 100.00, 98.06, 99.02 | 9133, 0, 0, 181 | 100.00, 98.06, 99.02 |
| any | 기준선 | 8938, 0, 0, 376 | 100.00, 95.96, 97.94 | 8938, 0, 0, 376 | 100.00, 95.96, 97.94 |
| any | 교정 | 8954, 0, 0, 360 | 100.00, 96.13, 98.03 | 8954, 0, 0, 360 | 100.00, 96.13, 98.03 |

교정한 17행 밖의 반환 범위는 두 경계에서 같습니다. `any`는
`안수현이다`에서 지정사의 `이`까지 반환합니다. #421의 불일치
1,113행에서 교정 후 `smart` 정확 일치는 946행, 부분 일치는 78행,
겹치는 반환 없음은 89행입니다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --lexical-whole > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `247faf42cb30ea97665ed7e7495b2b4402f0721825545490723c96a7a84ea7fd`입니다.
검색 코드 변경이 없어 성능 측정을 생략했습니다.
