# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 상태와 바로 이어갈 작업만 유지한다. 측정 과정과 완료한
작업 순서는 개별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [User smart precision 품질·성능](2026-07-14-user-smart-precision.md)
- [Agent precision shadow 판정](2026-07-14-agent-precision-shadow.md)
- [`-기` 명사형 조사 continuation 품질·성능](2026-07-14-gi-particle-continuation.md)
- [국소 lattice 제품 경로 최적화](2026-07-14-local-lattice-optimization.md)
- [Development false negative 진단](2026-07-14-development-fn-diagnostics.md)
- [`ending.connective-ji` 위치 근거](2026-07-14-connective-ji-position-evidence.md)
- [명시적 품사 `-지` 오른쪽 끝 recall](2026-07-14-connective-ji-right-edge-recall.md)
- [ㅎ 불규칙 core lexicon recall](2026-07-14-h-irregular-recall.md)
- [의존명사 coarse-POS fallback recall](2026-07-14-dependent-noun-recall.md)
- [Full POS coarse noun 분석 합집합 recall](2026-07-14-full-pos-coarse-noun-recall.md)
- [smart component 검색 근거](2026-07-13-smart-component-evidence.md)
- [copula lattice 폐기 판정](2026-07-13-copula-unseen-evaluation.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)

## 제품 상태

- CLI, Rust library와 WASM binding은 같은 query compiler와 matcher를 사용한다.
- 사람용 CLI 기본 경로는 full POS와 `smart`다. 품사를 명시하는 자동화 경로는
  `--boundary any --embedded --json`을 사용한다.
- 명시적 품사 `smart`는 precision 99.00% 하한과 hard-negative 보호 안에서 FN을 FP보다 우선해
  줄인다. 무품사 결과는 품사 모호성을 포함한 제품 한계로 그대로 보고한다.
- `smart`의 명사 branch는 문자열 token 경계 또는 compact component resource의 완전한 형태
  component 근거가 있어야 한다. component 경계를 가로지르는 substring은 거부한다.
- CLI는 `NominalComponent` 또는 `PredicateLexical` branch가 있는 plan에서 compact component resource를 자동으로
  해석한다. 필요 resource의 누락·손상·schema 또는 source 불일치는 초기화 오류이며 경계
  판정으로 fallback하지 않는다.
- Rust/WASM engine은 full POS와 component bytes를 자동으로 찾지 않는다. caller가 생성자나
  load API로 명시하며, resource가 없는 component `smart` compile은 오류다.
- `smart`의 지정사 strict-subspan match는 token 전체의 exact 분석이 모두 non-predicate일 때
  해당 predicate branch만 거부한다. token 전체 match, predicate·미해석 분석, 다른 query
  branch는 유지한다.
- `smart` 무품사 조사 검색은 입력한 표면형만 사용한다. 이형태 묶음 확장은 명시적 조사 품사
  입력에서 유지하며 `token`과 `any` 계획은 바꾸지 않는다.
- `ending.nominalizer-gi` predicate branch는 nominal particle verifier로 전이한다. `smart`와
  `token`은 유효한 조사 연쇄를 token 끝까지 소비하고 잘못된 이형태와 격조사 중복을 거부한다.
- 명시적 동사·형용사 품사의 `ending.connective-ji` branch는 오른쪽 token 경계를 유지하면서
  왼쪽 core 경계를 열어 오른쪽 끝 suffix를 복구한다. 무품사, `token`, `any`와 뒤에 문자가
  남는 candidate는 바꾸지 않는다.
- core lexicon의 ㅎ 불규칙 예외에는 `어떻다`, `이렇다`, `커다랗다`가 포함된다. 기존 `DropH`
  generator로 `어떤`, `이런`, `커다란`을 만들며 규칙형 `어떻은`, `이렇은`, `커다랗은`은
  만들지 않는다.
- 명시적 coarse `noun`의 사전 분석이 없으면 보통명사·고유명사·의존명사 fallback을 모두
  보존한다. component 판정은 corpus의 `NNBC`를 query-side `NNB`와 같은 의존명사로
  비교하며 artifact와 진단의 source tag는 바꾸지 않는다.
- 명시적 coarse `noun`에 full POS 분석이 있으면 누락된 보통명사·고유명사·의존명사
  fallback과 합집합으로 보존한다. user lexicon의 `replace = true`는 이 합집합보다 우선한다.
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
| embedded | smart | 415 / 0 / 85 | 100.00% | 83.0% | 90.71% |
| full-POS | smart | 415 / 0 / 85 | 100.00% | 83.0% | 90.71% |
| embedded/full-POS | token | 355 / 0 / 145 | 100.00% | 71.0% | 83.04% |
| embedded/full-POS | any | 480 / 11 / 20 | 97.76% | 96.0% | 96.87% |

development 명시적 품사 `smart`는 embedded와 full-POS 모두 TP 442 / FP 2 / FN 58이다.
full-POS의 기존 세부 품사 분석과 coarse `noun` fallback 합집합이 `197명이`를 복구했다.
`token`과 `any`에서도 두 lexicon profile의 품질이 같다. 세부 품사와 품질 계약은
[User smart precision 품질·성능](2026-07-14-user-smart-precision.md), 현재 처리량과 latency는
[Full POS coarse noun 분석 합집합 recall](2026-07-14-full-pos-coarse-noun-recall.md)을 기준으로 한다.

품사를 생략하는 사람용 1,000-case fixture에서 full-POS `smart`는 TP 411, FP 0, FN 89,
precision 100.00%, recall 82.2%, F1 90.23%다. embedded `smart`는 TP 319, FP 0, FN 181이다.
embedded `smart`는 기대 품사를 plan에 포함하는
비율이 47.8%이므로 사람용 기본 경로를 대신하지 않는다.

explicit-POS test fixture의 품사를 제거한 User persona도 full-POS `smart`에서 TP 411, FP 0,
FN 89, precision 100.00%, recall 82.2%, F1 90.23%다. `이다 -> 매일`은 whole-token lexical
근거로, determiner query `이 -> 날씨가`는 무품사 조사 이형태 확장을 제한해 제거했다.
fixture·gold·지표 정의는 바꾸지 않았으며 현재 `any`는 TP 480 / FP 11 / FN 20이다.

무품사 fixture와 persona 결과는 명시적 품사 품질과 분리한다. 목표 수치를 맞추기 위한 fixture,
gold, negative 선택 변경은 허용하지 않으며 품사 모호성에서 생긴 FP와 FN도 그대로 남긴다.

## 현재 경계

- `-기` 명사형 뒤의 유효한 조사 연쇄는 predicate token의 일부다. `걷기가`, `걷기를`,
  `걷기에서도`를 찾고 `걷기이`, `걷기을`, `걷기으로`, `걷기가를`은 `smart`와 `token`에서
  거부한다. 다른 명사형·종결형·연결형은 조사 verifier로 전이하지 않는다.
- `smart` component는 exact component span만 복구한다. `대학교`의 `학교`처럼 source 분석이
  component로 증명하지 않는 substring과 `역사과목`의 `사과`처럼 component 경계를 가로지르는
  span은 거부한다.
- component resource가 필요한 `smart` query의 fail-fast 동작은 호환성 계약이다. optional
  resource가 필요한 caller는 query compile 전에 resource를 준비해야 한다.
- whole-token 분석은 지정사 strict-subspan보다 우선한다. 향후 문맥 예외는 bounded local
  분석에서 whole-token을 포함하는 완전 경로가 없고 candidate를 포함하는 split 완전 경로만
  있을 때만 match를 복구한다. 경로 비용 우열만으로 이 결정을 뒤집지 않는다.
- `그건 매일 수도 있어`는 `매일/MAG + 수/NNB+도/JX + 있어` 경로가 완전하므로 위 문맥 예외의
  positive가 아니다. 구현 전에는 전체-token 경로가 실제로 불가능한 최소 대조 fixture를 먼저
  확보한다.
- Korean-Kaist·KSL dev의 실제 지정사 annotation에는 `예이다`, `생명인데`, `것인가를`처럼
  `any`에는 있고 `smart`가 제거한 gold token이 130개 있다. annotation의 split만으로
  whole-token 완전 경로의 부재를 증명하지 않으므로 제품 복구 근거로 사용하지 않는다.
- Agent precision 후보는 먼저 `embedded + any` 결과에 대한 benchmark shadow로만 측정한다.
  timed 결과와 제품 `any` 결과는 유지하고, bounded local lattice의 include/exclude 완전 경로
  존재 여부와 생성 근거를 development·hard-negative에서 분류한다.
- Agent shadow의 `include-path` 투영은 development TP를 484에서 444로 줄이면서 FP 15를
  유지했다. `include-only`는 FP를 0으로 줄이지만 TP도 10으로 줄였다. 제품 matcher와 `any`
  정책은 변경하지 않는다.
- Korean-Kaist·KSL dev의 실제 지정사 token과 겹치는 `이다` candidate 1,174개는 모두 include와
  exclude 완전 경로가 함께 존재했다. 지정사 split만 가능한 최소 대조가 없으므로 문맥 복구를
  구현하지 않는다.
- `ending.connective-ji` 오른쪽 끝은 `주다 -> 심어주지`를 복구한다. 같은 표면형 hard-negative
  `주지 스님`은 기존 FP이며, 남은 gold candidate 3건은 모두 `left-edge`다.
- `ending.connective-ji` left-edge의 bounded token 판정은 `없다/VA -> 없지는`과 같은 candidate
  표면형이지만 `없다/VX -> 없지요`인 대조군을 구분하지 못한다. 이 위치 유형은 제품에 열지 않고
  오른쪽 token 경계를 유지한다.
- 남은 full-POS `smart` FN 58건은 `boundary-rejected` 41건, `surface-missing` 11건,
  `span-mismatch` 3건, `lexicon-missing` 3건이다.

## 이어갈 작업

최우선 목표는 명시적 품사 full-POS `smart`의 FN을 줄이는 제품 변경이다. 계측·report·runner만
바꾼 상태는 작업 완료나 독립 PR 대상으로 보지 않는다. 무품사와 고정 test 결과는 규칙 선택에
사용하지 않고 제품 규칙을 고정한 뒤 회귀 판정에만 사용한다.

1. development full-POS `smart`의 TP 442 / FP 2 / FN 58을 기준선으로 사용한다. full-POS
   coarse `noun` 분석 합집합은 `197명이`를 복구했고 `명 -> 익명이`는 계속 거부한다.
2. 남은 `boundary-rejected` 41건을 품사와 any-boundary token 위치별로 나눈다. 같은 candidate
   surface 대조군과 exact component 경로가 positive와 negative를 구분하는 유형만 제품 후보로
   삼는다. `서사극이라`와 `인쇄업자가`는 구분되지 않으므로 열지 않는다.
3. development에서 FN 58 미만, precision 99.00% 이상과 기존 hard-negative 신규 FP 0을 모두
   만족해야 한다. 통과 후보가 여럿이면 FN이 적은 후보를 먼저 선택하고 FN이 같을 때 FP가 적은
   후보를 선택한다.
4. 규칙 고정 뒤 explicit-POS test의 FN 85를 늘리지 않고 precision 99.00% 이상을 유지해야 한다.
   같은 고정 무품사 fixture와 User persona도 다시 측정해 불리한 변화를 포함한 결과를 기록한다.
   관련 morphology workload의 성능 회귀도 없어야 한다.
5. Agent precision은 위 explicit-POS `smart` recall 작업 뒤에 재개한다. include/exclude lattice
   존재 여부와 다른 독립 근거가 정의되어야 하며, development TP 484 보존, FP 15 미만,
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
cargo bench -p kfind-testkit --bench query_matcher -- local_lattice
cargo fmt --manifest-path tools/morph-index-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-index-benchmark/Cargo.toml \
  --all-targets -- -D warnings
cargo test --locked --manifest-path tools/morph-index-benchmark/Cargo.toml
scripts/benchmark-morphology.sh
scripts/benchmark-morph-index.sh
```
