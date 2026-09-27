# 괄호 대안 검색의 기존 경로 비용

- 측정일: 2026-09-27
- 기준 revision: `8049fd25a9862f9d704241b8f8571e189b28e32d`; 후보 코드 revision: `3494916fe9a0794a5ccfe2f58db918a6588e44ec`
- 환경: macOS 26.6.2 / Darwin 25.6.0, Apple M1 Max, 32 GiB, Rust/Cargo 1.97.0, Criterion 0.7.0, Python 3.14.7. 두 revision을 별도 worktree에서 같은 호스트와 release 빌드 설정으로 순차 측정했다.
- Matcher 명령: 각 worktree에서 `scripts/benchmark-criterion.sh 'matcher/(phrase_find_all$|disjunction_find_all$|grouped_find_all$)'`. Criterion 기본 warm-up 3초 뒤 workload별 100개 sample을 수집했다. 기준에는 새 `grouped_find_all` workload가 없다.
- Matcher 입력: 기존 `disjunction_find_all`은 1,024줄 72,400 bytes, SHA-256 `179344011414c9c439eea3700b3f33e844eb21b6f241e97615696eeba25de450`; 기존 `phrase_find_all`과 새 `grouped_find_all`은 1,024줄 67,840 bytes, SHA-256 `3a4a988768c1ced64293e3cf3c6a850e761ba99ebdd06c17931ead0da4f82375`다. 공통 workload의 질의·입력·설정은 같다. Benchmark source checksum은 기준 `eaba65648df691bc73898d716d31c56a88828c68153753bf7edf855c6f2d21be`, 후보 `4d61ec3ab0436dafd2ee4dcbcc2378c6f0da57aa7d2411becea2d316235f1014`이며 후보의 차이는 새 grouped workload 추가다.
- 대표값: Criterion `sample.json`의 각 `times[i] / iters[i]`를 µs로 환산한 median과 nearest-rank p95. 작을수록 빠르다.

| Matcher workload | 기준 median / p95 (µs) | 후보 median / p95 (µs) | median 변화 | p95 변화 |
| --- | ---: | ---: | ---: | ---: |
| `disjunction_find_all` | 258.27 / 266.51 | 258.48 / 262.43 | +0.08% | -1.53% |
| `phrase_find_all` | 706.61 / 729.58 | 718.53 / 739.30 | +1.69% | +1.33% |
| `grouped_find_all` | 해당 없음 | 500.51 / 514.32 | 비교 불가 | 비교 불가 |

새 grouped workload는 literal 두 대안 뒤의 literal atom을 찾는다. 기존 phrase는 형태 검증을 포함하므로 둘의 절대 시간을 성능 우열로 해석하지 않는다. 공통 workload에서 p95 최대 악화는 1.33%다.

네이티브 CLI는 양쪽 revision에서 `cargo build --release -p kfind-cli --bin kfind`로 빌드한 뒤 다음 명령으로 측정했다. 기존 `사과 가격`은 양쪽 JSON 출력이 일치하는지 검증하고, 후보의 `(사과 | 배) (가격 | 품질)`은 별도로 측정했다. 입력은 두 줄 `사과 가격\n배 품질\n`을 512회 반복한 12,800 bytes이며 SHA-256은 `d20d76b2d6b73e214b3e08ab11330d0e13566e6069a9d8431b133cffd8c1d01f`다. 각 실행은 fresh process이고 warm-up 2회 뒤 20회를 교대 측정했다. 출력 수와 선택된 atom 번호를 검증했다.

```console
python3 tools/query-cli-benchmark/benchmark.py \
  --baseline /Users/seokmin/.codex/worktrees/kfind-benchmark-baseline/kfind/target/release/kfind \
  --candidate /Users/seokmin/.codex/worktrees/kfind-phrase-alternatives/kfind/target/release/kfind \
  --baseline-revision 8049fd25a9862f9d704241b8f8571e189b28e32d \
  --candidate-revision 3494916fe9a0794a5ccfe2f58db918a6588e44ec \
  --output target/benchmark/query-cli/report.json
```

Runner SHA-256은 `518bf813c80bbc85511c4e3cb88809131148cf232b2ae0c80eaea1cdf420b0ed`다. 실행별 ns sample, binary checksum, 환경과 요약 수치는 [CLI 원본 보고서](2026-09-27-query-groups-cli.json)에 보존했다.

| CLI workload | 기준 median / p95 (ms) | 후보 median / p95 (ms) | median 변화 | p95 변화 |
| --- | ---: | ---: | ---: | ---: |
| 기존 구 JSON | 5.746 / 6.440 | 5.787 / 6.420 | +0.70% | -0.32% |
| 괄호 대안 JSON | 해당 없음 | 6.858 / 7.277 | 비교 불가 | 비교 불가 |

기존 구의 CLI p95 악화는 없었고 matcher p95 변화는 1.33%다. 새 문법의 절대 비용은 입력·출력과 후보 수에 따라 달라진다.
