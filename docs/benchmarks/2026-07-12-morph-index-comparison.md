# 형태소 prefix index 비교

## 결론

P2의 full morphology index는 읽기 전용 mmap packed Double-Array trie를 사용한다. FST보다
index가 7,452,614 bytes 크지만 exact lookup은 약 6배, common-prefix 열거는 약 4.3배
빨랐고 peak RSS는 28.1 MiB로 40 MiB 게이트 안이다.

query-side full POS lexicon은 정규화된 표제어와 품사만 담당한다. corpus-side morphology
index는 같은 고정 source에서 원본 표면형, 품사, 좌·우 연결 ID와 단어 비용을 별도 payload로
보존한다. 이번 작업은 query 분석과 검색 결과를 변경하지 않는다.

## 입력

| 항목 | 값 |
| --- | ---: |
| source | `mecab-ko-dic-2.1.1-20180720` |
| source SHA-256 | `fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330` |
| 읽은 행 | 816,283 |
| 지원 품사 밖 행 | 57,923 |
| 중복 분석 | 733 |
| 고유 표면형 | 729,173 |
| 분석 | 757,627 |
| exact query | 9,989개, hit와 miss 교대 |
| prefix query | 9,989개 |
| 반복 | query당 100회 |

환경은 Apple M1 Max 10-core, 32 GB, macOS 26.4.1, Rust 1.97.0이다. `cold`는 artifact 생성 뒤
첫 probe process, `warm`은 바로 다음 별도 process다. 운영체제 page cache를 비우지 않았으므로
물리 디스크 cold-cache 수치로 해석하지 않는다.

## 크기

두 후보는 같은 12,008,228-byte payload를 사용한다.

| 형식 | index | container |
| --- | ---: | ---: |
| packed Double-Array | 13,725,696 bytes | 25,734,152 bytes |
| FST | 6,273,082 bytes | 18,281,538 bytes |

## 결과

시간은 query 1개당 ns, RSS는 process peak다. container 로드, schema·source digest·section
digest·payload 검증과 index 검증을 초기화 구간에 포함한다.

| 형식 | storage | cache | 초기화 ms | exact ns | prefix ns | peak RSS MiB |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Double-Array | resident | cold | 76.23 | 34.24 | 47.81 | 28.11 |
| Double-Array | resident | warm | 76.32 | 35.81 | 48.98 | 28.11 |
| Double-Array | mmap | cold | 74.49 | 36.30 | 48.99 | 28.09 |
| Double-Array | mmap | warm | 74.19 | 36.82 | 48.78 | 28.09 |
| FST | resident | cold | 56.24 | 207.68 | 212.97 | 21.02 |
| FST | resident | warm | 58.36 | 217.60 | 210.12 | 21.02 |
| FST | mmap | cold | 54.91 | 212.92 | 207.72 | 21.00 |
| FST | mmap | warm | 55.32 | 208.01 | 208.56 | 21.00 |

모든 probe의 prefix match 수와 checksum은 동일했다. container SHA-256 검증이 전체 section을
읽기 때문에 같은 형식에서 resident와 mmap의 초기화·RSS 차이는 작다. mmap은 immutable full
resource의 heap 복사를 피하고 process 간 page를 공유하는 저장 방식으로 선택한다.

## 검증 계약

container는 다음을 검증한 뒤 index와 payload를 노출한다.

- schema version과 index kind
- source archive SHA-256
- 표면형·분석·품사별 count
- index와 payload 길이·SHA-256
- payload offset 순서, record 수, POS code
- FST 구조 또는 Double-Array section 크기

손상, schema 불일치와 source digest 불일치는 서로 다른 오류로 보고한다. source를 확장한 뒤
Double-Array peak RSS가 40 MiB를 넘거나 배포 크기가 병목이 되면 같은 benchmark로 FST를 다시
비교한다.

## 재현

```console
scripts/benchmark-morph-index.sh
```

산출물은 `target/morph-index-benchmark` 아래에 생성한다. raw cold/warm probe JSON과 build
report에 source digest, count, artifact 크기, 시간, RSS와 checksum을 보존한다.
