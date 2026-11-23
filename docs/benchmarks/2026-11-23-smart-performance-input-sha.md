# Smart 성능 측정 입력의 SHA-256 검증

Smart 성능 비교에서 runner 출력의 입력 SHA-256을 필수로 확인한다.
종전에는 SHA-256이 빠져도 질의 ID 순서가 같으면 집계했다.
기존 테스트 실행으로 누락 출력이 통과함을 재현했고, 동일 ID에서
질의만 바꾼 입력의 SHA-256과 누락 출력을 각각 거절하는지 확인했다.
정확한 SHA-256을 보고한 출력은 워밍업과 측정을 마친다.

## 실제 runner 대조

도구 `308b5661`로 `b7bfb8b8`의 동일 release runner와 리소스를
양쪽에서 실행했다. macOS 26.6.2 arm64, Apple M1 Max, Rust
1.97.0, Python 3.14.8을 사용했다. 입력은 1,083건의
`있다` 집중 질의와 대조군이다. 실행별 입력·리소스 체크섬,
질의 순서와 측정군별 반환 범위를 대조했다. 체크섬 검사는
runner의 지연 시간·처리량 집계가 끝나고 출력 파일을 읽을 때 수행한다.
[입력·리소스 체크섬](2026-11-23-state-auxiliary-pos.md#네이티브-성능)

```sh
cargo build --locked --release --manifest-path tools/morph-compare/runner/Cargo.toml
scripts/benchmark-run.sh run --name smart-required-input-sha -- \
  python3 tools/morph-compare/measure-smart.py "$input" "$out" \
  --baseline "$runner" b7bfb8b8 "$lexicon" "$component" "$predicates" \
  --candidate "$runner" b7bfb8b8 "$lexicon" "$component" "$predicates"
```

새 프로세스로 워밍업 1회와 교대 측정 5회를 실행했다. 수치는
중앙값 [최솟값, 최댓값]이며 p95는 최근접 순위다.

| 측정군 | 초기화 (ms) | 처리량 (건/s) | p95 (ms) | 최대 RSS (KiB) |
| --- | ---: | ---: | ---: | ---: |
| 기준선 배치 | 64.622 [62.656, 77.714] | 26,166.3 [25,461.6, 26,899.8] | 0.0566 [0.0540, 0.0588] | 65,296 [65,120, 65,392] |
| 대조 배치 | 64.463 [62.911, 65.101] | 25,671.2 [25,220.6, 26,918.1] | 0.0588 [0.0545, 0.0608] | 65,296 [65,200, 65,392] |

두 배치의 처리량 차이는 −1.89%, p95 차이는 +3.89%였다. 두
측정군의 반환 범위 체크섬은 일치했다.
