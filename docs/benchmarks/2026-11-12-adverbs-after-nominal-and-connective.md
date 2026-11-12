# 명사와 연결형 뒤의 부사 검색

기준선은 `59d5d8d3e3a5dbafe121202b3a33d33c797d45b6`, 후보는
`9ad0cd68220148c75697cc382883dfcb3849e68c`입니다.

## 품질

`한 달 내내`, `돌리고서는 축 늘어졌다`, `태도는 참 가관이었다`에서는
현재 어절에 부사 분석이 있습니다. 앞 어절의 관형형 근거가 불완전하거나
앞 어절에 명사 분석도 있으면 의존 명사를 우선하지 않습니다. Korean-Kaist dev
2건과 Korean-GSD test 1건의 부사 범위를 `smart`와 `any`에서 확인합니다.
[UD 범위 회귀](../../data/fixtures/ud_morphology_cases.tsv) ·
[한국어기초사전 내내](https://krdict.korean.go.kr/m/eng/searchResultView?ParaSenseSeq=&ParaWordNo=60627&fileNo=&imgCount=&multiMediaSeq=&nation=eng&searchKind=&searchKindValue=&shortenUrl=&studySeq=)

KAIST는 기존 겹침 기준, GSD는 정확한 범위 기준입니다. 행렬은 TP, FP, TN, FN
순서이고 비율은 precision, recall, F1 순서입니다. GSD는 계약 검토 0건으로
raw와 계약 보정값이 같습니다. Hard-negative는 계약 양성 검토 5건을 포함합니다.
[KAIST 계약 검토](../../tools/morph-compare/quality-contract-reviews.tsv)

| 입력 | 모드 | revision | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) |
| --- | --- | --- | --- | --- | --- | --- |
| KAIST dev 1,000건 | smart | 기준선 | 483, 0, 500, 17 | 100.00, 96.60, 98.27 | 482, 0, 501, 15 | 100.00, 96.98, 98.47 |
| KAIST dev 1,000건 | smart | 후보 | 485, 0, 500, 15 | 100.00, 97.00, 98.48 | 484, 0, 501, 13 | 100.00, 97.38, 98.67 |
| KAIST test 1,000건 | smart | 양쪽 | 497, 0, 500, 3 | 100.00, 99.40, 99.70 | 497, 0, 501, 0 | 100.00, 100.00, 100.00 |
| Hard-negative 45건 | smart | 양쪽 | 0, 5, 40, 0 | 0.00, 0.00, 0.00 | 5, 0, 40, 0 | 100.00, 100.00, 100.00 |
| GSD 9,335건 | smart | 기준선 | 8079, 0, 0, 1256 | 100.00, 86.55, 92.79 | 8079, 0, 0, 1256 | 100.00, 86.55, 92.79 |
| GSD 9,335건 | smart | 후보 | 8080, 0, 0, 1255 | 100.00, 86.56, 92.79 | 8080, 0, 0, 1255 | 100.00, 86.56, 92.79 |
| GSD 9,335건 | any | 양쪽 | 8041, 0, 0, 1294 | 100.00, 86.14, 92.55 | 8041, 0, 0, 1294 | 100.00, 86.14, 92.55 |

## 성능

Apple M1 Max arm64, Darwin 25.6.0, Rust와 Cargo 1.97.0에서 full-POS 릴리스
runner로 측정했습니다. 입력·모드마다 새 프로세스에서 warm-up 1회 후 5회
실행했고 revision 순서는 교대로 바꿨습니다. p95는 nearest-rank로 계산했습니다.
표는 중앙값 [최솟값, 최댓값]입니다.

| 입력 | 모드 | revision | 초기화 ms | 처리량 cases/s | p95 ms | RSS MiB |
| --- | --- | --- | --- | --- | --- | --- |
| GSD | smart | 기준선 | 59.431 [59.266, 59.538] | 29,634 [28,048, 29,732] | 0.0763 [0.0759, 0.0803] | 78.609 [78.531, 78.875] |
| GSD | smart | 후보 | 59.236 [59.165, 59.374] | 29,617 [28,590, 29,697] | 0.0761 [0.0759, 0.0791] | 78.547 [78.531, 78.781] |
| GSD | any | 기준선 | 26.952 [26.784, 27.923] | 59,554 [55,894, 59,761] | 0.0409 [0.0406, 0.0431] | 43.297 [43.156, 43.359] |
| GSD | any | 후보 | 26.850 [26.749, 26.908] | 59,595 [57,068, 60,080] | 0.0406 [0.0405, 0.0421] | 43.047 [42.938, 43.078] |
| KAIST dev | smart | 기준선 | 61.111 [61.029, 61.238] | 28,274 [27,210, 28,437] | 0.0914 [0.0896, 0.0932] | 61.719 [61.641, 61.781] |
| KAIST dev | smart | 후보 | 61.156 [60.839, 61.379] | 28,081 [26,964, 28,300] | 0.0909 [0.0902, 0.0934] | 61.609 [61.578, 61.750] |

GSD `smart` 처리량은 0.06% 감소했고 p95는 0.22% 감소했습니다. KAIST dev
`smart` 처리량은 0.68% 감소했고 p95는 0.55% 감소했습니다. GSD `any` 처리량은
0.07% 증가했고 p95는 0.71% 감소했습니다.

```sh
cargo build --release --locked --manifest-path tools/morph-compare/runner/Cargo.toml
/usr/bin/time -l "$runner" boundary full-pos "$boundary" "$input" "$output"
```

[GSD·KAIST·hard-negative JSONL checksum](2026-11-12-topic-before-predicate.md#성능) ·
[사전 리소스 checksum](2026-11-12-ud-and-ending-coverage.md#성능)
