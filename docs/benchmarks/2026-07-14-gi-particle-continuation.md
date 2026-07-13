# `-기` 명사형 조사 continuation 품질·성능

- 측정일: 2026-07-14
- 기준 revision: `4c59fe7029a364d6a26ead830e04bfc7f1e3c40d`
- 후보 revision: `7d6a5f05393ae8a36d7a948e02e4af3f668d2f10`
- 환경: Linux 6.12.76/aarch64, 10 logical CPUs, Python 3.12.13, Docker
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- 기준 report SHA-256: `e9560ec6e0e0d135ebe9ba97696842cc17cdb06f78becc360c2550b142dcc3c5`
- 후보 report SHA-256: `ffb207e985ddd1d5baf7af9513585bba1648bca100965814e5e90c41ec1beee0`

## 결론

`ending.nominalizer-gi` branch만 기존 nominal particle verifier로 전이한다. `smart`와 `token`은
`걷기가`, `걷기를`, `걷기에서도`의 조사 연쇄를 token 끝까지 소비하고, 잘못된 이형태와 격조사
중복은 거부한다. `any`는 기존 substring candidate를 유지하되 유효한 조사 연쇄의 token span을
확장한다.

held-out test의 `외우다 -> 외우기가` 1건을 추가로 복구했다. embedded/full-POS `smart`와
`token`은 각각 TP가 1 늘고 FP는 0을 유지했다. Agent `any`의 TP 479 / FP 11 / FN 21은
변하지 않았다. User persona는 TP 410 / FP 0 / FN 90에서 TP 411 / FP 0 / FN 89로 개선됐다.

새 hard-negative 4건은 embedded/full-POS `smart` 모두 FP 0이다. 여섯 profile·boundary 조합의
처리량 변화는 -0.07%~+2.08%, p95 변화는 -3.62%~+3.12%, peak RSS 변화는
0.00%~+0.27%다.

## 품질

| profile | boundary | 기준 TP / FP / FN | 후보 TP / FP / FN | 기준 recall | 후보 recall |
| --- | --- | ---: | ---: | ---: | ---: |
| embedded | `smart` | 408 / 0 / 92 | 409 / 0 / 91 | 81.6% | 81.8% |
| full-POS | `smart` | 413 / 0 / 87 | 414 / 0 / 86 | 82.6% | 82.8% |
| embedded/full-POS | `token` | 354 / 0 / 146 | 355 / 0 / 145 | 70.8% | 71.0% |
| embedded/full-POS | `any` | 479 / 11 / 21 | 479 / 11 / 21 | 95.8% | 95.8% |

고정 morphology fixture 457건과 reference differential이 모두 통과했다. positive는 `걷기가`,
`걷기를`, `걷기에서도`, negative는 `걷기이`, `걷기을`, `걷기으로`, `걷기가를`이다. 유효한
결과 provenance에는 `ending.nominalizer-gi` 뒤에 소비한 `particle.*` rule path가 남는다.

## 성능

| profile | boundary | 기준 cases/s | 후보 cases/s | 변화 | 기준 p95 | 후보 p95 | 후보 RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| embedded | `smart` | 10,357.3 | 10,487.1 | +1.25% | 0.2631 ms | 0.2713 ms | 51.1 MiB |
| embedded | `token` | 14,559.8 | 14,862.0 | +2.08% | 0.1615 ms | 0.1578 ms | 5.2 MiB |
| embedded | `any` | 14,753.3 | 15,024.9 | +1.84% | 0.1583 ms | 0.1533 ms | 5.3 MiB |
| full-POS | `smart` | 8,788.1 | 8,845.0 | +0.65% | 0.3258 ms | 0.3140 ms | 92.1 MiB |
| full-POS | `token` | 12,845.8 | 12,972.4 | +0.99% | 0.2061 ms | 0.2051 ms | 46.5 MiB |
| full-POS | `any` | 13,388.1 | 13,378.3 | -0.07% | 0.2028 ms | 0.1973 ms | 46.5 MiB |

User persona의 처리량은 7,361.4에서 7,197.1 cases/s로 2.23% 낮아졌고 p95는 0.4250에서
0.4185 ms로 1.53% 낮아졌다. Agent CLI의 100 MiB 처리량은 5,688.27에서
5,524.14 MiB/s로 2.89% 낮아졌고 Human CLI는 314.64에서 316.57 MiB/s로 0.61% 높아졌다.
CLI query `학교`는 변경한 `-기` branch를 실행하지 않으므로 이 차이를 verifier 비용으로
귀속하지 않는다.

lexicon, morphology와 component artifact SHA-256은 기준선과 같다.

## Hard-negative

| case | 기대 | embedded/full-POS `smart` |
| --- | --- | --- |
| `걷기이 어렵다.` | 잘못된 주격 이형태 | no match |
| `걷기을 권했다.` | 잘못된 목적격 이형태 | no match |
| `걷기으로 충분하다.` | 잘못된 방향격 이형태 | no match |
| `걷기가를 권했다.` | 격조사 중복 | no match |

`any`는 substring candidate를 제거하지 않는 기존 계약 때문에 이 네 문장을 candidate로
유지한다. 제품 Agent 품질 지표와 shadow 진단은 이 동작을 변경하지 않는다.

## 재현

```console
KFIND_MORPH_IMAGE=kfind-morph-benchmark:agent-gi-final \
  KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/morph-agent-gi-final
```

기준선은 `4c59fe7029a364d6a26ead830e04bfc7f1e3c40d`의 별도 detached worktree에서 같은
환경·입력·warm-up·반복 횟수로 측정했다. 외부 분석기 snapshot은 fixture, schema와 고정
버전·설정이 바뀌지 않아 갱신하지 않았다.
