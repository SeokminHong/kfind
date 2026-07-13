# `ending.connective-ji` 위치 근거

- 측정일: 2026-07-14
- 기준 revision: `3baeee0`
- 후보 revision: `860c864`
- 환경: Linux/aarch64, 10 logical CPUs, 7.7 GiB memory, Python 3.12.13, Docker 29.6.1
- Rust: 1.97.0
- 반복: fresh process 1회 warm-up 뒤 5회 측정의 중앙값
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- 기준 hard-negative fixture: `a08964bec7a7d421d12cc76e94387b206c98167538e8986d52fe3def303e9b3b`
- 후보 hard-negative fixture: `d75b6482ecd6c4ba43aef78efe8f806a2502f65a886c9d00c20b677019f245d0`
- 기준 report SHA-256: `3d9b376e52d6925ce9acafef95e6904cfb8cfc68ee650e99bbe27d9c28623172`
- 후보 report SHA-256: `2f219b6451750d4569abb416085e4ec803974e67c3e2ddd1b1d46117e3d0a498`

## 결론

development의 `ending.connective-ji` 4건은 any token이 gold의 `left-edge`에 있는 3건과
`right-edge`에 있는 1건이다. `internal`은 없다. 위치 분류는 기존 span에서 계산하므로 timed
구간과 matcher 결과를 바꾸지 않는다.

오른쪽 유형의 candidate `주지`와 같은 표면형인 명사 `주지`를 hard-negative로 추가했다.
embedded와 full-POS `smart`가 모두 이 case를 찾으므로 오른쪽 유형은 표면형과 위치만으로
복구할 수 없다. 왼쪽 유형은 같은 표면형 hard-negative를 확보하지 못했으므로 계측 상태를
유지한다. 두 유형 모두 제품 후보로 열지 않는다.

## Development 위치 분류

| 위치 | cases |
| --- | ---: |
| `left-edge` | 3 |
| `right-edge` | 1 |
| `internal` | 0 |

| query/POS | gold surface | any token | 위치 |
| --- | --- | --- | --- |
| `소리치다/verb` | `소리치지요` | `소리치지` | `left-edge` |
| `주다/verb` | `심어주지` | `주지` | `right-edge` |
| `없다/adjective` | `없지는` | `없지` | `left-edge` |
| `낚다/verb` | `낚지못한다` | `낚지` | `left-edge` |

## 품질·성능

test와 development 품질은 기준선과 같다. test full-POS `smart`는 TP 414 / FP 0 / FN 86,
development는 TP 436 / FP 2 / FN 64다.

hard-negative는 14건에서 15건으로 늘었다. 새 `주지` case가 기존 matcher의 FP이므로 embedded와
full-POS 모두 FP 3 → 4, TN 11 유지, hard-negative precision 78.57% → 73.33%다. 이는 fixture가
새 ambiguity를 드러낸 결과이며 제품 matcher의 회귀가 아니다.

| profile | 기준 init | 후보 init | 기준 cases/s | 후보 cases/s | 증감 | 기준 p95 | 후보 p95 | 증감 | 후보 RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| embedded `smart` | 0.2802 s | 0.2798 s | 14,333.7 | 14,584.2 | +1.75% | 0.1507 ms | 0.1449 ms | -3.85% | 51.0 MiB |
| full-POS `smart` | 0.4235 s | 0.4222 s | 13,438.3 | 13,551.4 | +0.84% | 0.1840 ms | 0.1801 ms | -2.12% | 92.1 MiB |

full-POS 처리량 범위는 기준 13,168.3~13,490.9 cases/s, 후보
12,891.1~13,564.7 cases/s다. p95 범위는 기준 0.1823~0.1884 ms, 후보
0.1794~0.1859 ms로 겹친다. embedded도 양쪽 범위가 겹치므로 성능 회귀는 없다.

## 재현

```console
KFIND_MORPH_IMAGE=kfind-morph-benchmark:connective-ji-main-3baeee0 \
  scripts/benchmark-morphology.sh target/morph-benchmark-main-3baeee0
KFIND_MORPH_IMAGE=kfind-morph-benchmark:connective-ji-candidate-860c864 \
  scripts/benchmark-morphology.sh target/morph-benchmark-candidate-860c864
```

외부 분석기 snapshot은 test fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지 않았다.
