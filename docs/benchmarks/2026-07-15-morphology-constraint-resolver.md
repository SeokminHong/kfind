# 형태 구조 제약 resolver shadow 결과

## 결론

비용을 읽지 않는 `QueryMorphPattern`, `TokenAnalysisGraph`와 `ConstraintResolver`를 구현하고 development, 고정 test와 hard-negative 전체 집합에서 shadow 평가했다. 구현과 계측은 성공했지만 `opaque`, `transparent`, `explicit` 중 제품 채택 조건을 통과한 profile이 없으므로 제품 전환은 실패다. 현재 matcher의 lexical context registry, `ContextRequirement`와 1,500 비용 마진은 유지한다.

## 구현

query compiler는 같은 branch의 fine POS와 lexical identity를 `QueryMorphPattern` 합집합으로 만든다. resolver는 source whole, source component, runtime composition, opaque expression과 unknown 경로를 구분하고 query를 지지하는 완전한 known 경로가 하나라도 있으면 다른 분석의 존재만으로 이를 부정하지 않는다. word cost는 proof 진단에만 남고 수용 여부에는 사용하지 않으며 connection cost는 완전한 경로의 연결 가능성만 검증한다.

제품 matcher는 resolver verdict를 소비하지 않는다. 기존 제품 판정과 세 resolver profile을 같은 candidate에서 병렬 계산하고 resolver 시간은 제품 성능 측정 구간에서 제외했다.

## 입력과 재현

| 항목 | 값 |
| --- | --- |
| revision | `d1f1235b0b7d2298b9822aa6ebedd6f3111d5c38` |
| 명령 | `KFIND_MORPH_RUNS=5 scripts/benchmark-morphology.sh target/morph-constraint-shadow-full` |
| 환경 | Linux 6.12.76 linuxkit, aarch64, logical CPU 10, memory 7.65 GiB, Python 3.12.13 |
| 고정 test fixture SHA-256 | `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff` |
| development fixture SHA-256 | `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c` |
| hard-negative fixture SHA-256 | `cb8634491cba65916c9af510c50f909eaddfd9bb89935598875e134a01cbce99` |
| Human untagged fixture SHA-256 | `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592` |
| graph resource SHA-256 | `099e7ad977bff797d279a3c681818f7d21faf67ff7d683cf443db0509e3ac26f` |
| `report.json` SHA-256 | `e8616f967a2a43034c91b90603a139b56b9ca2f673b097a18e9fbf6b25c84a04` |
| `report.md` SHA-256 | `26597e0319effaff4109c427ec89c5a835cf1d83fb1fec1cdb94e101d73a3d8c` |

morphology benchmark 계약에 따라 fresh process에서 warm-up 1회 후 5회 측정하고 median과 min/max를 기록했다.

## 품질 결과

### 고정 test

| 판정 | TP | FP | TN | FN | precision | recall | 제품 대비 변경 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 466 | 0 | 500 | 34 | 100.00% | 93.20% | - |
| `opaque` | 344 | 0 | 500 | 156 | 100.00% | 68.80% | 122 |
| `transparent` | 444 | 8 | 492 | 56 | 98.23% | 88.80% | 60 |
| `explicit` | 344 | 0 | 500 | 156 | 100.00% | 68.80% | 122 |

### Development

| 판정 | TP | FP | TN | FN | precision | recall | 제품 대비 변경 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 현재 제품 | 475 | 2 | 498 | 25 | 99.58% | 95.00% | - |
| `opaque` | 363 | 2 | 498 | 137 | 99.45% | 72.60% | 112 |
| `transparent` | 452 | 11 | 489 | 48 | 97.62% | 90.40% | 44 |
| `explicit` | 363 | 2 | 498 | 137 | 99.45% | 72.60% | 112 |

### Hard-negative

| 판정 | FP | TN | 제품 대비 변경 |
| --- | ---: | ---: | ---: |
| 현재 제품 | 4 | 18 | - |
| `opaque` | 7 | 15 | 3 |
| `transparent` | 21 | 1 | 17 |
| `explicit` | 7 | 15 | 3 |

`explicit` profile은 현재 query capability가 component 노출을 선언하지 않으므로 `opaque`와 같다. `opaque`는 component positive를 대량으로 잃고 `transparent`는 고정 test, development와 hard-negative 모두에서 새 FP를 만든다.

## 구조만으로 닫히지 않는 경계

hard-negative에는 형태 분석 그래프 안에 query를 지지하는 완전한 경로가 실제로 존재하지만 문장의 의도는 다른 사례가 남는다. `새/noun`과 `새 기능`, `걷다/verb`와 `전화를 걸었다`, `들다/verb`와 `음악을 들었다`, `주다/verb`와 `주지 스님`은 같은 표면의 다른 품사·표제어·의미가 공존한다. `매일`의 copular·반복 문맥 일부는 인접 token 제약으로 좁힐 수 있지만 이것만으로 같은 표면의 다른 표제어와 의미 충돌 전체를 해결할 수 없다.

이 충돌을 자동으로 나누려면 문맥 의미, collocation, 통계 모델 또는 별도 정책이 필요하다. 이는 형태 구조 graph만 사용하는 no-heuristic resolver의 정보 범위를 벗어나므로 surface 예외, 새 비용 임계값이나 임의의 분석 우선순위로 숨기지 않는다.

## 성능

| profile | 초기화 median (min-max) | cases/s median (min-max) | p95 ms median (min-max) | RSS MiB median (min-max) |
| --- | ---: | ---: | ---: | ---: |
| embedded `smart` | 0.2892 (0.2838-0.3262) | 11,978.9 (11,506.2-12,051.6) | 0.1713 (0.1703-0.1803) | 51.0 (51.0-51.0) |
| full-POS `smart` | 0.4263 (0.4239-0.4289) | 10,532.7 (9,990.5-10,638.9) | 0.2418 (0.2398-0.2529) | 92.5 (92.5-92.5) |

이 수치는 제품 matcher의 현재 성능 기준선이다. resolver는 shadow 전용이며 계측 시간이 측정 구간 밖에 있으므로 resolver를 제품에 연결했을 때의 비용을 나타내지 않는다.

## 결정

no-heuristic resolver core, graph resource와 benchmark evidence는 후속 연구용 stack에 유지한다. 자동 제품 전환, lexical context registry 제거와 1,500 비용 마진 제거는 수행하지 않는다. 다음 FN 작업은 현재 제품 계약을 유지한 채 별도 구조 근거나 명시적 제품 정책이 있는 범위에서 진행한다.
