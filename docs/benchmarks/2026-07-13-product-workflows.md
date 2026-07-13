# 제품 workflow 형태소 벤치마크

- 측정일: 2026-07-13
- 측정 revision: `4288702`
- 환경: Linux/aarch64, 10 logical CPUs, Python 3.12.13, Docker
- 반복: 1회 warm-up 뒤 5회 측정의 중앙값

## 결론

- 에이전트 CLI는 `embedded + any + 명시적 품사`를 기준으로 본다. 주 지표는 recall과
  처리량이며 FP는 문맥 검토 대상 수로 기록한다.
- 사용자 CLI는 `full-POS + smart + 무품사`를 기준으로 본다. 주 지표는 precision, recall과
  자동 품사 계획 coverage다.
- 라이브러리 기본값은 optional resource가 없는 embedded engine이다. full-POS lexicon과
  component resource는 초기화 시간과 메모리를 감수하는 명시적 선택지다.

## 제품 profile trade-off

![제품 profile별 품질과 실제 CLI 비용](assets/product-workflows.svg)

Agent profile은 recall 95.80%와 5,570.2 MiB/s 처리량을 얻는 대신 FP 후보가 11건이다.
사람 CLI profile은 precision 99.76%와 FP 1건을 얻지만 recall은 82.00%, 처리량은
315.2 MiB/s다. 품질은 profile별 1,000-case held-out fixture, CLI 비용은 아래 고정 source
corpus에서 측정했으며 하나의 종합 점수로 합치지 않는다.

## 실제 CLI 사용 케이스 성능

![제품 사용 케이스별 CLI 및 라이브러리 비용](assets/product-use-cases.svg)

| use case | wall | throughput | peak RSS | 출력 |
| --- | ---: | ---: | ---: | --- |
| Agent: embedded + any + explicit POS | 18.0 ms | 5,570.2 MiB/s | 7.2 MiB | JSON Lines |
| Human: full-POS + smart + untagged | 317.3 ms | 315.2 MiB/s | 91.6 MiB | 기본 text |

100 MiB를 1,000개 파일에 나눈 고정 코퍼스에서 `학교`가 포함된 한 줄만 반환했다. 각 행은
독립 CLI process로 시작하며 query compile, 파일 순회, scan, verification과 출력 직렬화를
모두 포함한다. 파일시스템 cache warm-up 1회 뒤 5회 측정한 중앙값이다. 코퍼스 SHA-256은
`7692072cb7bff9261c1fa5933bde41b27e558170818eeac6d07cabdd673815ff`다.

| library resource | initialization | peak RSS |
| --- | ---: | ---: |
| embedded | 1.2 ms | 3.4 MiB |
| embedded + component | 157.1 ms | 49.1 MiB |
| full-POS | 137.9 ms | 46.0 MiB |
| full-POS + component | 292.7 ms | 87.6 MiB |

라이브러리는 검색 workload와 합산하지 않고 resource 조합별 초기화 비용을 따로 기록한다.

## fixture 단위 workflow 품질과 성능

| workflow | TP / FP / FN | precision | recall | init | cases/s | p95 | peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Agent: embedded + any + explicit POS | 479 / 11 / 21 | 97.76% | 95.80% | 1.23 ms | 14,687.1 | 0.1592 ms | 5.35 MiB |
| Human: full-POS + smart + untagged | 410 / 1 / 90 | 99.76% | 82.00% | 443.97 ms | 7,312.6 | 0.4107 ms | 92.09 MiB |

에이전트 workflow의 FP 11건은 strict boundary 오류 수보다 후속 문맥 검토 후보 수로 해석한다.
사람용 workflow에서는 기대 품사가 자동 plan에 포함된 비율이 96.4%(482/500), literal
fallback은 1.0%(5/500)였다.

이 표의 throughput은 CLI 파일 검색이 아니라 초기화된 runner에서 query와 문장을 평가한
속도다. 두 workflow는 각각 explicit-POS fixture와 untagged fixture를 사용한다. positive gold span은
같지만 negative의 의미가 다르므로 두 행의 F1을 합쳐 순위를 매기지 않는다. 두 fixture는 각각
1,000건이며 positive와 negative가 500건씩이다.

## profile 진단

전체 lexicon/boundary 행렬은 제품 기본값을 정하는 표가 아니라 원인 진단 자료다.

![lexicon profile 및 경계별 품질](assets/product-boundary-quality.svg)

![lexicon profile 및 경계별 성능](assets/product-boundary-performance.svg)

명시적 품사에서 `any`는 embedded와 full-POS가 같은 recall 95.8%를 냈다. full-POS를 읽어도
추가 품질 이득이 없으므로 에이전트 경로에서는 embedded가 맞다. `smart`는 component
resource를 읽고 경계를 검증하므로 더 느리지만 FP를 1건으로 줄인다.

무품사 입력에서는 full-POS가 query의 품사 후보를 보강한다. embedded `smart`의 recall은
63.0%이고 full-POS `smart`는 82.0%다.

![무품사 사용자 검색 품질](assets/product-human-untagged-quality.svg)

## 외부 분석기 동일 task 비교

Agent와 외부 분석기 4종은 같은 1,000-case explicit-POS fixture와 같은 query 평가
workload로 비교한다. Agent는 현재 run, 외부 분석기는 고정 버전·설정으로 갱신한 품질·성능
스냅샷이다. 모든 성능값은 별도 process에서 warm-up 1회 뒤 5회 측정한 중앙값이다.

![Agent와 외부 형태소 분석기의 품질·성능 비교](assets/product-external-comparison.svg)

| backend | 고정 버전·설정 | TP / FP / FN | precision | recall | F1 | init | cases/s | p95 | peak RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Agent | embedded + any | 479 / 11 / 21 | 97.76% | 95.80% | 96.77% | 1.23 ms | 14,687.1 | 0.1592 ms | 5.3 MiB |
| Kiwi | 0.23.2, model 0.23.0 | 426 / 0 / 74 | 100.00% | 85.20% | 92.01% | 1,720.42 ms | 1,672.0 | 1.1904 ms | 528.2 MiB |
| Lindera | 4.0.0, embedded-ko-dic | 393 / 0 / 107 | 100.00% | 78.60% | 88.02% | 30.11 ms | 15,609.1 | 0.1113 ms | 193.1 MiB |
| MeCab-ko | 1.0.2, dictionary 1.0.0 | 403 / 0 / 97 | 100.00% | 80.60% | 89.26% | 0.25 ms | 10,789.7 | 0.1940 ms | 102.8 MiB |
| KOMORAN | 3.3.9, FULL | 406 / 0 / 94 | 100.00% | 81.20% | 89.62% | 1,158.93 ms | 1,669.4 | 1.2370 ms | 686.6 MiB |

이 표에서는 Agent가 가장 높은 recall과 F1을 내면서 Lindera 다음의 처리량을 보인다. 외부
분석기 4종은 FP가 없지만 recall이 78.60~85.20%다. 사람용 `full-POS + smart`는 무품사
negative 정의가 달라 순위에서 제외하고 위 제품 workflow에서 따로 측정한다.

기본 benchmark는 kfind만 다시 실행하고 외부 품질·성능은 fixture digest와 버전·설정이 맞는
스냅샷에서 읽는다. test fixture, 정규화 adapter, 성능 schema 또는 고정 버전·설정이 바뀔
때만 다음 명령으로 외부 결과를 갱신한다.

```console
scripts/refresh-morph-baselines.sh
```

일반 측정과 차트 생성은 다음 명령을 사용한다.

```console
scripts/benchmark-morphology.sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets
```

explicit-POS fixture SHA-256은
`933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`, untagged fixture
SHA-256은 `94ccd70a093ee7af8435371b2ffdb81534ec97e29ada705ea72c940938d0c592`다.
