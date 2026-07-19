# 구문 후보 행 사전 필터 벤치마크

## 대상

- baseline: `aa376331c7ba700ad5bdafa43f78a2ffdced02b8`
- candidate: `2a0193fa3b35ca5028465d5343fb011e74fba06c`
- macOS 26.4.1 (25E253), Apple Silicon
- rustc 1.97.0, cargo 1.97.0, release profile

Line-local phrase 검색의 candidate 단계가 같은 줄에 모든 atom의 raw anchor가 있는지 먼저
확인하도록 변경했다. Raw coverage는 형태·경계 의미를 확정하지 않으며, 가능한 줄만 기존 verifier로
전달한다. 제품 cache는 추가하거나 변경하지 않았다.

## Criterion 측정

공식 wrapper를 사용했다. 각 revision에서 Criterion 기본 warm-up 3초와 100 sample을 사용했고,
`sample.json`의 `times[i] / iters[i]`를 정렬한 nearest-rank p50과 p95를 기록했다.

```console
scripts/benchmark-criterion.sh matcher/phrase_input_searcher_missing_atom_long_line
scripts/benchmark-criterion.sh matcher/phrase_input_searcher_repeated_line
```

`missing_atom_long_line` 입력은 `가 `를 262,144번 반복한 1,048,576 byte 단일 줄이며
SHA-256은 `ee6a4f2a0690110c394ef78f94104ed7498d8cd413ea88c68b6afc7c099bea80`이다.
Query는 `lit:가 lit:나`, boundary는 `any`, 결과는 0건이다.

| workload | baseline p50 / p95 | candidate p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| missing atom, summary | 17.014 / 18.087 ms | 6.217 / 6.362 ms | -63.46% / -64.83% |
| 모든 atom 존재, metadata | 1.781 / 1.805 ms | 1.784 / 1.820 ms | +0.19% / +0.86% |
| 모든 atom 존재, summary | 0.982 / 0.998 ms | 0.928 / 0.985 ms | -5.47% / -1.28% |

Sample checksum:

| workload | baseline | candidate |
| --- | --- | --- |
| missing atom | `98b6b25d658bc36396ab01997451a3adc27804109b4b66be07447a1987093bb3` | `7e3a16354736392b079f95aa68abfd4f1a6bd12e4471157f5ff7d69632983150` |
| metadata | `e722508232627cf331694766ec90125ded38a5c8985653ebe48dff7c2a9cf80c` | `fa2f5106d1d2511a20d8d9e9291eeb77363814f47155ee829e169e0708a3b091` |
| summary | `e6c3e7b9d7cee793f0ba697cc1daa1b3ac284956f056e8d3f7fb4b83434c8cbf` | `1bf756888832fb53392a7b6932c86224cdd7a880ef5118e9fbe4623ec873c5cd` |

## RSS 측정

4 MiB 입력은 `가 `를 1,048,576번 반복했고 SHA-256은
`2860b8ef339d3c14cd284011a32a47ee70165f160fc225f8808b34c71de0ee12`다. 각 revision의
release binary를 fresh process로 한 번 warm-up한 뒤 5회 실행하고 `/usr/bin/time -l`의 maximum
resident set size를 기록했다.

```console
perl -Mutf8 -CS -e 'print "가 " x 1048576' |
  /usr/bin/time -l target/release/kfind \
  --literal --boundary any '가 나' - --count --no-pager >/dev/null
```

| revision | RSS min / median / max | median 변화 |
| --- | ---: | ---: |
| baseline | 124.55 / 124.56 / 124.59 MiB | - |
| candidate | 12.28 / 12.30 / 12.33 MiB | -90.13% |

## 판정

Raw atom 하나가 없는 줄은 verifier와 검증 span 벡터를 만들지 않아 시간과 RSS가 함께 감소했다.
모든 atom이 있는 정상 일치 경로는 회귀 기준 안이다. Cache hit가 발생하는 workload가 아니므로
반복 표본의 cache 편중으로 이 결과를 설명할 수 없다.

이 prefilter는 모든 atom의 raw anchor가 있는 줄을 의도적으로 false positive로 허용한다. 따라서
모든 atom을 한 번 이상 포함하면서 한 atom만 비정상적으로 반복하는 줄의 span 적재 상한은 해결하지
않는다. 그 경로는 전체 줄 후보를 모으는 역방향 DP를 bounded streaming 선택기로 바꾸는 별도 구조
작업에서 다룬다.

## 검증

- workspace format, clippy `-D warnings`, test 통과
- Rust 1.97 전체 target과 `wasm32-unknown-unknown` check 통과
- fuzz target 8개를 각각 15초 실행: 4,940,362회, crash·panic·timeout·RSS 초과 0건
- benchmark guard와 README current-information guard 통과
