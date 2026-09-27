# 0건 검색 안내의 CLI 실행 비용

- 측정일: 2026-09-27
- 기준 revision: `cdab0dec2537219292de9a3f36d3f9f1592db71c`; 후보 revision: `d73855764e0416be839f02c0926d91a78d8a9179`
- 환경: macOS 26.6.2, Apple M1 Max, arm64, 32 GiB, Rust 1.97.0, Python 3.14.7. 두 revision을 별도 worktree에서 `cargo build --release -p kfind-cli --bin kfind`로 빌드했다.
- 명령: `python3 tools/no-match-cli-benchmark/benchmark.py --baseline <기준 kfind> --candidate <후보 kfind> --baseline-revision cdab0dec2537219292de9a3f36d3f9f1592db71c --candidate-revision d73855764e0416be839f02c0926d91a78d8a9179 --output target/benchmark/no-match/report.json`. Runner SHA-256은 `ab372ca9a887bf8ac5c736e9f566911a60f99c6ce1f5ea46b66f677c65774df8`이다.
- 입력: `대상이 없습니다.\n` 512줄, 12,288 bytes, SHA-256 `5580e19c150c92ffb436a767cf5035886c2a27b904086bf6d305f7d25f6ae9f9`. 각 fresh CLI process는 `--embedded --no-pager 걷다 <입력 파일>`을 실행하고, 안내 workload만 후보에서 `--explain-no-match`를 추가한다.
- 측정: warm-up 2회 뒤 20회를 교대 실행했다. 각 실행의 wall time을 ns로 기록하고 median과 nearest-rank p95를 ms로 환산했다. 종료 코드 1과 빈 stdout을 검증했고, 안내 workload는 stderr의 안내를 검증했다. 실행별 sample과 binary checksum은 [원본 보고서](2026-09-27-no-match-cli.json)에 있다.

| CLI workload | median / p95 (ms) | 기존 기준 대비 median / p95 | 후보 기본 대비 median / p95 |
| --- | ---: | ---: | ---: |
| 기준 기본 | 8.534 / 9.015 | 기준 | 해당 없음 |
| 후보 기본 | 8.463 / 8.981 | -0.83% / -0.38% | 기준 |
| 후보 `--explain-no-match` | 8.566 / 9.616 | +0.37% / +6.66% | +1.21% / +7.07% |

기본 검색의 관측 회귀는 없다. 0건 안내를 켜면 stderr 출력과 질의 재해석 비용이 추가되며, 이번 입력에서 p95는 후보 기본 검색보다 7.07% 늘었다.
