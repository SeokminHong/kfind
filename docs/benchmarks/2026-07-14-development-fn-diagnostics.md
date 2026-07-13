# Development false negative 진단

- 측정일: 2026-07-14
- 기준 revision: `b18f0c9`
- 후보 revision: `534d0ae`
- 환경: Linux/aarch64, 10 logical CPUs, 7.7 GiB memory, Python 3.12.13, Docker 29.6.1
- Rust: 1.97.0
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture: `a08964bec7a7d421d12cc76e94387b206c98167538e8986d52fe3def303e9b3b`
- 기준 report SHA-256: `cec42e387535bc74ec33966f450c957235de8e269af01ef220c49287749252a6`
- 후보 report SHA-256: `5074726cd1dfc5321615775cb1ed8ba5f0662858f7dc7033e7528a5f4521e98d`

## 결론

report schema 14는 `boundary=any`에서 gold span과 겹친 match의 core·token byte span과
origin별 analysis index·rule path를 failure evidence에 보존한다. Markdown은 development
full-POS positive false negative를 원인과 품사로 집계하고 verb·adjective
`boundary-rejected` 20건을 모두 표시한다. 계측은 성능 측정 구간 밖에서 실행하며 matcher와
fixture는 바꾸지 않는다.

development full-POS `smart`는 TP 436 / FP 2 / FN 64다. FN 중 `boundary-rejected`가
44건이고 adjective 6건과 verb 14건이 predicate slice다. 20건 모두 any-boundary span과
rule path를 가졌다. 가장 많이 반복된 단일 path는 `ending.connective-ji` 4건이지만,
gold 안의 any token 위치가 서로 달라 제품 continuation 후보로 바로 승격하지 않는다.

## Development FN 분류

| primary cause | POS | cases |
| --- | --- | ---: |
| `boundary-rejected` | adjective | 6 |
| `boundary-rejected` | determiner | 4 |
| `boundary-rejected` | noun | 8 |
| `boundary-rejected` | numeral | 6 |
| `boundary-rejected` | pronoun | 6 |
| `boundary-rejected` | verb | 14 |
| `lexicon-missing` | adjective | 2 |
| `lexicon-missing` | verb | 1 |
| `span-mismatch` | numeral | 1 |
| `span-mismatch` | verb | 2 |
| `surface-missing` | adjective | 9 |
| `surface-missing` | noun | 2 |
| `surface-missing` | numeral | 1 |
| `surface-missing` | verb | 2 |

Predicate slice의 case별 rule path는 다음과 같다. 같은 case의 중복 origin은 한 번만 센다.

| rule path | cases |
| --- | ---: |
| `ending.connective-ji` | 4 |
| `ending.past-adnominal` | 3 |
| `ending.future-adnominal` | 2 |
| `contraction.identical-vowel -> ending.aoeo` | 2 |
| `ending.past -> ending.future-adnominal` | 1 |
| `ending.final-da` | 1 |
| `ending.connective-ni` | 1 |
| `ending.connective-go` | 1 |
| `ending.aoeo` | 1 |
| `contraction.u-eo -> ending.aoeo` | 1 |
| `contraction.i-eo -> ending.past -> ending.final-da` | 1 |
| `contraction.i-eo -> ending.aoeo` | 1 |
| `contraction.eu-drop -> ending.aoeo` | 1 |

## 품질·성능

test, development와 hard-negative의 모든 품질 결과는 기준선과 같다. test full-POS `smart`는
TP 414 / FP 0 / FN 86, development는 TP 436 / FP 2 / FN 64다. hard-negative 14건의 기존
FP 3건도 바뀌지 않았다.

| profile | 기준 cases/s | 후보 cases/s | 증감 | 기준 p95 | 후보 p95 | 증감 | 후보 RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| embedded `smart` | 14,605.8 | 14,619.8 | +0.10% | 0.1459 ms | 0.1467 ms | +0.55% | 51.0 MiB |
| full-POS `smart` | 13,553.9 | 13,534.9 | -0.14% | 0.1837 ms | 0.1837 ms | 0.00% | 92.1 MiB |

full-POS 처리량 범위는 기준 12,847.7~13,561.9 cases/s, 후보
12,934.4~13,556.8 cases/s다. p95 범위는 기준 0.1824~0.1897 ms, 후보
0.1790~0.1966 ms로 겹친다. 계측은 timed 구간 밖에 있고 품질도 같으므로 성능 회귀는 없다.

## 재현

```console
KFIND_MORPH_IMAGE=kfind-morph-benchmark:dev-fn-main-b18f \
  scripts/benchmark-morphology.sh target/morph-benchmark-main-b18f
KFIND_MORPH_IMAGE=kfind-morph-benchmark:dev-fn-candidate-534d \
  scripts/benchmark-morphology.sh target/morph-benchmark-candidate-534d
```

외부 분석기 snapshot은 fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
