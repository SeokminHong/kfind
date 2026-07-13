# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 상태와 바로 이어갈 작업만 유지한다. 측정 과정과 완료한
작업 순서는 개별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [User smart precision 품질·성능](2026-07-14-user-smart-precision.md)
- [Agent precision shadow 판정](2026-07-14-agent-precision-shadow.md)
- [smart component 검색 근거](2026-07-13-smart-component-evidence.md)
- [copula lattice 폐기 판정](2026-07-13-copula-unseen-evaluation.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)

## 제품 상태

- CLI, Rust library와 WASM binding은 같은 query compiler와 matcher를 사용한다.
- 사람용 CLI 기본 경로는 full POS와 `smart`다. 품사를 명시하는 자동화 경로는
  `--boundary any --embedded --json`을 사용한다.
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
- copula 전용 lattice 분기와 shadow 계측, PUD/GSD 전용 실행 경로는 복원하지 않는다.
- 기본 morphology benchmark는 kfind 프로필만 다시 실행한다. Kiwi·Lindera·MeCab-ko·KOMORAN
  품질은 test fixture와 어댑터 schema에 묶인 저장소 스냅샷을 읽고, fixture나 고정한 비교기
  설정이 바뀔 때만 `scripts/refresh-morph-baselines.sh`로 갱신한다.
- 제품 persona 비교는 같은 explicit-POS fixture와 gold를 사용한다. Agent와 외부 분석기는
  품사를 명시하고 User는 같은 query의 품사를 제거한 `full-POS + smart`로 실행한다. 이 결과는
  동일 입력의 backend 순위가 아니라 실제 입력 조건을 반영한 비교다.
- compact component artifact는 Homebrew의 `share/kfind`와 npm의 별도 정적 asset으로
  배포한다. WASM binary에는 artifact bytes를 포함하지 않는다.

## 품질 기준선

명시적 품사를 사용하는 1,000-case test의 현재 제품 결과다.

| lexicon | boundary | TP / FP / FN | precision | recall | F1 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| embedded | smart | 408 / 0 / 92 | 100.00% | 81.6% | 89.87% |
| full-POS | smart | 413 / 0 / 87 | 100.00% | 82.6% | 90.47% |
| embedded/full-POS | token | 354 / 0 / 146 | 100.00% | 70.8% | 82.90% |
| embedded/full-POS | any | 479 / 11 / 21 | 97.76% | 95.8% | 96.77% |

full-POS `smart`가 embedded보다 추가로 찾는 5건은 모두 명사다. `token`과 `any`에서는 두
lexicon profile의 품질이 같다. 세부 품사, 처리량, latency, RSS와 외부 분석기 비교는
[User smart precision 품질·성능](2026-07-14-user-smart-precision.md)을 기준으로 한다.

품사를 생략하는 사람용 1,000-case fixture에서 full-POS `smart`는 TP 410, FP 0, FN 90,
precision 100.00%, recall 82.0%, F1 90.11%다. embedded `smart`도 TP 315, FP 0, FN 185다.
embedded `smart`는 기대 품사를 plan에 포함하는
비율이 46.8%이므로 사람용 기본 경로를 대신하지 않는다.

explicit-POS test fixture의 품사를 제거한 User persona도 full-POS `smart`에서 TP 410, FP 0,
FN 90, precision 100.00%, recall 82.0%, F1 90.11%다. `이다 -> 매일`은 whole-token lexical
근거로, determiner query `이 -> 날씨가`는 무품사 조사 이형태 확장을 제한해 제거했다.
fixture·gold·지표 정의와 `any`의 TP 479 / FP 11 / FN 21은 바꾸지 않았다.

## 현재 경계

- `-기` 명사형은 token 경계에서 끝난다. `걷기`, `걷기 운동`은 찾지만 `걷기가`, `걷기를`의
  조사 continuation은 지원하지 않는다.
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

## 이어갈 작업

1. `-기` 명사형 뒤 조사 continuation을 독립 규칙과 hard-negative 단위로 다룬다.

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
cargo fmt --manifest-path tools/morph-index-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-index-benchmark/Cargo.toml \
  --all-targets -- -D warnings
cargo test --locked --manifest-path tools/morph-index-benchmark/Cargo.toml
scripts/benchmark-morphology.sh
scripts/benchmark-morph-index.sh
```
