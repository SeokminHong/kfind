# kfind 검색 질의 compile benchmark

## 환경

```text
revision: 026ff8104c867eaf1a7a8239eac5e589a26aa11f
CPU: Apple M1 Max
memory: 32 GiB
architecture: arm64
OS: macOS 26.4.1 (25E253)
rustc: 1.94.1 (e408947bf 2026-03-25)
```

## 실행 명령

```console
cargo bench -p kfind-testkit --bench query_matcher -- query_compile
```

Criterion 기본 설정으로 각 benchmark의 warm-up 뒤 100개 sample을 수집했다. p95는
`new/sample.json`의 `times[i] / iters[i]`를 nearest-rank 방식으로 계산했다.

| benchmark | p95 | target | result |
| --- | ---: | ---: | --- |
| single atom | 0.077875 ms | 15 ms 이하 | pass |
| 8 atom phrase | 0.153838 ms | 50 ms 이하 | pass |
