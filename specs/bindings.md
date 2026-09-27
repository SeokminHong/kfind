# Rust와 JavaScript API

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

## 0. 제품 계약

이 절은 뒤의 일반 설명보다 우선한다.

### 0.7 Rust 라이브러리와 WASM 대상

- native CLI와 npm CLI의 입력·resource 해석과 달리 Rust 라이브러리와 npm binding API는 filesystem, URL 또는 package
  asset 위치를 추정하지 않는다. caller가 component 기능을 사용할 때만 bytes를 명시적으로 전달한다.
- `kfind` 파사드 crate는 `ResourceBundle { full_pos, enriched_predicates, component }`와
  `Engine::with_resources(resources)`를 전체 사전 profile의 기본 생성 API로 제공한다. full POS binary와
  enriched predicate UTF-8 TSV는 생성 중 lexicon에 병합하고, component bytes는 compact resource로
  검증해 engine이 소유한다. 각 필드는 선택 사항이며 빈 bundle은 `Engine::new()`와 같은 profile이다.
- 기존 `with_full_pos`, `with_component_resource`, `with_full_pos_and_component` 생성자는 1.x 호환
  API로 유지하되 같은 bundle 생성 경로에 위임한다.
- 1.0의 안정 Rust facade는 `Engine`, `Matcher`, `ResourceBundle`, compile option·오류와 match
  provenance 타입을 crate root에 둔다. 이 타입의 공개 method, enum variant와 field는 1.x 호환
  계약이다.
- caller-configured `Lexicons`, `QueryPlan`과 matcher의 plan 접근은 `kfind::expert` 아래에만 둔다.
  expert API는 계획 IR과 사전 조립 실험을 위한 것으로 1.x 안정 facade 계약에 포함하지 않는다.
  root의 engine 생성·검색 경로는 expert 타입을 인자나 반환값으로 노출하지 않는다.
- `kfind-data`, `kfind-morph`, `kfind-query`, `kfind-matcher`, `kfind-search`, `kfind-testkit`은 workspace
  내부 crate이며 crates.io 배포 대상이 아니다. 공개 Rust 소비자는 `kfind` facade만 사용한다.
- component resource는 생성 이후 first-use에 자동 fetch·load하지 않는다. 검증된 resource는 engine이
  소유하고 여러 matcher에서 재사용하며 query compile마다 다시 decode하지 않는다. resource가 없는
  engine에서 source 또는 runtime component capability가 필요한 smart plan을
  compile하면 명시적 `ComponentResourceRequired` 오류를 반환하고 기존 경계 판정으로
  fallback하지 않는다.
- component resource decoder는 resource를 공개하기 전에 모든 section digest와 payload 구조를
  검증하고 header의 package version이 현재 binary/library version과 정확히 같은지 확인한다.
  Encoded component resource는 128 MiB를 초과할 수 없으며, decoder는 section digest 계산이나
  payload 구조 검증을 시작하기 전에 이 상한을 검사한다.
  Native에서는 큰 index와 payload section의 digest를 서로 다른 thread에서 검증할 수 있고,
  같은 큰 resource의 payload 구조 검증을 section digest 검증과 겹쳐 수행할 수 있다. Thread를
  만들 수 없으면 순차 검증으로 돌아가고 WASM은 순차 검증한다. 어느 경로도 digest나 payload
  구조 검증을 생략하거나 두 검증이 모두 끝나기 전 resource를 engine 상태에 설치하지 않는다.
  병렬 경로에서 두 검증이 모두 실패하면 section digest 오류를 먼저 반환한다.
- section SHA-256은 지원 CPU에서 runtime detection으로 hardware backend를 사용하고, 사용할 수
  없으면 target-compatible backend로 돌아간다. Backend와 무관하게 같은 digest를 계산하며 검증
  범위와 오류 계약을 바꾸지 않는다.
- 같은 fail-fast 계약은 저수준 `MorphMatcher` 생성자에도 적용한다. resource가 필요한 plan을
  `MorphMatcher::new`로 만들면 `MorphMatcherBuildError::ComponentResourceRequired`를 반환하며,
  resource 또는 evaluator를 받는 생성자만 해당 plan을 초기화할 수 있다.
- 생성 후 `Engine::load_component_resource(component_resource)`와 JavaScript
  `loadComponentResource(componentResource)`로 resource를 명시적으로 초기화하거나 교체할 수 있다.
  새 bytes를 모두 검증한 뒤에만 상태를 교체하며 실패하면 기존에 검증된 resource를 유지한다.
- engine은 full POS, enriched predicate와 component resource의 초기화 여부를 각각 getter로 노출한다.
  resource가 필요 없는 literal, `token`, `any`와 boundary-only plan은 component가 없는
  engine에서 그대로 compile한다.
- 라이브러리 matcher는 UTF-8 byte slice에서 겹치지 않는 match와 형태 분석 provenance를
  반환한다. 파일 순회, 인코딩 판별, 출력 형식과 CLI locale 처리는 라이브러리 API에
  포함하지 않는다.
- `kfind`, `kfind-wasm`, `kfind-data`, `kfind-morph`, `kfind-query`, `kfind-matcher`는
  Rust 1.97에서 `wasm32-unknown-unknown` 대상으로 빌드되어야 한다.
- `kfind-wasm`은 `wasm-bindgen` JavaScript glue와 TypeScript declaration을 생성한다.
  npm package metadata와 게시 계약은 [0.8절](distribution.md)을 따른다.
- JavaScript API는 `Kfind.withResources({ fullPos?, enrichedPredicates?, component? })`를 전체 사전
  profile의 기본 생성 API로 제공한다. binary resource는 `Uint8Array`, enriched predicate TSV는
  JavaScript string이다. `new Kfind(componentResource?)`와
  `Kfind.withFullPos(fullPos, componentResource?)`는 같은 bundle 경로에 위임하는 호환 API다.
  재사용 가능한 `Matcher`를 만드는 `compile`, 수동 `loadComponentResource`, UTF-16 JavaScript
  문자열을 검색하는 `findAll`을 제공한다.
  component bytes를 명시했을 때 빈 bytes, 손상, schema·source mismatch는
  `failed to initialize kfind` JavaScript `Error`다. component가 없는 인스턴스의 component smart
  compile은 `failed to compile query` JavaScript `Error`이며 자동 load나 fallback을 수행하지 않는다.
- WASM binary에는 compact component resource bytes를 `include_bytes!` 또는 동등한 방식으로
  포함하지 않는다. binding은 URL fetch, filesystem과 bundler asset resolution을 수행하지 않으며
  호출자가 외부 호스팅 URL 또는 별도 정적 asset에서 bytes를 읽어 생성자에 전달한다.
- `compile`은 선택적 camelCase 객체로 `expand`, `boundary`, `pos`, `normalization`,
  `maxGap`, `literal`을 받는다. 값 집합과 충돌 규칙은 CLI compile option과 동일하며
  알 수 없는 필드, 잘못된 값과 컴파일 실패는 JavaScript `Error`로 드러낸다.
- match와 atom의 `start`, `end` offset은 JavaScript `String.prototype.slice`에 바로
  사용할 수 있는 UTF-16 code unit 기준이다. 각 atom은 core·token span과 모든
  `analysisIndex`, `rulePath` provenance를 보존한다.
- 기본 CI는 Linux, Apple Silicon macOS와 x64 Windows에서 네이티브 Rust 테스트를 실행한다.
  POSIX process·file-lock 계약을 사용하는 benchmark guard와 `getrusage` 기반 morph index
  benchmark test는 Linux와 macOS에서 실행하고, Linux에서 MSRV의 `kfind-wasm` build를 검사한다.

## 24. 공개 코드 인터페이스

Rust 공개 API는 재사용 가능한 `Engine`과 컴파일된 `Matcher`를 중심으로 한다.

```rust
let engine = Engine::with_resources(ResourceBundle {
    full_pos: Some(full_pos_bytes),
    enriched_predicates: Some(enriched_predicates),
    component: Some(component_bytes),
})?;

let matcher = engine.compile("권한", &CompileOptions::default())?;
let matches = matcher.find_all("사용자권한을 확인한다.".as_bytes());
```

- `Engine::new`는 embedded lexicon만 초기화한다.
- `ResourceBundle`과 `Engine::with_resources`는 full POS, enriched predicate와 component resource를
  한 profile로 검증한다. 기존 개별 생성자는 이 경로에 위임한다.
- `load_component_resource`는 새 bytes를 모두 검증한 뒤 상태를 교체하며 실패하면 기존
  resource를 보존한다.
- `compile`은 query plan과 anchor matcher를 만들고 component resource가 필요한 plan의 누락을
  `ComponentResourceRequired`로 보고한다.
- `Matcher::find_at`과 `find_all`은 UTF-8 byte offset과 형태 provenance가 포함된
  `PhraseMatch`를 반환한다. `find_all_limit(input, max_matches)`는 같은 결과를 최대
  `max_matches`개까지 수집하고 추가 일치가 있으면 `MatchLimitExceeded`를 반환한다.
  `max_matches = 0`은 일치가 없을 때만 빈 결과를 반환한다.
- `find_at_with_route`, `find_all_with_routes`, `find_all_with_routes_limit`과
  `find_all_with_routes_with_diagnostics`는 기존 `PhraseMatch` 구조를 유지하면서 선택된
  쿼리 atom 번호를 `RoutedMatch.query_atom_indices`에 함께 반환한다.
- root의 `PhraseMatch`, `VerifiedSpan`, `Origin`, `RuleId`와 compile option·오류는 1.x 안정
  계약이다. `QueryPlan`, candidate program·structural constraint 표현, `Lexicons`와 plan inspection은 `kfind::expert`의
  변경 가능한 저수준 API다.
- CI는 `kfind` crate의 공개 Rust API를 최신 1.x 정식 release tag와 비교해 1.x 호환성 파괴를 거절한다.
- workspace 내부 crate는 게시하지 않으며 `kfind::expert` 외의 경로를 공개 API로 간주하지 않는다.
- JavaScript API는 같은 profile을 `Kfind.withResources`, 같은 수명 주기를
  `loadComponentResource`, `compile`, `Matcher.findAll`, `Matcher.findAllLimit`,
  `Matcher.findAt`, `Matcher.findAllWithDiagnostics`로 노출한다. 일치 span과 `findAt`의
  시작 위치는 UTF-16 code unit이다. `findAllLimit`은 결과가 상한을 초과하면 오류를 던지고,
  `findAt`은 일치가 없으면 `null`을 반환하며 surrogate pair 중간 위치는 거절한다.
  `findAllWithDiagnostics`는 일치 목록과 구조 검증 불완전 여부를 반환한다.
  괄호 또는 phrase와 대안을 함께 사용하는 쿼리의 match에는 선택된 원본 atom 번호를
  `queryAtomIndices`로 포함한다.
