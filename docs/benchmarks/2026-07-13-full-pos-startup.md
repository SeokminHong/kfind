# full POS 지연 조회 초기화 비교

## 결론

full POS entry 전체를 `Analysis` map으로 전개하지 않고 정렬된 POS index로 보존한 뒤 query
atom lookup에서만 분석을 생성했다. 632,667-entry artifact에서 native warm startup은
0.39초에서 0.08초로, Node WASM 초기화는 908.74ms에서 101.70ms로 줄었다. query compile
지연은 측정 오차 범위에서 유지됐다.

## 입력과 환경

- baseline revision: `5c7792c`
- candidate revision: `8845f33`
- full POS SHA-256: `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88`
- full POS entries: 632,667
- host: Apple M1 Max, 32 GiB, macOS 26.4.1 arm64
- Rust: 1.97.0
- Node: 24.5.0
- wasm-pack: 0.15.0

native는 각 revision의 release binary를 별도 build했다. 첫 filesystem cold run을 버리고
`/usr/bin/time -l`로 4회 이상 측정했다. Node WASM은 별도 process에서 module load, core
초기화, lexicon read, `Kfind.withFullPos`를 나눠 3회 측정했다. RSS는 full POS 생성 직전과
직후의 process RSS 차이다.

## 결과

| runtime | metric | baseline | candidate | 변화 |
| --- | --- | ---: | ---: | ---: |
| native CLI | warm startup median | 0.39 s | 0.08 s | -79.5% |
| native CLI | peak RSS median | 263.0 MiB | 39.5 MiB | -85.0% |
| Node WASM | full POS init median | 908.74 ms | 101.70 ms | -88.8% |
| Node WASM | RSS delta median | 184.4 MiB | 60.4 MiB | -67.2% |
| Node WASM | repeated query compile | 143.98 us | 140.58 us | no regression |

WASM package의 `kfind_bg.wasm`은 1,014,792 bytes에서 1,018,340 bytes로 3,548 bytes
증가했다. candidate의 `npm pack --dry-run` 결과는 387,414 bytes packed, 1,039,655 bytes
unpacked다.

## 명령

```sh
cargo build --release --locked --package kfind-cli
/usr/bin/time -l target/release/kfind \
  --data-dir /tmp/kfind-official-analysis-full-pos \
  --quiet 분석하다 /dev/null

pnpm --dir packages/kfind run pack:check
cargo run --release --locked --package kfind-testkit --bin verify-gold -- \
  /tmp/kfind-official-analysis-full-pos/lexicon.bin
```

Node query compile은 full POS engine을 한 번 만든 뒤 `분석하다` matcher를 100회 warm-up하고
5,000회 생성·해제한 전체 시간을 나눴다. 초기화와 query compile 측정에는 lexicon 다운로드
시간을 포함하지 않았다.
