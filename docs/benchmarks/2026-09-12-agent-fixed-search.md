# 에이전트 고정 문자열 검색 허용 비용

- 측정일: 2026-09-12
- 기준 revision: `bae0a005`
- 후보 revision: `e324ecd04698fbdab4b5531ac88218dadb6bc0bc`
- 환경: {'machine': 'arm64', 'platform': 'macOS-26.6.2-arm64-arm-64bit-Mach-O', 'python': '3.14.7'}; Rust 1.97.0, release locked build
- 각 workload는 fresh process warm-up 10회 후 200회 측정하며 순서를 순환한다.
- 대표값은 중앙값, p95는 정렬 표본의 nearest rank다. 원본 JSON은 각 입력과 binary SHA-256을 포함한다.

## 결과

중앙값 변화는 -0.46%~+1.47%이며 최대 절대 증가는 약 0.055 ms다.
p95의 최대 증가는 약 1.74%다. 이 fresh-process 표본에서는 뚜렷한 시작 비용 회귀가 없다.

| workload      | 기준 median / p95 (ms) | 후보 median / p95 (ms) | median 변화 |
| ------------- | ---------------------: | ---------------------: | ----------: |
| version       |        3.7449 / 4.6600 |        3.7413 / 4.6945 |      -0.10% |
| codex_allow   |        3.7703 / 4.7736 |        3.7970 / 4.6851 |      +0.71% |
| codex_deny    |        3.7506 / 4.7767 |        3.7389 / 4.8595 |      -0.31% |
| gemini_deny   |        3.7704 / 4.7490 |        3.7532 / 4.7691 |      -0.46% |
| session_start |        3.7440 / 4.7762 |        3.7444 / 4.7144 |      +0.01% |
| codex_fixed   |        3.7290 / 4.6607 |        3.7837 / 4.7415 |      +1.47% |

`codex_fixed`는 같은 `rg -F 사용자 crates` 입력에서 기준의 차단과 후보의 허용을 각각 검증한다.
나머지 workload는 같은 허용·차단·세션 응답 계약을 검증한다. 형태 검색 경로를 바꾸지 않아
morphology 품질·성능은 이 hook 측정에 포함하지 않는다.

## 재현

```sh
scripts/benchmark-run.sh run --name agent-fixed-search -- \
  python3 tools/agent-hook-benchmark/benchmark.py \
    --baseline target/review-baseline/kfind --baseline-revision bae0a005 \
    --candidate target/release/kfind --candidate-revision e324ecd04698fbdab4b5531ac88218dadb6bc0bc \
    --warmups 10 --runs 200 \
    --output target/benchmark/agent-fixed-search/report.json
```
