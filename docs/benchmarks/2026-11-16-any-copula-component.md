# Any 지정사의 선택 리소스 검증

component 리소스가 있는 Any는 Smart와 같은 지정사 구조 검증을 사용해
`너라면`, `누굴까`, `뭐야`처럼 지정사가 융합되거나 생략된 어절을
회수한다. 선택 구조 프로그램은 해당 지정사 질의에서 실행한다.

## 품질

고정 입력 12세트의 모드별 결과 249,243행을 비교했다. 리소스 없는 Any와
Smart·Token의 반환 범위는 기준선과 같다. 리소스를 사용한 Any에서 변경된
결과 181행은 모두 `이다` 질의다. 검토 사전에서 빠졌던 지정사 83건이
회수됐고 남은 1건은 동사 `웃기다`의 `웃기담`이다. KAIST dev·test와
무품사 matrix의 계약 FN은 각각 3→0, 2→1, 2→1이다. 고정 평가 11세트의
Smart 계약 FN은 0건이다.

고정 GSD·PUD 음성에는 `이다` 질의가 없다. 지정사 전용 대조는
`data/fixtures/ud_morphology_cases.tsv`와
`tools/morph-compare/*copula*-context-cases.jsonl`의 `이다` 형용사 행
233개를 사용했다. 기존 대조의 계약 양성 5건을 동일하게 적용했다.
추가 raw FP 3건은 `영화라도`, `새라면`, `가라면`으로, 각각 기존
계약에 경쟁 해석이 기록된 사례다. 전체 대조의 계약 FP는 16건이다.
[대조 문장과 기존 계약](../../tools/morph-compare/copula-conditional-contract-reviews.tsv)

행렬은 TP, FP, TN, FN 순서이며 비율(%)은 precision, recall, F1 순서다.

| Any 입력 | 버전 | 원본 행렬 / 비율 (%) | 계약 행렬 / 비율 (%) |
| --- | --- | --- | --- |
| 검토 사전 4,458행 | 기준선 | 4374, 0, 0, 84 / 100.00, 98.12, 99.05 | 4374, 0, 0, 84 / 100.00, 98.12, 99.05 |
| 검토 사전 4,458행 | 리소스 후보 | 4457, 0, 0, 1 / 100.00, 99.98, 99.99 | 4457, 0, 0, 1 / 100.00, 99.98, 99.99 |
| 지정사 대조 233행 | 기준선 | 98, 18, 50, 67 / 84.48, 59.39, 69.75 | 100, 16, 47, 70 / 86.21, 58.82, 69.93 |
| 지정사 대조 233행 | 리소스 후보 | 165, 21, 47, 0 / 88.71, 100.00, 94.02 | 170, 16, 47, 0 / 91.40, 100.00, 95.51 |

Workspace 테스트 642건, gold 925건과 walk/hang 스트레스, Clippy 및
포맷 검증을 통과했다. CLI에서 리소스 유무에 따른 실제 반환도 확인했다.

## 성능

Apple M1 Max arm64, macOS 26.6.2 (25G83), Rust/Cargo 1.97.0 release에서
기준선 `9f7aeb88`과 후보 `4fa6b3d9`를 비교했다. 양쪽에 같은 전체
데이터를 지정하고 새 프로세스로 워밍업 1회, 측정 5회를 수행했다. 실행
순서를 교대했다. 표는 중앙값 [최솟값, 최댓값]이며 p95는 최근접 순위다.

| 검토 사전 Any | 초기화 ms | 처리량 cases/s | p95 ms | RSS MiB |
| --- | --- | --- | --- | --- |
| 기준선 | 28.134 [28.067, 28.162] | 24,075 [23,153, 24,089] | 0.06988 [0.06896, 0.07317] | 34.938 [34.922, 35.063] |
| 리소스 후보 | 61.458 [61.063, 63.057] | 23,416 [23,291, 23,537] | 0.07067 [0.07004, 0.07113] | 70.719 [70.641, 70.781] |

| CLI 지정사 대조 stdin 9,359바이트 | 벽시계 ms | RSS MiB | 반환 범위 |
| --- | --- | --- | --- |
| 기준선 | 32.717 [32.474, 32.824] | 26.297 [26.234, 26.328] | 175 |
| 리소스 후보 | 50.909 [49.800, 51.736] | 62.219 [62.156, 62.250] | 244 |

검토 사전 처리량은 2.74% 감소했고 p95는 1.13% 증가했다.
리소스 검증을 포함한 CLI 벽시계 중앙값은 18.192ms 증가했고 최대 RSS
중앙값은 35.922MiB 증가했다.

## 재현

component·품사·용언 리소스는 [기존 체크섬](2026-11-16-selected-nominal-particle-tail.md#성능)을 따른다.
검토 사전 입력 SHA-256은
`e0856d5a9ca69d311dc37134fb32b1bc50549cca912d061c1ed6526b6c7eea5f`다.
대조는 TSV 순서로 대상 행을 읽은 뒤 파일명 순으로 JSONL의 `이다` 행을 추가한다.
CLI용 stdin은 각 행의 원문에 줄바꿈을 붙여 만들며 SHA-256은
`9cd657a8850a2c269fe328d391369cae2f509ec0dbc2608ef7f55a1aaf403495`다.

```sh
cargo build --release --locked --manifest-path tools/morph-compare/runner/Cargo.toml --target-dir "$runner_build"
cargo build --release --locked -p kfind-cli --target-dir "$cli_build"
KFIND_FULL_POS_LEXICON="$data/lexicon.bin" KFIND_COMPONENT_RESOURCE="$data/morphology-component-compact.kfc" KFIND_ENRICHED_PREDICATES="$data/predicates.enriched.tsv" \
  /usr/bin/time -l "$runner_build/release/morph-benchmark-runner" boundary full-pos any "$input" "$output"
/usr/bin/time -l "$cli_build/release/kfind" --data-dir "$data" --boundary any --pos adjective --json 이다 - < "$context_text"
```
