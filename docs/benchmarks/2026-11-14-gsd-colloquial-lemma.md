# GSD 구어 표제어와 명사형 표기 교정

GSD에서 과거형 뒤의 `슴`과 `제`, `비추` 표기를 찾아 세 용례를
대조했다. 만리장성을 건축하는 뜻의 `지었제`는 `짓다`에
연결하고 구어 어미를 보존했다. 추천하지 않는다는 뜻의 `비추`는
`비추천`의 줄임말인 명사로 교정했다. `알았슴`은 과거형 뒤에
명사형 어미를 쓴 `알았음`으로 교정했다.
[한국어기초사전 짓다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=66585) ·
[국립국어원 비추의 형성](https://www.korean.go.kr/nkview/nklife/2013_1/23_01.pdf) ·
[원문 교정 목록](../../tools/morph-compare/gsd-colloquial-lemma-edits.tsv) ·
[주석 교정 목록](../../tools/morph-compare/gsd-colloquial-lemma-annotations.tsv)

`b840213a1f870330d84363b0a3b332abf6960640` full POS 바이너리로
9,306행을 평가했다. 두 경계에서 `비추`와 `알았음`이 정확히
일치했고 `지었제`는 `지었`까지만 반환됐다. 행렬은 TP, FP, TN,
FN 순서이며 비율은 precision, recall, F1 순서다. 계약 검토는 0건이다.

| 경계 | 입력 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- |
| smart | 기준선 | 9226, 0, 0, 80 | 100.00, 99.14, 99.57 | 9226, 0, 0, 80 | 100.00, 99.14, 99.57 |
| smart | 교정 | 9228, 0, 0, 78 | 100.00, 99.16, 99.58 | 9228, 0, 0, 78 | 100.00, 99.16, 99.58 |
| any | 기준선 | 9029, 0, 0, 277 | 100.00, 97.02, 98.49 | 9029, 0, 0, 277 | 100.00, 97.02, 98.49 |
| any | 교정 | 9031, 0, 0, 275 | 100.00, 97.04, 98.50 | 9031, 0, 0, 275 | 100.00, 97.04, 98.50 |

#421의 불일치 1,113행 중 교정 후 `smart`는 정확 1,041행,
부분 24행, 누락 48행이다. `any`는 각각 921행, 178행, 14행이다.

```sh
node tools/morph-compare/apply-gsd-sentence-edits.mjs "$input" --colloquial-lemma > "$corrected"
"$runner" boundary full-pos "$boundary" "$input" "$baseline"
"$runner" boundary full-pos "$boundary" "$corrected" "$output"
```

교정 입력 SHA-256은 `4e21b5c0df73957f9bd50a347e189c26e28e717eebe7355693a7962d191ae8e9`이다.
검색 코드 변경이 없어 성능 측정을 생략했다.
