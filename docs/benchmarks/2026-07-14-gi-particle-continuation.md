# `-기` 명사형 조사 continuation 품질·성능

- 측정일: 2026-07-14
- 기준 revision: `079c53e8d0c21ae4de8c8822c2007c88af4327f6`
- 후보 revision: `5889068592537180c185fe6ba2ddef091649e5fb`
- 환경: Linux 6.12.76/aarch64, 10 logical CPUs, Python 3.12.13, Docker
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- 기준 report SHA-256: `c1292fdf01bb15c3954ecc571adf396ef321b085d7e95a7ea2d14d022585d26d`
- 후보 report SHA-256: `d431665f9d90248a5084e6778fa37e165024e754d7daec61c80a4d8555cf63cd`

## 결론

`ending.nominalizer-gi` branch만 기존 nominal particle verifier로 전이한다. `smart`와 `token`은
`걷기가`, `걷기를`, `걷기에서도`의 조사 연쇄를 token 끝까지 소비하고, 잘못된 이형태와 격조사
중복은 거부한다. `any`는 기존 substring candidate를 유지하되 유효한 조사 연쇄의 token span을
확장한다.

held-out test의 `외우다 -> 외우기가` 1건을 추가로 복구했다. embedded/full-POS `smart`와
`token`은 각각 TP가 1 늘고 FP는 0을 유지했다. Agent `any`의 TP 479 / FP 11 / FN 21은
변하지 않았다. User persona는 TP 410 / FP 0 / FN 90에서 TP 411 / FP 0 / FN 89로 개선됐다.

새 hard-negative 4건은 embedded/full-POS `smart` 모두 FP 0이다. 여섯 profile·boundary 조합의
처리량 변화는 -5.94%~+3.46%, p95 변화는 -4.98%~+13.47%, peak RSS 변화는
-0.07%~+0.29%다. 변경 경로인 embedded/full-POS `smart` 처리량은 각각 5.94%, 3.43%
낮아졌고 p95는 13.47%, 4.30% 높아졌다. morphology benchmark에는 별도 회귀 임계가 없으므로
측정값을 그대로 기록하며 성능 불변을 주장하지 않는다.

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
| embedded | `smart` | 11,000.0 | 10,346.5 | -5.94% | 0.2368 ms | 0.2687 ms | 51.1 MiB |
| embedded | `token` | 14,898.6 | 15,414.2 | +3.46% | 0.1565 ms | 0.1487 ms | 5.2 MiB |
| embedded | `any` | 14,997.6 | 15,399.2 | +2.68% | 0.1537 ms | 0.1468 ms | 5.3 MiB |
| full-POS | `smart` | 9,354.5 | 9,033.8 | -3.43% | 0.2862 ms | 0.2985 ms | 92.1 MiB |
| full-POS | `token` | 14,479.4 | 13,853.3 | -4.32% | 0.1821 ms | 0.1891 ms | 46.5 MiB |
| full-POS | `any` | 14,360.4 | 13,859.9 | -3.49% | 0.1836 ms | 0.1929 ms | 46.5 MiB |

User persona의 처리량은 7,824.2에서 7,625.4 cases/s로 2.54% 낮아졌고 p95는 0.3807에서
0.4033 ms로 5.94% 높아졌다. Agent CLI의 100 MiB 처리량은 5,690.43에서
5,197.10 MiB/s로 8.67% 낮아졌고 Human CLI는 328.33에서 323.11 MiB/s로 1.59% 낮아졌다.
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
KFIND_MORPH_IMAGE=kfind-morph-benchmark:rebase-gi-5889 \
  KFIND_MORPH_RUNS=5 \
  scripts/benchmark-morphology.sh target/morph-rebase-gi-5889
```

기준선은 `079c53e8d0c21ae4de8c8822c2007c88af4327f6`의 detached checkout에서 같은
환경·입력·warm-up·반복 횟수로 측정했다. 외부 분석기 snapshot은 fixture, schema와 고정
버전·설정이 바뀌지 않아 갱신하지 않았다. 기준 report schema는 12, 후보는 timed 경로 밖의
Agent shadow 진단이 추가된 13이며 품질·성능 필드와 fixture는 같다.
