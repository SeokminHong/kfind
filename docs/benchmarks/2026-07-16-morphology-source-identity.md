# 형태 구조 source identity와 계층적 context fact

## 결론

source lexical identity와 complete-path alignment를 보강해 고정 test의 `possible-analysis`를 TP 461 / FP 5 / FN 39에서 TP 480 / FP 5 / FN 20으로 개선했다. 비용 임계값, lexical context registry와 제품 verifier를 호출하지 않는 독립 evaluator의 품질이다.

성능은 query-directed trace traversal, runtime support streaming, prefix trie, graph string-table identity와 계층적 context fact로 닫았다. 최종 5회 median은 evaluator 9,754.1 cases/s, p95 0.2645ms, RSS 104.8MiB이고 같은 revision의 product control 대비 처리량 -5.6%, p95 +6.7%, RSS +13.2%로 세 gate를 모두 통과했다.

candidate coverage와 형태 분석만으로 남는 `CompoundExposure` 및 lexical meaning ambiguity는 해결되지 않았다. `possible-analysis`는 제품 positive 6건을 잃고 다른 positive 20건과 negative 5건을 추가로 수용하며 `unambiguous-analysis`는 제품 positive를 다수 잃으므로 제품 matcher 전환은 보류한다.

## 구조

query lexical form은 lexical automaton과 source graph를 교차하는 query-directed traversal로 canonical trace만 준비한다. pattern별 trace는 prefix trie로 컴파일하고 compact decision은 source graph를 순회하면서 trie 상태를 증분 전이하므로 이미 수용한 prefix, 중간 path 목록과 path-pattern 곱을 다시 검사하지 않는다.

graph decoder는 schema 4 string table의 surface와 POS identity를 analysis와 component에 보존한다. 같은 `MorphologyGraphResource`를 공유하는 source graph와 query trace는 compact 경로에서 정수 identity를 비교하고 문자열은 진단 proof와 외부 표현에만 사용한다.

current token의 copular split은 `PreparedTokenSummary`에 DAG fact로 memoize한다. adjacent token의 `VCN+EC` 완전 sequence와 `NNB/NNBC` 시작 조건은 `(node, automaton state)`만 전파해 판정하며 전체 witness path는 diagnostic 요청에서만 만든다.

runtime lexical support는 query core를 덮는 source path를 제자리에서 확장하고 terminal에서만 canonical support를 만든다. 같은 final source node 안에서 끝나는 짧은 trace와 긴 trace를 모두 보존하며 compact decision과 diagnostic resolution의 outcome과 policy projection은 동일하다.

## 입력과 재현

| 항목 | 값 |
| --- | --- |
| candidate와 product control revision | `98d696236d8516abe447eb47ba733d56018405eb` |
| 명령 | `KFIND_MORPH_RUNS=5 ./scripts/benchmark-morphology.sh target/morph-hierarchical-context-final` |
| 환경 | Linux 6.12.76 linuxkit, aarch64, logical CPU 10, Rust 1.97.0, Python 3.12.13 |
| 측정 | fresh process warm-up 1회 후 5회, median과 min/max |
| 고정 test fixture SHA-256 | `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff` |
| development fixture SHA-256 | `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c` |
| hard-negative fixture SHA-256 | `cb8634491cba65916c9af510c50f909eaddfd9bb89935598875e134a01cbce99` |
| graph resource SHA-256 | `c7b0c4b1f01c4d2e60f453ae63f4f24dc6af132599dfd0152cefa7380691426b` |
| 생성 `report.json` SHA-256 | `bfa07358f9e9c515daf6faf9b2ca9a37c1248b6dc21796d145ddb9f98b0893da` |
| 생성 `report.md` SHA-256 | `713f28bff6cd3e4a2f7789b4453136d59371f33796b82466154ddaba72e94525` |

## 품질

독립 candidate coverage는 고정 test positive 500건 중 494건인 98.8%이고 development positive 500건 중 489건인 97.8%다.

### 고정 test

| 정책 | TP | FP | TN | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 466 | 0 | 500 | 34 | 100.00% | 93.20% |
| `whole` | 102 | 0 | 500 | 398 | 100.00% | 20.40% |
| `explicit-component` | 476 | 3 | 497 | 24 | 99.37% | 95.20% |
| `possible-analysis` | 480 | 5 | 495 | 20 | 98.97% | 96.00% |
| `unambiguous-analysis` | 294 | 0 | 500 | 206 | 100.00% | 58.80% |

### Development

| 정책 | TP | FP | TN | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 475 | 2 | 498 | 25 | 99.58% | 95.00% |
| `whole` | 91 | 1 | 499 | 409 | 98.91% | 18.20% |
| `explicit-component` | 470 | 7 | 493 | 30 | 98.53% | 94.00% |
| `possible-analysis` | 470 | 7 | 493 | 30 | 98.53% | 94.00% |
| `unambiguous-analysis` | 300 | 0 | 500 | 200 | 100.00% | 60.00% |

### Hard-negative

| 정책 | FP | TN |
| --- | ---: | ---: |
| 현재 제품 | 4 | 18 |
| `whole` | 2 | 20 |
| `explicit-component` | 8 | 14 |
| `possible-analysis` | 9 | 13 |
| `unambiguous-analysis` | 2 | 20 |

## 성능

| 경로 | 초기화 median | cases/s median (min-max) | p95 ms median (min-max) | RSS MiB median (min-max) |
| --- | ---: | ---: | ---: | ---: |
| 현재 제품 full-POS `smart` | 0.4270 | 10,335.0 (6,423.6-10,531.8) | 0.2479 (0.2414-0.4347) | 92.6 (92.6-92.6) |
| 독립 constraint evaluator | 0.9124 | 9,754.1 (9,623.3-10,008.5) | 0.2645 (0.2615-0.2662) | 104.8 (104.8-104.9) |

evaluator median은 product control 대비 처리량 -5.6%, p95 +6.7%, RSS +13.2%다. product control 한 회에 처리량 6,423.6 cases/s와 p95 0.4347ms의 불리한 outlier가 있었으며 계약대로 median을 gate에 사용하고 전체 범위를 누락하지 않았다.

evaluator의 median stage 시간은 compile 0.0588초, candidate enumeration 0.0062초, graph preparation 0.0088초, decision 0.0282초, policy 0.0002초다. query compile이 가장 큰 단계이고 계층적 context fact 전환 뒤 decision은 product 대비 성능 여유를 확보했다.

초기화 0.9124초는 제품 0.4270초보다 2.1배 크다. steady-state 채택 gate에는 포함되지 않지만 제품 통합 전 graph resource mmap, lazy section validation과 query compile 공유로 줄여야 한다.

## 결정

source identity 부족과 compact 성능은 구조적으로 해소할 수 있다. surface 예외와 비용 조정 대신 canonical query trace, resource identity, DAG reachability와 context automaton을 계층으로 분리하면 같은 품질 근거를 유지하면서 성능 gate를 통과한다.

현재 stack은 장기 실험으로 유지한다. 다음 단계는 query compiler의 선언형 anchor IR을 독립 enumerator가 실행하도록 해 고정 test candidate coverage 미달 6건을 먼저 닫고, 이후 `SupportedAnalysisSet`을 입력으로 받으며 morphology hard constraint를 우회하지 않는 `AmbiguityResolver`로 고정 test의 제품 TP 466 / FP 0과 development TP 475 / FP 2를 보존하는 것이다.
