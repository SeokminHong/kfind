# 에이전트 세션 지침 시작 비용

- 측정일: 2026-08-29
- 기준 revision: `d3e3b2a5f9461f286cb5d2adc64a217dedd46677`
- 후보 revision: `47d1fb4cfc37cb54f724ba23a6ed480eaef3660b`
- 환경: macOS 26.4.1, Apple M1 Max, 32 GiB, arm64, Rust 1.97.0,
  Python 3.14.6
- 반복: workload별 fresh process warm-up 10회 뒤 200회 측정,
  round-robin 순서 교대

## 판정

공통 `--version` workload의 후보 중앙값은 기준보다 1.16%, p95는 0.65% 낮다.
절대 변화는 각각 -0.044 ms와 -0.027 ms이며 기존 CLI 시작 경로의 성능 회귀로
판단하지 않는다.

새 `SessionStart` workload의 중앙값은 3.833 ms, p95는 4.342 ms다. 후보
`--version`보다 각각 0.081 ms, 0.186 ms 높으며 fresh process 시작 비용과 같은
범위다. 이 hook은 지원 agent의 session 시작·재개·초기화 시점에만 실행되므로
shell tool 호출마다 반복되지 않는다.

기존 실행 전 hook workload의 중앙값은 3.763~3.798 ms, p95는
4.232~4.311 ms다. 한국어 검색 계약을 session context에 추가하는 경로가 기존
허용·차단 경로의 비용을 바꾸지 않았다.

## Fresh-process latency

단위는 ms이며 낮을수록 좋다.

| workload | median | min | max | p95 |
| --- | ---: | ---: | ---: | ---: |
| 기준 `--version` | 3.795750 | 3.331542 | 4.609167 | 4.182084 |
| 후보 `--version` | 3.751583 | 3.284625 | 4.672750 | 4.155000 |
| 후보 Codex 허용 | 3.762979 | 3.347209 | 5.297250 | 4.311291 |
| 후보 Codex 차단 | 3.777249 | 3.392208 | 5.749667 | 4.232292 |
| 후보 Gemini 차단 | 3.798458 | 3.253542 | 5.114375 | 4.244834 |
| 후보 `SessionStart` | 3.832917 | 3.446292 | 10.500125 | 4.341500 |

Release binary는 기준 6,764,608 bytes, 후보 6,783,024 bytes다. 후보는
18,416 bytes, 0.27% 크다.

## 입력과 산출물

- 기준 binary SHA-256:
  `c5e7fb7075eb6d6deda327a777b3905abf88fb360b1b106397f7c9122710d834`
- 후보 binary SHA-256:
  `5c8e109fa600a16cd09e42b2f84b17a4d5ab8bef736b498c02958c2f272b2823`
- Codex 허용 payload SHA-256:
  `fd55227298ccb427949b8c12e9a52b188396cee05aa9b2f57bcc23ccfadfa2f8`
- Codex 차단 payload SHA-256:
  `7bf3655f755a870052c29eabe9ec6e78ea0db7429c55ba2b768ee40f7af5e724`
- Gemini 차단 payload SHA-256:
  `60f6bba91de545b322f66cc61bd45454f992b356265451c81a296ac1408c7d88`
- `SessionStart` payload SHA-256:
  `9f93751a7a648627789067d2c9ffa0aa4f936750631ae13109196da75cbe7f46`
- Report SHA-256:
  `cfc29a39ee819ac4cebfbeb64c040c432e477ee9b97b5f206d1c9520f9a05074`
- Runner SHA-256:
  `522da5b0b2aa0e3201f4a752c4c31b75ec45056e9b30ae05cd44a46b3710df5b`

성능 지표에는 품질 계약을 적용하지 않는다. 이 변경은 형태 검색 계획과 matcher를
바꾸지 않으므로 morphology confusion matrix를 다시 측정하지 않았다. Runner는
1,260회의 warm-up·측정 실행에서 session context와 각 agent protocol의 허용·차단
응답을 함께 검증했다.

## 재현

공식 wrapper로 저장소 공통 benchmark lock을 획득한다. 두 revision을 별도 target
directory에 `--release --locked`로 빌드한 뒤 다음 runner에 전달한다.

```console
scripts/benchmark-run.sh run --name agent-session-instructions -- \
  python3 tools/agent-hook-benchmark/benchmark.py \
    --baseline /path/to/baseline/release/kfind \
    --baseline-revision d3e3b2a5f9461f286cb5d2adc64a217dedd46677 \
    --candidate /path/to/candidate/release/kfind \
    --candidate-revision 47d1fb4cfc37cb54f724ba23a6ed480eaef3660b \
    --warmups 10 \
    --runs 200 \
    --output target/benchmark/agent-session-instructions/report.json
```
