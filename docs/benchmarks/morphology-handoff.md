# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 상태와 바로 이어갈 작업만 유지한다. 측정 과정과 완료한
작업 순서는 개별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [smart component 검색 근거](2026-07-13-smart-component-evidence.md)
- [copula lattice 폐기 판정](2026-07-13-copula-unseen-evaluation.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)

## 제품 상태

- CLI, Rust library와 WASM binding은 같은 query compiler와 matcher를 사용한다.
- 사람용 CLI 기본 경로는 full POS와 `smart`다. 품사를 명시하는 자동화 경로는
  `--boundary any --embedded --json`을 사용한다.
- `smart`의 명사 branch는 문자열 token 경계 또는 compact component resource의 완전한 형태
  component 근거가 있어야 한다. component 경계를 가로지르는 substring은 거부한다.
- CLI는 `NominalComponent` branch가 있는 plan에서 compact component resource를 자동으로
  해석한다. 필요 resource의 누락·손상·schema 또는 source 불일치는 초기화 오류이며 경계
  판정으로 fallback하지 않는다.
- Rust/WASM engine은 full POS와 component bytes를 자동으로 찾지 않는다. caller가 생성자나
  load API로 명시하며, resource가 없는 component `smart` compile은 오류다.
- `이다/아니다` 계열은 homonym union을 유지한다. copula 전용 lattice 분기와 shadow 계측,
  PUD/GSD 전용 실행 경로는 유지하지 않는다.
- 기본 morphology benchmark는 kfind 프로필만 다시 실행한다. Kiwi·Lindera·MeCab-ko·KOMORAN
  품질은 test fixture와 어댑터 schema에 묶인 저장소 스냅샷을 읽고, fixture나 고정한 비교기
  설정이 바뀔 때만 `scripts/refresh-morph-baselines.sh`로 갱신한다.
- persona 진단은 같은 explicit-POS fixture와 gold를 사용한다. Agent와 외부 분석기는 품사를
  명시하고 User는 같은 query의 품사를 제거한 `full-POS + smart`로 실행한다. User 행은 자동
  계획과 explicit-POS gold의 의미 차이를 보는 진단이며 제품 품질 gate나 backend 순위가 아니다.
- compact component artifact는 Homebrew의 `share/kfind`와 npm의 별도 정적 asset으로
  배포한다. WASM binary에는 artifact bytes를 포함하지 않는다.

## 품질 기준선

명시적 품사를 사용하는 1,000-case test의 현재 제품 결과다.

| lexicon | boundary | TP / FP / FN | precision | recall | F1 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| embedded | smart | 408 / 1 / 92 | 99.76% | 81.6% | 89.77% |
| full-POS | smart | 413 / 1 / 87 | 99.76% | 82.6% | 90.37% |
| embedded/full-POS | token | 354 / 0 / 146 | 100.00% | 70.8% | 82.90% |
| embedded/full-POS | any | 479 / 11 / 21 | 97.76% | 95.8% | 96.77% |

full-POS `smart`가 embedded보다 추가로 찾는 5건은 모두 명사다. `token`과 `any`에서는 두
lexicon profile의 품질이 같다. 세부 품사, 처리량, latency, RSS와 외부 분석기 비교는
[smart component 검색 근거](2026-07-13-smart-component-evidence.md)를 기준으로 한다.

품사를 생략하는 사람용 1,000-case fixture에서 full-POS `smart`는 TP 410, FP 1, FN 90,
precision 99.76%, recall 82.0%, F1 90.01%다. embedded `smart`는 기대 품사를 plan에 포함하는
비율이 46.8%이므로 사람용 기본 경로를 대신하지 않는다.

explicit-POS test fixture의 품사를 제거한 User persona는 full-POS `smart`에서 TP 410, FP 2,
FN 90, precision 99.51%, recall 82.0%다. 오탐은 `이다 -> 매일`의 corpus homonym 1건과
`이`의 조사 이형태 match를 determiner gold로 평가한 query POS ambiguity 1건이다. dev의 5건도
corpus homonym 1건과 query POS ambiguity 4건으로 나뉜다. query POS ambiguity는 User 제품
false positive로 보지 않는다.

`whole-token-lexical` shadow는 dev의 TP 433 / FP 5 / FN 67을 바꾸지 않는다. test에서는
`이다 -> 매일` 1건만 제거해 TP 410 / FP 1 / FN 90, precision 99.76%, recall 82.0%가 된다.
dev 이득이 없으므로 제품 verifier로 승격하지 않는다. `살다 -> 사실`은 compact resource에도
`VV+EP+ETM` exact 분석이 있어 whole-token 사전 존재만으로 안전하게 제거할 수 없다.

## User precision 결정

- 사람용 `--pos auto`는 여러 coarse POS 분석의 합집합을 유지한다. query 문자열만으로 단일
  품사를 고르거나 검색 전에 품사 선택을 요구하지 않는다.
- User 제품 precision의 기준은 별도 무품사 fixture다. negative 문장에는 query 표제어가 지원하는
  어떤 품사로도 존재하지 않아야 한다.
- explicit-POS fixture에서 품사만 제거한 persona 행은 교차-persona 진단으로 유지한다.
  `query-pos-ambiguity`는 별도 집계하되 User precision gate와 backend 순위에 넣지 않는다.
- 실제 User precision backlog는 `corpus-homonym`이다. 기존 `any` candidate의 부분집합만
  선택하며 copula lattice를 복원하거나 새 span을 만들지 않는다.

## 현재 경계

- `-기` 명사형은 token 경계에서 끝난다. `걷기`, `걷기 운동`은 찾지만 `걷기가`, `걷기를`의
  조사 continuation은 지원하지 않는다.
- `smart` component는 exact component span만 복구한다. `대학교`의 `학교`처럼 source 분석이
  component로 증명하지 않는 substring과 `역사과목`의 `사과`처럼 component 경계를 가로지르는
  span은 거부한다.
- component resource가 필요한 `smart` query의 fail-fast 동작은 호환성 계약이다. optional
  resource가 필요한 caller는 query compile 전에 resource를 준비해야 한다.

## 이어갈 작업

1. Agent의 `embedded + any + 명시적 품사` 기준선 TP 479 / FP 11 / FN 21에서 FP를 원인별로
   분류한다. 첫 구현은 현재 `any` match만 제거할 수 있는 report-only shadow로 두고 matcher
   결과와 candidate coverage를 바꾸지 않는다.
2. Agent shadow는 `tools/morph-compare/runner/src/main.rs`에서 evidence를 내고
   `tools/morph-compare/python/report.py`에서 dev를 먼저 판정한다. test는 회귀 기록에만 사용하며
   제품 승격 전에 문장 hash가 겹치지 않는 새 sealed unseen source를 고정한다.
3. `tools/morph-compare/runner/src/user_precision_shadow.rs`의 `whole-token-lexical` projection은
   dev에서 recall을 잃지 않는 근거가 생기기 전까지 shadow로 유지한다. 이미 판정을 마친 PUD와
   GSD를 새 sealed unseen처럼 재사용하지 않으며 copula lattice를 복원하지 않는다.
4. `-기` 명사형 뒤 조사 continuation을 독립 규칙과 hard-negative 단위로 다룬다.

다음 작업은 최신 `main`에서 이 문서와 `specs/kfind.md`를 다시 읽고 시작한다. 보고서 계약을
기준으로 Agent shadow를 만드는 동안 matcher와 `crates/kfind-morph/src/lattice.rs`는 수정하지
않는다. 검증 순서는 Python 보고서 unit test, runner test, workspace fmt·clippy·test, Docker
morphology smoke와 full benchmark다.

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
