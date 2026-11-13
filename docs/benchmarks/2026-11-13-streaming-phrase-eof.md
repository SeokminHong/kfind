# 입력 끝의 구문 검색

Windows CI가 축소한 회귀에서 기준선 `f1ff979f`는 `1..4`를 반환했고
후보 `d64b6123`은 `1..4`와 `14..18`을 반환했습니다. 입력이 끝난 뒤 재생한
미완성 경로가 후속 일치의 확정을 막던 문제를 수정했습니다.
순서·겹침·결과 한도 판정은 완전 탐색과 100,000회 대조했습니다.

Apple M1 Max arm64, macOS 26.6.2(25G83), Rust·Cargo 1.97.0, Python 3.14.8에서
동일 release 설정으로 빌드했습니다. CLI 워밍업 2회 후 순서를 교대로 바꿔 20회
측정했습니다. 구문 검색은 512개 JSON 행의 출력이 바이트 단위로 일치했습니다.

| `사과 가격` 구문 JSON | 중앙값 ms | 최솟값 ms | 최댓값 ms | p95 ms |
| --- | ---: | ---: | ---: | ---: |
| 기준선 | 6.799 | 6.614 | 7.780 | 7.710 |
| 후보 | 6.795 | 6.595 | 8.457 | 7.662 |

중앙값은 0.06%, nearest-rank p95는 0.63% 줄었고 실행 범위가 겹쳤습니다.
입력 SHA-256은 `d20d76b2d6b73e214b3e08ab11330d0e13566e6069a9d8431b133cffd8c1d01f`입니다.

```sh
cargo build --locked --release -p kfind-cli --bin kfind
python3 tools/query-cli-benchmark/benchmark.py --baseline "$baseline" \
  --candidate "$candidate" --baseline-revision f1ff979f --candidate-revision d64b6123 \
  --warmup 2 --runs 20 --output "$report"
```
