# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 상태와 바로 이어갈 작업만 유지한다. 측정 과정과 완료한
작업 순서는 개별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [ㄷ·ㅅ·ㅂ·ㅎ 불규칙 enriched 용언 lexicon](2026-07-14-consonant-irregular-enriched-lexicon.md)
- [르·러 불규칙과 enriched 용언 lexicon](2026-07-14-reu-reo-enriched-lexicon.md)
- [User smart precision 품질·성능](2026-07-14-user-smart-precision.md)
- [Agent precision shadow 판정](2026-07-14-agent-precision-shadow.md)
- [`-기` 명사형 조사 continuation 품질·성능](2026-07-14-gi-particle-continuation.md)
- [`-ㅁ/음` 명사형 품질·성능](2026-07-14-mieum-nominalizer.md)
- [합성 불규칙 용언 core lexicon](2026-07-14-compound-irregular-core-lexicon.md)
- [국소 lattice 제품 경로 최적화](2026-07-14-local-lattice-optimization.md)
- [Development false negative 진단](2026-07-14-development-fn-diagnostics.md)
- [`ending.connective-ji` 위치 근거](2026-07-14-connective-ji-position-evidence.md)
- [명시적 품사 `-지` 오른쪽 끝 recall](2026-07-14-connective-ji-right-edge-recall.md)
- [ㅎ 불규칙 core lexicon recall](2026-07-14-h-irregular-recall.md)
- [의존명사 coarse-POS fallback recall](2026-07-14-dependent-noun-recall.md)
- [Full POS coarse noun 분석 합집합 recall](2026-07-14-full-pos-coarse-noun-recall.md)
- [`매일` 인접 문맥 판별 품질·성능](2026-07-14-contextual-maeil-disambiguation.md)
- [smart component 검색 근거](2026-07-13-smart-component-evidence.md)
- [copula lattice 폐기 판정](2026-07-13-copula-unseen-evaluation.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)

## 제품 상태

- CLI, Rust library와 WASM binding은 같은 query compiler와 matcher를 사용한다.
- 사람용 CLI 기본 경로는 full POS, enriched 용언 metadata와 `smart`다. 품사를 명시하는
  자동화 경로는 `--boundary any --embedded --json`을 사용한다.
- 명시적 품사 `smart`는 precision 99.00% 하한과 hard-negative 보호 안에서 FN을 FP보다 우선해
  줄인다. 무품사 결과는 품사 모호성을 포함한 제품 한계로 그대로 보고한다.
- `smart`의 명사 branch는 문자열 token 경계 또는 compact component resource의 완전한 형태
  component 근거가 있어야 한다. component 경계를 가로지르는 substring은 거부한다.
- CLI는 `NominalComponent`, `PredicateLexical` 또는 `LexicalContext` branch가 있는 plan에서
  compact component resource를 자동으로 해석한다. 필요 resource의 누락·손상·schema 또는
  source 불일치는 초기화 오류이며 경계 판정으로 fallback하지 않는다.
- Rust/WASM engine은 full POS와 component bytes를 자동으로 찾지 않는다. caller가 생성자나
  load API로 명시하며, resource가 없는 component `smart` compile은 오류다.
- `smart`의 지정사 strict-subspan match는 token 전체의 exact 분석이 모두 non-predicate일 때
  해당 predicate branch만 거부한다. token 전체 match, predicate·미해석 분석, 다른 query
  branch는 유지한다.
- `smart` 무품사 조사 검색은 입력한 표면형만 사용한다. 이형태 묶음 확장은 명시적 조사 품사
  입력에서 유지하며 `token`과 `any` 계획은 바꾸지 않는다.
- `ending.nominalizer-gi`와 `ending.nominalizer` predicate branch는 nominal particle verifier로
  전이한다. `smart`와 `token`은 유효한 조사 연쇄를 token 끝까지 소비하고 잘못된 이형태와
  격조사 중복을 거부한다.
- 명시적 동사·형용사 품사의 `ending.connective-ji` branch는 오른쪽 token 경계를 유지하면서
  왼쪽 core 경계를 열어 오른쪽 끝 suffix를 복구한다. 무품사, `token`, `any`와 뒤에 문자가
  남는 candidate는 바꾸지 않는다.
- core lexicon의 ㅎ 불규칙 예외에는 `어떻다`, `이렇다`, `커다랗다`가 포함된다. 기존 `DropH`
  generator로 `어떤`, `이런`, `커다란`을 만들며 규칙형 `어떻은`, `이렇은`, `커다랗은`은
  만들지 않는다.
- core lexicon은 `다르다 → 달라`, `이르다 → 일러/이르러`, `푸르다 → 푸르러`처럼 자주 쓰는
  르·러 불규칙과 동형어를 보존한다. full-POS 제품 경로는 국립국어원 사전 snapshot에서
  검토한 불규칙 분석 278개와 규칙형 동형어 companion 2개를 추가한다. 기존 르·러 102개에
  ㄷ·ㅅ·ㅂ·ㅎ 176개를 더하며, `푸다 → 퍼`는 core에서 유지한다. `곱다/VA`, `굽다/VV`처럼
  규칙형과 불규칙형이 독립적으로 확인된 표제어는 두 검색 branch를 모두 보존한다.
- 표준국어대사전·우리말샘의 고정 snapshot과 양방향 fixture로 검증한 `가려듣다`, `덧싣다`,
  `쏟아붓다`, `흘려듣다`는 core 불규칙 예외다. 자동 enriched 승격 조건은 유지하고 생성
  report에서는 네 항목을 `core-duplicate`로 기록한다.
- 명시적 coarse `noun`의 사전 분석이 없으면 보통명사·고유명사·의존명사 fallback을 모두
  보존한다. component 판정은 corpus의 `NNBC`를 query-side `NNB`와 같은 의존명사로
  비교하며 artifact와 진단의 source tag는 바꾸지 않는다.
- 명시적 coarse `noun`에 full POS 분석이 있으면 누락된 보통명사·고유명사·의존명사
  fallback과 합집합으로 보존한다. user lexicon의 `replace = true`는 이 합집합보다 우선한다.
- `smart`의 `LexicalContext` branch는 같은 줄의 bounded 인접 token 구조로 동형이의어를
  선택한다. `아니라 매일 것`은 `매/NNG + 이/VCP + ㄹ/ETM`, 반복된 `매일 매일`은
  `매일/MAG`, `매일을`은 `매일/NNG + 을/JKO`로 판정한다. 구조가 불충분하면 기존 경계
  판정을 유지한다.
- copula 전용 lattice 분기와 shadow 계측, PUD/GSD 전용 실행 경로는 복원하지 않는다.
- 기본 morphology benchmark는 kfind 프로필만 다시 실행한다. Kiwi·Lindera·MeCab-ko·KOMORAN
  품질은 test fixture와 어댑터 schema에 묶인 저장소 스냅샷을 읽고, fixture나 고정한 비교기
  설정이 바뀔 때만 `scripts/refresh-morph-baselines.sh`로 갱신한다.
- 제품 persona 비교는 같은 explicit-POS fixture와 gold를 사용한다. Agent와 외부 분석기는
  품사를 명시하고 User는 같은 query의 품사를 제거한 `full-POS + smart`로 실행한다. 이 결과는
  동일 입력의 backend 순위가 아니라 실제 입력 조건을 반영한 비교다.
- compact component artifact는 Homebrew의 `share/kfind`와 npm의 별도 정적 asset으로
  배포한다. WASM binary에는 artifact bytes를 포함하지 않는다.
- 제품 matcher의 local lattice는 query 포함·제외별 최저 비용만 계산한다. N-best 경로는
  shadow 진단에서만 생성하며 두 경로의 판정과 최저 비용은 같다. unknown model은 component
  evaluator마다 한 번 파싱하고 Engine이 matcher 사이에서 공유한다.

## 품질 기준선

명시적 품사를 사용하는 1,000-case test의 현재 제품 결과다.

| lexicon | boundary | TP / FP / FN | precision | recall | F1 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| embedded | smart | 416 / 0 / 84 | 100.00% | 83.2% | 90.83% |
| full-POS | smart | 423 / 0 / 77 | 100.00% | 84.6% | 91.66% |
| embedded | token | 356 / 0 / 144 | 100.00% | 71.2% | 83.18% |
| full-POS | token | 363 / 0 / 137 | 100.00% | 72.6% | 84.13% |
| embedded | any | 481 / 11 / 19 | 97.76% | 96.2% | 96.98% |
| full-POS | any | 491 / 11 / 9 | 97.81% | 98.2% | 98.00% |

development 명시적 품사 `smart`는 embedded가 TP 442 / FP 2 / FN 58, full-POS가
TP 443 / FP 2 / FN 57이다.
full-POS의 기존 세부 품사 분석과 coarse `noun` fallback 합집합이 `197명이`를 복구했다.
세부 품사와 품질 계약은 [User smart precision 품질·성능](2026-07-14-user-smart-precision.md),
현재 처리량과 latency는 [`-ㅁ/음` 명사형 품질·성능](2026-07-14-mieum-nominalizer.md)을
기준으로 한다.

품사를 생략하는 사람용 1,000-case fixture에서 full-POS `smart`는 TP 419, FP 0, FN 81,
precision 100.00%, recall 83.8%, F1 91.19%다. embedded `smart`는 TP 322, FP 0, FN 178이다.
embedded `smart`는 기대 품사를 plan에 포함하는
비율이 48.4%이므로 사람용 기본 경로를 대신하지 않는다.

explicit-POS test fixture의 품사를 제거한 User persona도 full-POS `smart`에서 TP 419, FP 0,
FN 81, precision 100.00%, recall 83.8%, F1 91.19%다. `이다 -> 매일`은 bounded 지정사 관형형
문맥이 아니면 whole-token lexical 근거로 거부하고, determiner query `이 -> 날씨가`는 무품사
조사 이형태 확장을 제한해 제거했다. fixture·gold·지표 정의는 바꾸지 않았으며 현재 `any`는
TP 481 / FP 11 / FN 19이다.

무품사 fixture와 persona 결과는 명시적 품사 품질과 분리한다. 목표 수치를 맞추기 위한 fixture,
gold, negative 선택 변경은 허용하지 않으며 품사 모호성에서 생긴 FP와 FN도 그대로 남긴다.

## 현재 경계

- `-기`와 `-ㅁ/음` 명사형 뒤의 유효한 조사 연쇄는 predicate token의 일부다. `걷기가`,
  `걷기에서도`, `걸음이`, `걸음으로`를 찾고 `걷기이`, `걷기가를`, `걸음가`, `걸음이를`은
  `smart`와 `token`에서 거부한다. 다른 종결형·연결형은 조사 verifier로 전이하지 않는다.
- `smart` component는 exact component span만 복구한다. `대학교`의 `학교`처럼 source 분석이
  component로 증명하지 않는 substring과 `역사과목`의 `사과`처럼 component 경계를 가로지르는
  span은 거부한다.
- component resource가 필요한 `smart` query의 fail-fast 동작은 호환성 계약이다. optional
  resource가 필요한 caller는 query compile 전에 resource를 준비해야 한다.
- whole-token 분석은 기본적으로 지정사 strict-subspan보다 우선한다. 예외는 직전 token이
  완전한 `VCN+EC`, 현재 token이 유일한 명사 prefix와 exact `VCP+ETM` suffix, 다음 token이
  `NNB/NNBC`로 시작하는 같은 줄의 bounded 구조다. 이때 명사 prefix와 지정사 suffix만
  선택하고 whole-token 명사·부사는 거부한다.
- NFC가 같은 인접 token에 exact `MAG` 분석이 있으면 반복된 두 token을 부사로 선택하고
  명사·명사 component branch는 거부한다. `매일을`처럼 조사까지 포함한 whole-token 명사
  분석은 그대로 명사로 선택한다.
- 인접 문맥은 같은 줄의 raw 256 bytes와 NFC scalar 64개로 제한한다. 줄을 넘거나 한도를
  초과하거나 구조가 모호하면 별도 문맥 결정을 만들지 않고 기존 `smart` 결과를 유지한다.
- Korean-Kaist·KSL dev의 실제 지정사 annotation에는 `예이다`, `생명인데`, `것인가를`처럼
  `any`에는 있고 `smart`가 제거한 gold token이 130개 있다. annotation의 split만으로
  whole-token 완전 경로의 부재를 증명하지 않으므로 제품 복구 근거로 사용하지 않는다.
- Agent precision 후보는 먼저 `embedded + any` 결과에 대한 benchmark shadow로만 측정한다.
  timed 결과와 제품 `any` 결과는 유지하고, bounded local lattice의 include/exclude 완전 경로
  존재 여부와 생성 근거를 development·hard-negative에서 분류한다.
- Agent shadow의 `include-path` 투영은 development TP를 487에서 452로 줄이면서 FP 16을
  유지했다. `include-only`는 FP를 0으로 줄이지만 TP도 8로 줄였다. 제품 matcher와 `any`
  정책은 변경하지 않는다.
- Korean-Kaist·KSL dev의 실제 지정사 token과 겹치는 `이다` candidate 1,174개는 모두 include와
  exclude 완전 경로가 함께 존재했다. 지정사 split만 가능한 최소 대조가 없으므로 문맥 복구를
  일반화하지 않는다.
- `ending.connective-ji` 오른쪽 끝은 `주다 -> 심어주지`를 복구한다. 같은 표면형 hard-negative
  `주지 스님`은 기존 FP이며, 남은 gold candidate 3건은 모두 `left-edge`다.
- `ending.connective-ji` left-edge의 bounded token 판정은 `없다/VA -> 없지는`과 같은 candidate
  표면형이지만 `없다/VX -> 없지요`인 대조군을 구분하지 못한다. 이 위치 유형은 제품에 열지 않고
  오른쪽 token 경계를 유지한다.
- 남은 full-POS `smart` FN 57건은 `boundary-rejected` 41건, `surface-missing` 10건,
  `span-mismatch` 4건, `lexicon-missing` 2건이다.

## 1.0 RC 안정화

새 형태 규칙보다 다음 안정화 조건을 먼저 닫는다.

1. 모든 공개 matcher 생성 경로는 plan의 resource 요구를 같은 방식으로 검사한다. 필요한
   resource가 없으면 fallback이나 부분 판정 없이 초기화 오류를 반환한다.
2. phrase 검색은 반복 token, 줄바꿈 없는 긴 입력과 큰 `max-gap`에서도 가능한 중간 조합을
   모두 만들어 메모리에 쌓지 않는다. 가장 이른 match만 필요한 경로와 전체 metadata 경로를 분리하고,
   병적 입력 benchmark와 fuzz target으로 시간·메모리 경계를 검증한다.
3. kfind 1.0의 안정 Rust API는 `Engine`, `Matcher`, compile option과 오류를 중심으로 확정한다.
   `QueryPlan`, branch·verifier 표현과 public field는 stable facade, 변경 가능한 expert API,
   내부 crate 중 하나로 분류한 뒤 공개 범위를 고정한다. 내부 crate의 배포 여부도 같은 기준으로
   명시한다.
4. 모든 fuzz target은 CI에서 target당 15초의 고정 예산과 개별 입력 timeout·RSS 상한으로 실제
   실행한다. phrase join의 반복 span·큰 gap, 손상된 UTF-8과 component resource 누락을 seed
   corpus에 포함하고 crash, panic, timeout과 RSS 초과가 없어야 한다.
5. 재배포 조건이 명확한 한국어 소스 코드 주석·README·기술 문서 snapshot으로 blind 검색 평가를
   추가한다. query와 기대 span은 규칙 개발 전에 고정하고 canonical 문장 중복을 제거하며,
   식별자 주변 한글, 띄어쓰기 오류, 한영·숫자 혼합, 동형이의어와 복합명사 substring을 별도
   slice로 보고한다. 이 결과는 기존 UD 회귀 fixture를 대체하지 않는다.

규칙 capability 일원화는 위 안정화 조건 뒤에 진행한다. 단순 boolean metadata가 실제 구현을
대신하지 않으므로 rule artifact에서 생성한 typed capability registry와 generator·verifier handler
coverage 검사를 함께 도입한다. 새 규칙이 없는 RC 변경에는 이 schema migration을 섞지 않는다.

## 이어갈 형태 품질 작업

RC 안정화 뒤의 형태 품질 목표는 명시적 품사 full-POS `smart`의 FN을 줄이는 제품 변경이다.
계측·report·runner만 바꾼 상태는 작업 완료나 독립 PR 대상으로 보지 않는다. 무품사와 고정
test 결과는 규칙 선택에 사용하지 않고 제품 규칙을 고정한 뒤 회귀 판정에만 사용한다.

1. development full-POS `smart`의 TP 443 / FP 2 / FN 57을 기준선으로 사용한다. full-POS
   coarse `noun` 분석 합집합은 `197명이`를 복구했고 `명 -> 익명이`는 계속 거부한다.
2. 남은 `boundary-rejected` 41건을 품사와 any-boundary token 위치별로 나눈다. 같은 candidate
   surface 대조군과 exact component 경로가 positive와 negative를 구분하는 유형만 제품 후보로
   삼는다. `서사극이라`와 `인쇄업자가`는 구분되지 않으므로 열지 않는다.
3. development에서 FN 57 미만, precision 99.00% 이상과 기존 hard-negative 신규 FP 0을 모두
   만족해야 한다. 통과 후보가 여럿이면 FN이 적은 후보를 먼저 선택하고 FN이 같을 때 FP가 적은
   후보를 선택한다.
4. 규칙 고정 뒤 explicit-POS test의 FN 77을 늘리지 않고 precision 99.00% 이상을 유지해야 한다.
   같은 고정 무품사 fixture와 User persona도 다시 측정해 불리한 변화를 포함한 결과를 기록한다.
   관련 morphology workload의 성능 변화도 불리한 결과를 포함해 기록해야 한다.
5. Agent precision은 위 explicit-POS `smart` recall 작업 뒤에 재개한다. include/exclude lattice
   존재 여부와 다른 독립 근거가 정의되어야 하며, development TP 487 보존, FP 16 미만,
   hard-negative 신규 FP 0을 모두 요구한다. `include-path`와 `include-only` 투영은 재사용하지
   않는다.

## 재현과 검증

```console
scripts/benchmark-morphology.sh
scripts/refresh-morph-baselines.sh
scripts/benchmark-morph-index.sh
pnpm --dir packages/kfind run benchmark:startup
```

형태소 계약을 변경할 때는 다음 검증을 함께 실행한다.

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/benchmark-criterion.sh local_lattice
cargo fmt --manifest-path tools/morph-index-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-index-benchmark/Cargo.toml \
  --all-targets -- -D warnings
cargo test --locked --manifest-path tools/morph-index-benchmark/Cargo.toml
scripts/benchmark-morphology.sh
scripts/benchmark-morph-index.sh
```
