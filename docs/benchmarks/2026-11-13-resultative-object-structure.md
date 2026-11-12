# 소유 목적어와 이동 결과형의 검색

`재산을 가지지 않았다`에서 `가다`가 반환된 사례를 일반 오탐으로 채점했습니다.
소유 목적어 앞의 `가지다`와 경로 성분을 받는 이동 결과형의 정확한 범위를 회귀로
검증했습니다. [한국어기초사전 가다의 문형](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=27500) ·
[국립국어원 -어지다](https://korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=5737&mn_id=217&pageIndex=222)

기준선은 `776a08c63841956bbbf5e4e9e18a0248372b1725`, 후보는
`19269fb8ef58d4a837ac4487f57fc10047c63b65`입니다.
[회귀](../../data/fixtures/ud_morphology_cases.tsv) ·
[대조 입력](../../tools/morph-compare/resultative-context-cases.jsonl) ·
[계약 검토](../../tools/morph-compare/resultative-contract-reviews.tsv)

행렬 순서는 TP, FP, TN, FN이며 비율은 precision, recall, F1입니다. 결과형 대조
양성과 GSD 양성은 정확한 범위, 결과형 대조 음성과 KAIST는 겹침, 일반 음성은
반환 여부로 채점했습니다. 계약 검토 0건에서는 두 지표가 같습니다.

| 명시적 품사 full POS smart | 리비전 | Raw 행렬 | Raw 비율 (%) | 계약 보정 행렬 | 계약 보정 비율 (%) | 검토 |
| --- | --- | --- | --- | --- | --- | ---: |
| 결과형 대조 8건 | 기준선 | 4, 2, 2, 0 | 66.67, 100.00, 80.00 | 5, 1, 2, 0 | 83.33, 100.00, 90.91 | 4 |
| 결과형 대조 8건 | 후보 | 4, 1, 3, 0 | 80.00, 100.00, 88.89 | 5, 0, 3, 0 | 100.00, 100.00, 100.00 | 4 |
| GSD 교정 양성 9,335건 | 양쪽 | 8153, 0, 0, 1182 | 100.00, 87.34, 93.24 | 8153, 0, 0, 1182 | 100.00, 87.34, 93.24 | 0 |
| GSD·PUD 원본 음성 53,308건 | 양쪽 | 0, 4081, 49227, 0 | 0.00, 0.00, 0.00 | 26, 4055, 49227, 0 | 0.64, 100.00, 1.27 | 26 |
| KAIST dev 교정 2,532건 | 양쪽 | 1265, 1, 1266, 0 | 99.92, 100.00, 99.96 | 1266, 0, 1266, 0 | 100.00, 100.00, 100.00 | 1 |
| KAIST test 교정 2,592건 | 양쪽 | 1289, 2, 1294, 7 | 99.85, 99.46, 99.65 | 1290, 0, 1298, 0 | 100.00, 100.00, 100.00 | 25 |

## 성능

Apple M1 Max arm64, macOS 26.6.2(25G83), Darwin 25.6.0, Rust·Cargo 1.97.0에서
같은 release 설정과 리소스를 사용했습니다. 입력별로 새 프로세스에서 warm-up 1회 뒤
5회 측정했고 실행 순서는 교대했습니다. 처리량은 평가 시간, p95는 nearest-rank,
RSS는 `/usr/bin/time -l`의 최대 상주 메모리입니다. 중앙값 [최솟값, 최댓값]을 표기합니다.

| 입력 | 리비전 | 초기화 ms | 처리량 cases/s | p95 ms | RSS MiB |
| --- | --- | --- | --- | --- | --- |
| GSD 교정 양성 | 기준선 | 61.083 [59.850, 62.323] | 30736 [28615, 31165] | 0.0732 [0.0713, 0.0791] | 79.078 [78.844, 79.266] |
| GSD 교정 양성 | 후보 | 59.894 [59.737, 60.348] | 30887 [29448, 31208] | 0.0733 [0.0719, 0.0760] | 79.000 [78.781, 79.344] |
| GSD·PUD 원본 음성 | 기준선 | 59.955 [59.642, 60.155] | 18797 [18693, 19053] | 0.1234 [0.1203, 0.1240] | 125.625 [125.344, 125.703] |
| GSD·PUD 원본 음성 | 후보 | 59.783 [59.325, 63.147] | 18944 [18655, 19113] | 0.1218 [0.1204, 0.1243] | 125.578 [125.500, 126.422] |

양성 처리량은 0.49%, 음성 처리량은 0.78% 늘었습니다. 양성 p95는 0.14% 늘고
음성 p95는 1.30% 줄었습니다.

```sh
cargo build --locked --release --manifest-path tools/morph-compare/runner/Cargo.toml
KFIND_FULL_POS_LEXICON="$full_pos_dir/lexicon.bin" \
KFIND_COMPONENT_RESOURCE="$component_resource" \
KFIND_ENRICHED_PREDICATES="$enriched_predicates" \
  /usr/bin/time -l "$runner" boundary full-pos smart "$input" "$output"
```

결과형 대조 JSONL SHA-256은 `30e26ad541ce4587a60ba9b95c93df52dd2b9e6a0eabc1cfe0ea777d226776c7`,
계약 검토 SHA-256은 `197553db04b3476a2693997c355b9bae34e4b13519088597306ba422c0b9a96a`입니다.
[공통 말뭉치와 리소스 SHA-256](2026-11-13-grammatical-nominal-suffixes.md#성능)
