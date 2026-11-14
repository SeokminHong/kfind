# 네이티브 검색과 Node/WASM 초기화 기준선

검색 코드 기준 `6219246c`를 Apple M1 Max, RAM 32 GiB, APFS SSD,
macOS 26.6.2에서 측정했다. 네이티브는 Rust/Cargo 1.97.0 release,
WASM은 wasm-pack 0.15.0 release 및 Node 26.11.1을 사용했다.

1 GiB 혼합 코퍼스는 seed `323301756484`, 1,024개 파일이며 내용
SHA-256은 `fbb57e78aec449eb75702a3bdc386d7aca49c7dedea6b870a249db2f4b61a2b7`이다.
캐시를 예열한 뒤 도구마다 10회씩 3묶음을 실행했다. 무결과 literal 검색의
묶음별 1회 평균 중앙값은 kfind 54 ms, rg 15.2.0은 47 ms였다.
kfind의 최대 RSS 중앙값은 10.75 MiB였다.

| 네이티브 입력 | 반환 줄 수 | 중앙값 [최소, 최대] ms |
| --- | ---: | --- |
| 빈 입력, `v:가다`, 매회 새 프로세스 | 0 | 50.93 [49.55, 51.60] |
| 일치 80,000줄, 3,160,000 bytes, literal | 80,000 | 18.78 [18.71, 18.86] |
| 2,000,013 bytes 단일 줄, literal | 1 | 9.21 [8.95, 9.24] |

네이티브 세 입력은 워밍업 1회 후 각각 새 프로세스로 5회 측정했다.
빈 입력은 프로세스·사전 초기화 비용을 포함한다. 운영체제의 콜드 파일 캐시는
측정하지 않았다. 고밀도는 `그는 특허를 가지지 않았다.\n아무도 산길을 가지지 않았다.\n`
40,000쌍, 단일 줄은 `사과와 바나나 ` 100,000회 뒤에 `희귀표식\n`을 더했다.
입력 SHA-256은 각각
`23bd6680c64f2b1ebc907c0ba59a8a516d3148d31b6a25646f7db09fae9c4844`,
`aaf5a1f793d89ce21e5125c8743fc9a51f2e8e3fc7a44e6c4459562021f7d427`이다.

Node/WASM은 모듈 로드 후 엔진 생성과 리소스 로딩을 측정한다. 프로필마다
워밍업 1회 후 5회, 매회 새 Node 프로세스를 사용했다.

| Node/WASM 리소스 | 초기화 중앙값 [최소, 최대] ms | RSS 중앙값 MiB |
| --- | --- | ---: |
| embedded | 19.16 [19.03, 19.45] | 54.25 |
| embedded + component | 926.97 [924.07, 929.93] | 140.46 |
| full POS | 61.28 [60.95, 61.56] | 92.19 |
| full POS + component | 954.23 [946.29, 1023.86] | 165.05 |

full POS + component에서 component 로딩 중앙값만 892.93 ms였다.

```sh
cargo build --release --locked -p kfind-cli -p kfind-testkit --bins
KFIND_BENCH_REPORT="$report" scripts/benchmark-1gib.sh
KFIND_DATA_DIR="$data_dir" ./target/release/kfind --count 'v:가다' /dev/null
./target/release/kfind --literal --count --threads 1 '가지지' "$dense"
./target/release/kfind --literal --count --threads 1 '희귀표식' "$long_line"
wasm-pack build crates/kfind-wasm --target nodejs --out-dir "$wasm_dir" --out-name kfind --release --locked
node --expose-gc scripts/benchmark-npm-startup.cjs "$wasm_dir/kfind.js" "$component" "$full_pos" "$output" 5
```

WASM SHA-256: `e0f516d5aef55879609b3313e252aed0132d03cf9e71013d37c7c29ce109c538`.
[WASM 리소스 체크섬](2026-11-14-gu-exclamative.md#재현).
