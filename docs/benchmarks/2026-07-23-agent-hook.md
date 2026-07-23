# 에이전트 hook 시작 비용

- 측정일: 2026-07-23
- 기준 revision: `33404b860e1d5251fc1766fa88102daf4971c9d6`
- 후보 revision: `9d7d00f73cbf8bb44770a6b220ee7569b92756fb`
- 환경: macOS 26.4.1, Apple M1 Max, 32 GiB, arm64, Rust 1.97.0,
  Python 3.14.6
- 반복: workload별 fresh process warm-up 10회 뒤 200회 측정,
  round-robin 순서 교대

## 판정

공통 `--version` workload의 후보 중앙값은 기준보다 0.88% 낮고 p95는 1.55% 높다.
절대 변화는 각각 -0.033 ms와 +0.071 ms이며 측정 범위 안이다. 기존 CLI 시작 경로의
성능 회귀로 판단하지 않는다.

새 hook workload의 중앙값은 3.723~3.732 ms, p95는 4.545~4.623 ms다. 한 번의 Codex
허용 측정에서 30.889 ms가 기록됐으며 다른 workload의 최대값은 7.273~11.325 ms다.
이 불리한 outlier를 포함해도 중앙값과 p95는 후보 `--version` 시작 비용과 같은 범위다.
Hook protocol 해석 비용보다 fresh process 시작 비용이 지배적이다.

기준 revision에는 hook 실행 경로가 없으므로 허용·차단 workload를 0 ms 기준과 비교하지
않는다. 제품 변경으로 지원 agent의 관찰 가능한 shell 호출마다 후보 hook 시간이
추가된다. 한국어 literal 검색을 실행 전에 `kfind`로 유도하는 이득에 비해 수 ms의
지연을 수용한다.

## Fresh-process latency

단위는 ms이며 낮을수록 좋다.

| workload | median | min | max | p95 |
| --- | ---: | ---: | ---: | ---: |
| 기준 `--version` | 3.733917 | 3.150625 | 10.618375 | 4.582416 |
| 후보 `--version` | 3.701021 | 3.197042 | 7.532958 | 4.653375 |
| 후보 Codex 허용 | 3.731750 | 3.181583 | 30.888667 | 4.623209 |
| 후보 Codex 차단 | 3.727958 | 3.188875 | 11.325125 | 4.544917 |
| 후보 Gemini 차단 | 3.723417 | 3.177625 | 7.273125 | 4.611667 |

Release binary는 기준 6,619,808 bytes, 후보 6,743,056 bytes다. 후보는
123,248 bytes, 1.86% 크다.

## 입력과 산출물

- 기준 binary SHA-256:
  `91fc16b0f9d4bec9b8f9b8e7ec3166e1b39bbbec95b9c228295fb08893374b1b`
- 후보 binary SHA-256:
  `cc36d563c94a657a26c8b8a93eba6027e5b2ca4be920775f1c118ae09a7785fb`
- Codex 허용 payload SHA-256:
  `fd55227298ccb427949b8c12e9a52b188396cee05aa9b2f57bcc23ccfadfa2f8`
- Codex 차단 payload SHA-256:
  `7bf3655f755a870052c29eabe9ec6e78ea0db7429c55ba2b768ee40f7af5e724`
- Gemini 차단 payload SHA-256:
  `60f6bba91de545b322f66cc61bd45454f992b356265451c81a296ac1408c7d88`
- Report SHA-256:
  `744cc5901fda8b06c29cc4f8dccb4d57f710a514a3778f08e3822b6ef310fe44`
- Runner SHA-256:
  `8f745b7dd957e588987d1ac6c6e6a77febb6f2173bc901ba74e068ac7fbef934`

성능 지표에는 품질 계약을 적용하지 않는다. 이 변경은 형태 검색 계획과 matcher를
바꾸지 않으므로 morphology confusion matrix를 다시 측정하지 않았다. Runner는 1,050회의
warm-up·측정 실행에서 각 agent protocol의 허용·차단 응답을 함께 검증했다.

## 재현

공식 wrapper로 저장소 공통 benchmark lock을 획득한다. 두 revision을 별도 target
directory에 `--release --locked`로 빌드한 뒤 다음 runner에 전달한다.

```console
scripts/benchmark-run.sh run --name issue-219-agent-hook -- \
  python3 tools/agent-hook-benchmark/benchmark.py \
    --baseline /path/to/baseline/release/kfind \
    --baseline-revision 33404b860e1d5251fc1766fa88102daf4971c9d6 \
    --candidate /path/to/candidate/release/kfind \
    --candidate-revision 9d7d00f73cbf8bb44770a6b220ee7569b92756fb \
    --warmups 10 \
    --runs 200 \
    --output target/benchmark/agent-hook/report.json
```
