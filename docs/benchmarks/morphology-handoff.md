# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 상태와 바로 이어갈 작업만 유지한다. 측정 과정과 완료한
작업 순서는 개별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [smart component 검색 근거](2026-07-13-smart-component-evidence.md)
- [지정사 lattice dev gold 진단](2026-07-13-copula-dev-diagnosis.md)
- [지정사 lattice 독립 평가](2026-07-13-copula-blind-evaluation.md)
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
- VCP 지정사 branch의 `EojeolLattice`는 shadow 계측 전용이다. 지정사 결과는 homonym union을
  유지하며 local lattice 비용으로 필터링하지 않는다.
- 지정사 필터링 후보는 `copula-lattice`, 제품 기본값은 `union`으로 고정된다.
  후보는 기존 compact resource의 include·exclude 최저 비용만 사용하며 추가 threshold를
  두지 않는다.
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

## 현재 경계

- `-기` 명사형은 token 경계에서 끝난다. `걷기`, `걷기 운동`은 찾지만 `걷기가`, `걷기를`의
  조사 continuation은 지원하지 않는다.
- 지정사 Korean-GSD fixture에는 source 정렬 불일치 2건을 제외하고 정상 VCP gold reject
  13개가 남아 있다. 이 fixture는 regression baseline이며 비용이나 threshold 선택에 사용하지
  않는다.
- Korean-Kaist·KSL dev의 gold-aligned lattice candidate 1,007건 중 50건이 reject다. 주원인은
  segmented nominal competitor 33건과 whole-window competitor 13건이며 embedded/full-POS가
  같다.
- 최종 unseen 입력은 UD Korean-PUD r2.18 test다. source-only 선택 계약은 양성 436개,
  음성 485개, excluded source copula 22개와 expected fixture SHA-256
  `d02cd5e78ebc4d02d626ead6206b3ed1dddc6d4c71d7a19543981699e45ebebd`를 고정한다.
- PUD source·license와 `pud-copula` adapter는 manifest에 고정되어 있고 Docker corpus build는
  921개 sealed fixture를 생성한다. backend 결과는 아직 관찰하지 않았다.
- `smart` component는 exact component span만 복구한다. `대학교`의 `학교`처럼 source 분석이
  component로 증명하지 않는 substring과 `역사과목`의 `사과`처럼 component 경계를 가로지르는
  span은 거부한다.
- component resource가 필요한 `smart` query의 fail-fast 동작은 호환성 계약이다. optional
  resource가 필요한 caller는 query compile 전에 resource를 준비해야 한다.

## 이어갈 작업

1. benchmark에 `copula-lattice` 제품 후보 투영을 추가한다. `accept`·`ambiguous`·
   `unresolved`는 유지하고 `reject`만 contextual origin에서 제거하며 비용 threshold는
   추가하지 않는다. config는 `unseen_local_context`, entrypoint는 `unseen_benchmark.py`,
   보고서는 schema 13의 `copula_policy_projection`을 사용한다.
2. 밀봉된 PUD fixture를 한 번 평가한다. gold recall 80.00%, target precision 99.00%,
   `unresolved` 0개, revised hard-negative 신규 FP 0개와 기존 품질 gate를 모두 통과하면
   CLI·Rust·WASM에 opt-in 정책을 구현한다. 실패하면 제품 옵션을 추가하지 않는다.
3. `-기` 명사형 뒤 조사 continuation은 지정사 판정 작업을 닫은 뒤 별도 규칙
   단위로 다룬다.

Kaist·KSL test, Korean-GSD 및 PUD 결과를 본 뒤 비용·threshold·fixture 선택을
변경하지 않는다.

## 재현과 검증

```console
scripts/benchmark-morphology.sh
KFIND_MORPH_BLIND=1 scripts/benchmark-morphology.sh target/morph-blind-report
KFIND_MORPH_UNSEEN=1 scripts/benchmark-morphology.sh target/morph-unseen-report
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
