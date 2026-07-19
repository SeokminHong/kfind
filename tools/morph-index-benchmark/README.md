# 형태 index 벤치마크

동일한 형태 payload를 저장한 immutable prefix index를 비교하는 개발 도구입니다.

- `yada` packed Double-Array trie
- `fst` map

Payload는 지원하는 모든 MeCab 표면 분석의 품사, left/right context ID와 word cost를
보존합니다. 검색 질의용으로 정규화한 full POS lexicon과는 별도입니다.

같은 실행에서 전체 schema 3 lattice resource와 판정 결과가 같은 compact component
projection도 비교합니다. Compact artifact는 source node의 품사, context ID, word
cost, connection matrix와 unknown-word 정의를 보존하고 component cost 판정이 읽지
않는 source 분석 metadata를 제외합니다. 두 artifact의 exact/common-prefix workload
분석 수와 scoring checksum이 다르면 build가 실패합니다.

저장소 root에서 고정된 전체 규모 benchmark를 실행합니다.

```console
scripts/benchmark-morph-index.sh
```

Artifact와 raw probe report는 `target/morph-index-benchmark`에 생성합니다. 각
container에는 schema version, source archive SHA-256, entry 통계, section 길이와
section SHA-256이 들어 있습니다. `validate`는 schema, source digest와 content
integrity 오류를 구분합니다.

`component-build-report.json`은 full/compact 크기와 lookup 동등성을 기록합니다.
`component-*-{resident,mmap}-{cold,warm}.json`은 initialization, lookup latency,
analysis hit, checksum과 peak RSS를 기록합니다.

각 storage/index 조합은 첫 open process인 `cold`와 두 번째 process인 `warm`에서
실행합니다. 운영체제 page cache를 비우지 않으므로 cold 수치는 물리 disk cold-cache가
아니라 first-open 측정으로 설명해야 합니다.

File-backed mapping은 read-only로 엽니다. Memory mapping API의 안전 계약에 따라
probe process가 살아 있는 동안 생성된 `.kfm` 파일은 변경하지 않습니다.
