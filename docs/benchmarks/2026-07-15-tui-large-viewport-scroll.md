# 대형 TUI viewport 반복 스크롤

## 결론

73×316 terminal에서 50 Hz로 300행을 이동할 때 scroll frame 중앙값은 300회에서 122회로
59.3% 줄었다. PTY 출력은 169,242 bytes에서 158,853 bytes로 6.1% 줄었고, 최종 이동량은
두 revision 모두 300행이었다.

25×80에서는 scroll frame과 출력 bytes 중앙값이 각각 300회와 54,234 bytes로 유지되었다. 큰
viewport에서 반복 입력을 더 많이 합치며 작은 viewport의 frame cadence는 바꾸지 않는다.

Candidate의 마지막 입력 뒤 최종 offset 표시 지연은 73×316에서 40.23 [2.54, 50.54] ms였다.
기준선의 0.39 [0.36, 0.55] ms보다 늘었다. 설정한 frame 간격은 48 ms이고 관측값에는 process
scheduling과 PTY 전달 시간이 더해진다. 입력된 이동량은 누락하지 않았다.

## 측정 조건

| 항목 | 값 |
| --- | --- |
| baseline revision | `2c18e72b91516afa49b1643c1dab9803b0d8b4af` |
| baseline binary SHA-256 | `cc91926665fc66d32f078fcf53fc766eeef93c3a782755db9afeee8c29ca9168` |
| candidate revision | `07703c2edcd65aa9ac87cc0fc2405894ad63daca` |
| candidate binary SHA-256 | `d5ab9eed639102ca2a0a5673db58155438e7134c2d46e203290c466880698217` |
| benchmark tool revision | `f6015fe52a7048c53680c755bfb22572b5741076` |
| OS | macOS 26.4.1 (25E253) |
| machine | MacBookPro18,2, Apple M1 Max, 32 GiB |
| Rust | rustc 1.97.0, cargo 1.97.0 |
| Python | 3.12.13 |
| build | `--release --locked` |
| 반복 | geometry마다 fresh process warm-up 1회 뒤 5회 |

Fixture는 389-byte ASCII line 2,000개로 구성한다. 각 line에는 `needle` match가 하나 있고,
전체 SHA-256은
`4f0f95fa7e420be741e7a67527fdfba2129b0394cad7077725f93e463854d064`다. PTY를 계속 읽으면서
`j` 300회를 20 ms 간격으로 보내고 마지막 상태 행이 `301/2000`이 될 때까지 측정한다.

```console
cargo +1.97.0 build --release --locked -p kfind-cli --bin kfind

python3 tools/tui-scroll-benchmark/benchmark.py \
  --binary BINARY \
  --revision REVISION \
  --label baseline-or-candidate
```

## 결과

값은 `median [min, max]`다.

| geometry | metric | baseline | candidate | 변화 |
| --- | --- | ---: | ---: | ---: |
| 25×80 | scroll frame | 300 [300, 300] | 300 [299, 300] | 0.0% |
| 25×80 | scrolled row | 300 [300, 300] | 300 [300, 300] | 0.0% |
| 25×80 | output bytes | 54,234 [54,234, 54,234] | 54,234 [54,112, 54,234] | 0.0% |
| 25×80 | catch-up | 0.38 [0.32, 0.73] ms | 0.28 [0.18, 0.42] ms | -0.10 ms |
| 73×316 | scroll frame | 300 [300, 300] | 122 [121, 122] | -59.3% |
| 73×316 | scrolled row | 300 [300, 300] | 300 [300, 300] | 0.0% |
| 73×316 | output bytes | 169,242 [169,242, 169,242] | 158,853 [158,795, 158,853] | -6.1% |
| 73×316 | catch-up | 0.39 [0.36, 0.55] ms | 40.23 [2.54, 50.54] ms | +39.84 ms |

입력 전송 시간 중앙값은 baseline 5,986.75 ms, candidate 5,982.13 ms로 같았다. 따라서 frame
감소는 입력 속도 저하가 아니라 여러 offset 이동을 같은 frame에 반영한 결과다.

이 benchmark는 실제 pager event loop와 PTY 출력을 측정하지만 terminal emulator의 cell render
시간은 포함하지 않는다. 특정 terminal의 체감 지연을 직접 수치화하지 않고, 큰 화면에서 비용을
만드는 scroll operation 수와 출력량이 줄었는지 판정한다.
