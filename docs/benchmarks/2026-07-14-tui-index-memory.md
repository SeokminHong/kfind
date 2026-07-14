# TUI index memory benchmark

## 결론

TUI index 메모리는 source line 수 `S`와 현재 terminal 너비에서 전개된 layout row 수 `R`에
선형 비례한다. Apple Silicon 64-bit 대상에서 `SourceIndex`는 16 bytes, `RowKey`는 24 bytes이므로
초기화된 entry 메모리는 `16S + 24R` bytes다.

100만 plain line은 index entry 38.15 MiB와 peak RSS 43.50 MiB를 사용했다. 500만 plain line은
각 벡터의 capacity가 8,388,608로 증가해 320 MiB를 예약했고, 초기화된 entry는 190.73 MiB,
peak RSS는 194.59 MiB였다. 100만 source line이 4개 match row씩 전개된 profile은 400만 row,
초기화된 entry 106.81 MiB, peak RSS 110.81 MiB였다.

현재 pager에는 자동 결과 상한이나 대용량 fallback이 없다. 수백만 결과가 예상되고 interactive
navigation이 필요하지 않으면 `--no-pager`로 bounded stdout stream을 사용한다. sparse·chunk index나
자동 fallback은 이 측정에서 도입하지 않는다.

## 측정 조건

| 항목 | 값 |
| --- | --- |
| revision | `4d81a229d5b85b4c2f579a9799ea775be2795af7` |
| OS | macOS 26.4.1 (25E253) |
| machine | MacBookPro18,2, Apple M1 Max, 32 GiB |
| Rust | rustc 1.97.0, cargo 1.97.0 |
| build | `--release --locked`, `pager-memory-benchmark` feature |
| 반복 | fresh process warm-up 1회 뒤 5회 |
| RSS | `/usr/bin/time -l`의 maximum resident set size |

benchmark binary는 실제 `Document::open`과 `Document::layout`을 사용한다. Plain profile은 한 source
line이 한 row이고, expanded profile은 32-column layout에서 긴 source line 하나가 4개 match row로
전개된다. 입력은 process마다 unnamed temporary file에 생성해 종료 시 제거한다.

```console
cargo build --release --locked -p kfind-cli \
  --features pager-memory-benchmark \
  --bin kfind-pager-memory-benchmark

target/release/kfind-pager-memory-benchmark SOURCE_LINES MATCHES_PER_LINE TERMINAL_WIDTH
/usr/bin/time -l target/release/kfind-pager-memory-benchmark \
  SOURCE_LINES MATCHES_PER_LINE TERMINAL_WIDTH
```

측정 profile은 `(100000, 0, 80)`, `(1000000, 0, 80)`, `(5000000, 0, 80)`,
`(25000, 4, 32)`, `(250000, 4, 32)`, `(1000000, 4, 32)`다. 각 JSON은 입력 bytes,
source·row length와 capacity, entry 크기, length·capacity 기준 bytes와 생성·index·layout 시간을
기록한다.

## 메모리

시간과 RSS는 `median [min, max]`다. MiB는 1,048,576 bytes다.

| profile | source / row | temp file | entry initialized | vector capacity | peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| plain 100k | 100,000 / 100,000 | 1.34 MiB | 3.81 MiB | 5.00 MiB | 7.17 [7.16, 7.19] MiB |
| plain 1m | 1,000,000 / 1,000,000 | 13.35 MiB | 38.15 MiB | 40.00 MiB | 43.50 [42.02, 43.53] MiB |
| plain 5m | 5,000,000 / 5,000,000 | 66.76 MiB | 190.73 MiB | 320.00 MiB | 194.59 [194.59, 196.11] MiB |
| expanded 100k rows | 25,000 / 100,000 | 10.75 MiB | 2.67 MiB | 3.50 MiB | 6.14 [6.11, 6.14] MiB |
| expanded 1m rows | 250,000 / 1,000,000 | 107.53 MiB | 26.70 MiB | 28.00 MiB | 30.67 [30.66, 30.69] MiB |
| expanded 4m rows | 1,000,000 / 4,000,000 | 430.11 MiB | 106.81 MiB | 112.00 MiB | 110.81 [110.80, 112.31] MiB |

500만 plain profile의 capacity bytes와 RSS 차이는 벡터가 예약한 capacity 전체가 resident page로
초기화되지 않았기 때문이다. 실제 entry 수에 따른 메모리와 예약 capacity를 구분해 보고해야 한다.

## 시간

| profile | input 생성 | source index | layout | process wall |
| --- | ---: | ---: | ---: | ---: |
| plain 100k | 0.0015 [0.0014, 0.0020] s | 0.0021 [0.0018, 0.0021] s | 0.0810 [0.0803, 0.0843] s | 0.09 [0.08, 0.09] s |
| plain 1m | 0.0115 [0.0106, 0.0124] s | 0.0201 [0.0193, 0.0203] s | 0.8186 [0.8070, 0.8239] s | 0.85 [0.84, 0.86] s |
| plain 5m | 0.0545 [0.0534, 0.0606] s | 0.0961 [0.0941, 0.1023] s | 4.0471 [4.0127, 4.0710] s | 4.20 [4.17, 4.23] s |
| expanded 100k rows | 0.0218 [0.0213, 0.0224] s | 0.0025 [0.0023, 0.0034] s | 0.0612 [0.0586, 0.0617] s | 0.08 [0.08, 0.09] s |
| expanded 1m rows | 0.2159 [0.2116, 0.2566] s | 0.0255 [0.0250, 0.0261] s | 0.5850 [0.5811, 0.6419] s | 0.84 [0.82, 0.89] s |
| expanded 4m rows | 0.8680 [0.8183, 0.8738] s | 0.0987 [0.0962, 0.1198] s | 2.3839 [2.3199, 2.4146] s | 3.36 [3.24, 3.41] s |

이 benchmark는 index와 layout의 규모 특성을 측정한다. 실제 검색, TTY event loop, 화면 render
비용은 포함하지 않으므로 end-to-end 검색 처리량으로 해석하지 않는다.
