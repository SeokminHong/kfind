# Agent precision shadow 판정

측정일: 2026-07-14

구현 기준: `7d140087cb35170b81d27e1d314099a718a9453b`

## 결론

bounded local lattice의 완전 경로 존재 여부만으로는 Agent precision을 개선할 수 없다.
development의 현재 `embedded + any` TP 484를 보존하는 `include-path` 투영은 FP 15를 그대로
유지한다. FP를 0으로 만드는 `include-only` 투영은 TP가 10으로 감소한다. 제품 `any` 정책과
resource 초기화 계약은 변경하지 않는다.

Korean-Kaist·KSL dev 3,546문장의 실제 지정사 token과 겹치는 `이다` candidate 1,174개는 모두
include와 exclude 완전 경로가 함께 존재했다. exclude 경로가 없고 지정사 split 경로만 있는
자연 문장 최소 대조는 0건이므로 지정사 문맥 복구를 구현하지 않는다.

## 측정 계약

```console
docker run --rm --network none \
  --entrypoint /usr/local/bin/morph-benchmark-runner \
  kfind-morph-benchmark:agent-shadow \
  agent-shadow CASES.jsonl OUTPUT.json
```

- 환경: Linux 6.12.76 aarch64, 10 logical CPUs, Python 3.12.13
- morphology resource SHA-256:
  `50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23`
- development fixture SHA-256:
  `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture SHA-256:
  `81a7339d8eb6cb116967581578fafa237d6d98d7d3d852ef572a029474cdda81`
- development shadow SHA-256:
  `0e4e87a3a7e1b3216e9734ed656e8714095da2730d773c1b5a69cddc1cc79ce8`
- hard-negative shadow SHA-256:
  `832dff20e67783b1217a0bbb4206328ce51594f4fe10b2fc25fdf1c26625a94f`
- 계측은 기존 timed evaluation 뒤 별도 process에서 실행하며 제품 match와 성능 수치에 포함하지
  않는다.
- 규칙 판정에는 development와 hard-negative만 사용했다. held-out test 결과는 규칙 선택에
  사용하지 않았다.

## 품질 투영

| fixture | 정책 | TP | FP | FN | precision | recall | F1 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| development | 현재 `any` | 484 | 15 | 16 | 96.99% | 96.80% | 96.90% |
| development | include 경로 존재 | 444 | 15 | 56 | 96.73% | 88.80% | 92.60% |
| development | include 경로만 존재 | 10 | 0 | 490 | 100.00% | 2.00% | 3.92% |
| hard-negative | 현재 `any` | 0 | 10 | 0 | 0.00% | n/a | n/a |
| hard-negative | include 경로 존재 | 0 | 8 | 0 | 0.00% | n/a | n/a |
| hard-negative | include 경로만 존재 | 0 | 0 | 0 | n/a | n/a | n/a |

development candidate 중 gold span과 겹치는 484개는 include/exclude 434개, include-only 10개,
exclude-only 40개다. negative case의 match 18개는 모두 include/exclude였으며 case 기준 FP는 15개다.
경로 비용 우열은 투영에 사용하지 않았다.

## Gate

| 조건 | 결과 | 판정 |
| --- | --- | --- |
| 기존 Agent TP 484 보존 | include-path TP 444 | 실패 |
| Agent FP 15 미만 | include-path FP 15 | 실패 |
| hard-negative 새 FP 0 | include-path FP 8 | 실패 |
| `any` 밖의 span 0 | candidate 부분집합만 평가 | 통과 |

필수 조건이 실패했으므로 held-out 제품 판정과 제품 matcher 변경을 진행하지 않는다.
