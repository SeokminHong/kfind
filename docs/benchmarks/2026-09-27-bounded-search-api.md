# 공개 결과 상한 API의 기존 phrase 검색 비용

- 측정일: 2026-09-27
- 기준 revision: `2f3c93cdb6d9dbd256f5fb606373023a6e39752d`; 후보 코드 revision: `3e96751440a06127da79b083971f5aa18966cd71`
- 환경: macOS Darwin 25.6.0, Apple M1 Max, 32 GB, Rust/Cargo 1.97.0, Criterion 0.7.0. 두 revision을 별도 worktree에서 같은 호스트와 `--release` 설정으로 순차 측정했다.
- 명령: 각 worktree에서 `scripts/benchmark-criterion.sh matcher/phrase_find_all`. Criterion 기본 warm-up 3초 뒤 각 workload 100개 sample을 수집했다.
- 고정 입력: `query_matcher.rs` SHA-256 `eaba65648df691bc73898d716d31c56a88828c68153753bf7edf855c6f2d21be`. `phrase_find_all`은 1,024줄 67,840 bytes, SHA-256 `3a4a988768c1ced64293e3cf3c6a850e761ba99ebdd06c17931ead0da4f82375`; `phrase_find_all_repeated`는 128음절 384 bytes, SHA-256 `d52a8bc70bef97ac9c43f989776b3288d2b117525d129e9e9d23ace578efd7c1`이다. 두 revision의 질의·입력·구성 요소 fixture는 같다.
- 대표값은 각 sample의 `times[i] / iters[i]`로 계산한 실행 1회당 시간의 median이다. p95는 오름차순 nearest rank다. 단위는 µs이며 작을수록 빠르다.

| workload                           | 기준 median / p95 | 후보 median / p95 | median 변화 | p95 변화 |
| ---------------------------------- | ----------------: | ----------------: | ----------: | -------: |
| `matcher/phrase_find_all`          |   737.83 / 759.66 |   752.86 / 773.06 |      +2.04% |   +1.76% |
| `matcher/phrase_find_all_repeated` |   146.01 / 150.08 |   149.52 / 152.93 |      +2.41% |   +1.90% |

두 기존 경로 모두 불리한 변화가 있었으나 p95 악화는 2% 미만이다. 이번 API는 기존 matcher의 결과 상한 경로를 호출하며 이 benchmark의 검색 경로는 바꾸지 않는다. 단일 호스트 순차 측정이므로 작은 차이를 코드 원인으로 단정하지 않는다. 새 상한 API의 메모리 계약은 반환 결과 수에 적용되며 입력과 검색기의 작업 상태 전체를 제한한다는 뜻은 아니다.
