# PUD 동사 나다의 원문 범위

고정 음성에서 미검토 PUD 동사 `나다`가 Smart에 적중한 50행·51범위를
UD Korean-PUD 2.18의 `# text`와 CoNLL-U 토큰에 정렬했다.
그중 원문에 `나다`의 활용이 있는 세 행을 계약 양성으로 교정했다.
한국어기초사전의 감정·사건 발생 문형과 원문의 종결 위치 및 관형형의
수식 대상을 확인했다. 세 표적의 원문 표기는 그대로 보존했다.
[한국어기초사전 나다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=62210)

| PUD 원문 | 토큰 | 표적 | UTF-8 범위 |
| --- | ---: | --- | --- |
| `n01069023` | 14 | 화도 안 **나는군요** | 105..117 |
| `n01086016` | 13 | 짜증이 **날** 것이다 | 86..89 |
| `w01143015` | 14 | 파탄이 **난** 후 | 102..105 |

Smart, Any, Token에서 세 정답 범위를 모두 확인했다. Any는 첫 원문에서
`남자답게`의 `남`도 반환했다. Smart의 `p:나`는 `파탄이 난 후`에서
`난`을 반환했다. 대명사 대조문 `나는 아무 말 없이 자리를 비웠다`와
`난 먼저 돌아갈게`에서 각각 `p:나`와 함께 `v:나다`도 반환했다.
`아버지는 날 좋아하지 않아`의 `날`은 Smart에서 `p:나`만 반환했다.
[계약 검토](../../tools/morph-compare/pud-verb-label-corpus-reviews.tsv) ·
[대조 예문](../../tools/morph-compare/contract-contrast-evidence.jsonl)

## 품질

기준선 `9f76ccaf`의 고정 12세트에서 모드별 반환 249,243개에 이전·신규 계약을
각각 적용했다. 목표 11세트의 Smart FNc는 0이며 FPc는 2,281에서 2,278로
바뀌었다. 고정 음성 53,308행의 계약 검토는 1,319건에서 1,322건으로
늘었고 제외 64건은 동일하다. 아래 행렬은 TP, FP, TN, FN 순이며 비율은
precision, recall, F1 순이다.

| 모드 | 원본 행렬 / 비율 (%) | 이전 계약 행렬 / 비율 (%) | 신규 계약 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| Smart | 0, 3576, 49732, 0 / 0.00, —, 0.00 | 1255, 2278, 49711, 0 / 35.52, 100.00, 52.42 | 1258, 2275, 49711, 0 / 35.61, 100.00, 52.52 |
| Any | 0, 17616, 35692, 0 / 0.00, —, 0.00 | 1255, 16297, 35692, 0 / 7.15, 100.00, 13.35 | 1258, 16294, 35692, 0 / 7.17, 100.00, 13.38 |
| Token | 0, 2847, 50461, 0 / 0.00, —, 0.00 | 1069, 1778, 50211, 186 / 37.55, 85.18, 52.12 | 1072, 1775, 50211, 186 / 37.65, 85.21, 52.23 |

## 재현

`pud.conllu` SHA-256은
`55f9c8d465400e070664c5319091d31ce35a16ec220b0bebc5e3c9b12f530948`이다.
`negatives.jsonl` SHA-256은
`482e55d417516e8db503dbd421725fcc7affc6c6afcf5b5d42cec3bbd4d35d54`이다.
같은 입력 및 리소스에서 `smart`, `any`, `token`을 각각 실행하고
원문 SHA-256과 정답 범위를 위 계약 검토 기록에 대조한다.

```sh
"$runner" boundary full-pos "$mode" "$negatives" "$results"
cd tools/morph-compare
python3 -m unittest discover --start-directory python --pattern 'test_*.py'
python3 -m unittest test_contract_contrast
```

[러너와 리소스](2026-11-16-auxiliary-competition.md#재현)
