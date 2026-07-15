# 선택적 국소 형태 추론 계약

## 적용 범위

기본 검색은 query-side 형태 컴파일과 anchor 주변 verifier를 사용한다. `smart`의 선택적
corpus-side 분석은 token 내부 component와 바로 인접한 token 구조를 확인한다.

| context requirement | 적용 대상 | 제품 결과 |
| --- | --- | --- |
| `None` | literal과 일반 형태 branch | 기존 verifier 결과 |
| `PredicateLexical` | 왼쪽 경계를 연 지정사 branch | 어휘 분석과 구조 판정으로 좁힘 |
| `ExactComponent` | token 내부 명사류·관형사와 full-POS 일반 용언 component 후보 | `accept`만 match로 복구 |
| `LexicalContext` | 문맥에서 품사를 검증할 부사 branch | 구조 판정이 있으면 해당 품사만 유지 |

## 실행 계약

```text
byte scan
  -> anchor hit
  -> morphology/boundary verifier
  -> bounded Unicode token 추출
  -> context requirement별 compact component 검증
  -> exact component 복구 또는 인접 token 구조로 품사 선택
```

- anchor가 없는 파일과 줄에서는 lattice를 실행하지 않는다.
- literal, `token`, `any`와 context가 필요 없는 branch는 resource를 읽지 않는다.
- token 내부 원문 범위와 같은 줄의 인접 token 문맥은 256 bytes, NFC 문자열은 64 Unicode
  scalar로 제한한다. local lattice는 4,096 node로 제한한다.
- NFC 안정 경계와 원문 byte offset을 양방향으로 보존한다.
- fixture 전용 비용, corpus 단어 denylist와 결과별 예외를 사용하지 않는다.
- resource 오류와 상한 초과를 조용히 다른 판정으로 바꾸지 않는다.

## resource 계약

query-side full POS와 corpus-side morphology resource는 같은 고정 source snapshot에서 생성하지만
별도 artifact다. full POS는 표제어·품사를, corpus-side resource는 원본 표면형의 모든 분석,
연결 ID, 단어 비용, matrix와 unknown 정의를 보존한다.

제품 `ExactComponent`는 compact schema 1을 사용한다. full morphology schema 3은 benchmark
동등성 검증에 사용한다. compact/full의 exact/common-prefix hit, scoring checksum, candidate
decision, 비용, node와 path provenance가 모두 일치해야 한다.

## benchmark 계약

- `ExactComponent`는 품사별 기존 경계 reject, resource lookup, accept/reject와 경로 provenance를
  기록한다.
- component candidate가 있는데 resource가 없거나 검증에 실패하면 benchmark를 실패시킨다.
- 고정 test, dev와 hard-negative의 역할을 섞지 않는다.

## Optional component startup

resource 없는 Rust/WASM engine과 생성 후 compact component resource를 수동 초기화한 engine의
시간과 RSS를 분리해 측정한다. native 결과는 morphology report의 `component_startup`, WASM
결과는 별도 JSON에 기록한다.

```console
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
```

최신 수치는 [Full-POS 용언 exact component 확장](2026-07-15-predicate-exact-component.md)에
기록한다.
