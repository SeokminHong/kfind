# 형태 분석 그래프 schema 2 projection과 비용

## 결론

정책 중립 형태 분석 그래프 schema 2는 full schema 3의 773,105개 surface, 815,725개 source analysis, 684,999개 expression component, 10,292,646개 연결 비용과 unknown 정의를 손실 없이 투영한다. exact와 common-prefix workload도 full·compact·graph가 같은 match, analysis와 checksum을 반환했다.

구조 보존은 현재 압축·로딩 효율을 악화시킨다. graph artifact는 92,758,442 bytes로 full의 128.54%, compact의 193.81%이며 resident cold 기준 초기화는 full보다 117.61%, exact lookup은 32.38%, prefix lookup은 33.10%, peak RSS는 27.15% 높다. 이 artifact는 제품 matcher가 아직 읽지 않는 실험 근거 계층이며, resolver 전환 전에 저장 구조와 소유 모델을 별도 최적화해야 한다.

## 측정 계약

| 항목 | 값 |
| --- | --- |
| candidate revision | `42668d5fb9903422dc5aecb3d6f40574c9489117` |
| source | `mecab-ko-dic-2.1.1-20180720` |
| source SHA-256 | `fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330` |
| 명령 | `scripts/benchmark-morph-index.sh target/morph-analysis-graph-index-benchmark-42668d5` |
| 환경 | Apple Silicon arm64, macOS 26.4.1, Rust 1.97.0 |
| workload | exact 9,912개, common-prefix 9,912개 |
| 반복 | query당 100회 |

`cold`는 artifact 생성 뒤 첫 probe process이고 `warm`은 바로 다음 별도 process다. 운영체제 page cache를 비우지 않았으므로 물리 디스크 cold-cache로 해석하지 않는다. graph loader는 검증된 byte buffer를 소유하므로 graph는 resident storage만 측정했으며, full·compact의 mmap 결과와 섞어 비교하지 않았다.

## 구조 projection

| 항목 | 수 |
| --- | ---: |
| surface | 773,105 |
| source analysis | 815,725 |
| expression component | 684,999 |
| `absent` | 495,620 |
| `span-aligned` | 275,276 |
| `fused` | 12,589 |
| `unaligned` | 32,231 |
| `invalid` | 9 |
| right context | 3,822 |
| left context | 2,693 |
| connection matrix cost | 10,292,646 |

builder는 graph를 쓰기 전에 같은 source에서 full schema 3을 만들고 모든 surface의 분석 순서, POS, left/right context ID, word cost, `analysis_type`, source position, expression relation과 component를 비교한다. connection matrix와 `char.def`·`unk.def`도 전체 비교하며, 한 필드라도 다르면 artifact를 생성하지 않는다. `invalid` 9건은 source expression을 유효한 relation으로 파싱하지 못했다는 보존된 상태이지 누락된 analysis가 아니다.

## 크기

| 형식 | schema | artifact bytes | MiB | full 대비 |
| --- | ---: | ---: | ---: | ---: |
| full | 3 | 72,164,646 | 68.82 | 100.00% |
| compact | 1 | 47,859,711 | 45.64 | 66.32% |
| graph | 2 | 92,758,442 | 88.46 | 128.54% |

compact는 현재 비용 판정이 읽지 않는 source metadata를 버리지만 graph는 구조 resolver의 근거가 될 분석 종류, source 위치와 expression component를 보존한다. graph가 full보다 큰 결과는 별도 string table과 component record를 추가하면서 full schema의 raw row 표현보다 아직 조밀하지 않기 때문이다.

## 조회 결과

| 형식 | cache | 초기화 ms | exact ns/query | prefix ns/query | peak RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| full resident | cold | 317.48 | 646.73 | 670.02 | 72.86 |
| full resident | warm | 318.21 | 662.92 | 674.42 | 72.84 |
| compact resident | cold | 146.65 | 329.57 | 356.90 | 49.53 |
| compact resident | warm | 142.06 | 397.82 | 356.52 | 49.53 |
| graph resident | cold | 690.86 | 856.17 | 891.82 | 92.64 |
| graph resident | warm | 670.68 | 834.16 | 873.60 | 92.64 |

| workload | queries | matches | analyses | checksum |
| --- | ---: | ---: | ---: | ---: |
| exact | 9,912 | 4,956 | 5,189 | 5,901,055,339,043,549,701 |
| common-prefix | 9,912 | 26,925 | 108,502 | 7,072,030,433,407,230,049 |

세 형식의 workload 결과는 모두 일치한다. graph의 추가 비용은 현재 decoder가 모든 relation과 span을 초기화 때 검증하고 각 analysis를 소유 구조로 복원하며, 조회 때 graph analysis view를 구성하는 데서 발생한다. 이 결과는 policy-neutral schema가 의미 보존 계약을 만족한다는 근거이지만 제품 전환의 성능 채택 조건은 만족하지 않는다.

## 다음 단계

다음 stack은 `CompoundExposure`의 `opaque`, `transparent`, `explicit` profile 계약을 정한 뒤 schema 2 위에 `ConstraintResolver` shadow를 구현한다. 저장 크기와 초기화 비용 최적화는 projection equivalence를 유지하는 별도 stack으로 분리하며, surface registry나 새 비용 임계값으로 구조 구분을 대체하지 않는다.
