# 형태소 검색 개선 핸드오프

기준 보고서: [2026-07-12 형태소 비교 분석](2026-07-12-morphology-comparison.md)

fixture SHA-256: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`

## 현재 상태

- kfind embedded profile: F1 82.67%, recall 70.60%, precision 99.72%
- 품질 순위: Kiwi 92.01% > Lindera 88.02% > kfind 82.67%
- kfind 비용: 17,616.5 cases/s, p95 0.1293 ms, peak RSS 4.9 MiB (5회 median)
- kfind 오류: FN 147, FP 1
- 가장 큰 FN 영역: 명사 71, 동사 33, 형용사 25

benchmark runner는 embedded/full-POS profile을 같은 fixture에서 비교한다. full-POS의
생산적 용언 활용을 보존하도록 수정한 뒤 test split에서 recovered 0, regressed 0이며 두
profile의 FN은 147개다.

## 2026-07-12 진행 결과

- FN 147개에 `primary_cause`와 판정 근거를 자동 기록한다: `boundary-rejected` 67,
  `continuation-rejected` 2, `gold-or-adapter` 23, `lexicon-missing` 50,
  `span-mismatch` 3, `surface-missing` 2.
- dev split을 별도 고정했다. ㅂ 불규칙 형용사 `가볍다`, `무겁다`, `무섭다`, `아쉽다`,
  `쉽다`, `춥다`를 dev 근거로 보강해 embedded recall이 70.60%에서 72.00%로 올랐고 test
  baseline은 변하지 않았다.
- 5개 slice, 10개 hard negative를 별도 metric으로 기록한다. embedded는 7 TN, 3 FP다.
- 기본 성능 측정은 1회 warm-up 뒤 5회 실행하고 median/min/max를 기록한다. CI는 28개 dev
  smoke case를 실행한다.
- copula FP는 [homonym union 정책](2026-07-12-copula-boundary-plan.md)을 유지하고 matcher를
  변경하지 않기로 확정했다.
- full POS artifact는 632,667개 entry와 614,794개 고유 표제어를 포함한다. dev의
  `lexicon-missing`은 embedded 38건, full-POS 0건이다.
- `-며/으며` 연결형을 보강해 dev TP가 360에서 361로 늘었고 recall은 72.20%다. test와
  hard-negative 결과는 변하지 않았다.
- MeCab의 문맥용 계사 표면형 14개를 표제어 후보에서 제외했다. `보이다`는 동사·보조 동사
  분석만 보존하고, 비정규 copula stem은 형태 생성 전에 거부한다.

dev 명사 FN 70개 중 64개는 사전 누락이 아니라 smart boundary 거부다. 합성어 substring
계약을 완화하면 hard-negative 정밀도와 충돌하므로 이번 어휘 보강에는 포함하지 않았다.
[명사 경계 계획](2026-07-12-nominal-boundary-plan.md)에서 별도 mode와 복합어 resource
선택지를 분리했다.

## 재현

```console
scripts/benchmark-morphology.sh
```

산출물:

- `target/morph-benchmark/report.json`: 모든 metric, 실패 case, match span
- `target/morph-benchmark/report.md`: 사람이 읽는 요약
- `docs/benchmarks/assets/morphology-quality.svg`: 품질 차트
- `docs/benchmarks/assets/morphology-performance.svg`: 성능 차트

입력 source, SHA-256, quota, seed는 `tools/morph-compare/sources.json`에 있다. Docker image
빌드 뒤 실제 평가는 `--network none`으로 실행된다.

## 작업 순서

### P0. 비교 profile을 분리한다 (완료)

목표: 사전 coverage 부족과 matcher 규칙 실패를 같은 FN으로 취급하지 않는다.

1. runner에 `embedded`와 `full-pos` kfind profile을 추가한다.
2. report의 version/profile metadata에 lexicon artifact SHA-256을 기록한다.
3. 동일 fixture에서 두 profile의 TP/FN, 초기화, RSS를 함께 출력한다.
4. full-POS에서 회복되는 case와 그대로 실패하는 case를 별도 목록으로 저장한다.

완료 조건:

- 같은 report에서 embedded/full-POS 결과가 명시적으로 구분된다.
- full-POS artifact가 없으면 조용히 embedded로 대체하지 않고 실패한다.
- profile별 fixture·case 순서가 동일하다.

### P0. 단일 false positive를 고정한다 (정책 계획 완료, 구현 보류)

case:

```text
query: 이다/adjective
text: 매일 아러바이트가도 있습니다.
observed span: 매일의 마지막 음절
```

한 음절 copula anchor가 복합 명사 내부에서 통과하는 경로를 `compile_query`의 branch와
`BoundaryVerifier` 양쪽에서 추적한다. `이다`의 올바른 copula 축약은 유지하면서 명사 내부
substring만 거부하는 회귀 fixture를 먼저 추가한다.

완료 조건:

- 위 case가 TN으로 바뀐다.
- 기존 copula positive와 contraction fixture가 모두 유지된다.
- `--boundary any`의 명시적 substring 동작은 바꾸지 않는다.

### P1. FN 147개를 원인별로 분류한다 (완료)

각 case에 다음 하나의 primary cause를 부여한다.

| 분류 | 판정 기준 |
| --- | --- |
| lexicon-missing | expected POS 분석이 query plan에 없음 |
| surface-missing | 분석은 있으나 gold 활용형 anchor가 생성되지 않음 |
| continuation-rejected | core anchor는 있으나 ending continuation이 거부됨 |
| boundary-rejected | 형태는 맞지만 smart boundary가 거부됨 |
| span-mismatch | 같은 lemma/POS를 찾았으나 gold 어절과 겹치지 않음 |
| gold-or-adapter | 세 도구가 모두 실패하거나 corpus 정규화가 의심됨 |

우선 113개 `kfind=false, Kiwi=true, Lindera=true` case를 분류한다. 세 도구가 모두 놓친
23개는 제품 규칙보다 gold/adapter audit를 먼저 수행한다. 분류 결과는 report의 failure
레코드에 기계 판독 가능한 필드로 남긴다.

### P1. 명사 coverage를 보강한다

명사 recall은 60.56%이고 FN은 71개다. full-POS profile 결과를 본 뒤 다음 순서로 처리한다.

1. full-POS로 회복되는 일반·고유·의존 명사를 분리한다.
2. full-POS에서도 실패하는 합성 명사는 left/right smart boundary와 particle continuation을
   확인한다.
3. corpus case를 core lexicon에 개별 추가하지 않는다. core 편입 기준을 만족하는 빈도·기능어만
   별도 제안한다.

개선 목표 후보는 dev split 명사 recall 80% 이상이다. 이 값은 합의 전 release gate가 아니다.

### P1. 용언 활용 실패를 보강한다 (진행 중)

동사 FN 33개와 형용사 FN 25개를 다음 slice로 나눈다.

- 규칙 활용 / 불규칙 활용
- 보조 용언
- 합성·파생 용언
- 관형형·연결형·종결형
- 학습자 오탈자

case별 surface를 일반 규칙으로 설명할 수 있을 때만 rule fixture로 승격한다. 특정 문장이나
특정 표제어만 통과시키는 예외 branch는 추가하지 않는다. dev split 목표 후보는 동사 recall
82%, 형용사 recall 78%이며 precision 하락은 2%p 이내로 둔다.

### P2. benchmark의 판별력을 높인다 (완료)

현재 negative는 쉬워 precision 차이가 거의 나타나지 않는다. 도구 출력과 독립적인 규칙으로
다음 hard-negative slice를 추가한다.

- 동음이의어의 다른 품사
- 합성어 내부 substring
- 잘못 붙여 쓴 앞말+용언
- 표면형은 같지만 lemma가 다른 활용
- 한 음절 query의 왼쪽·오른쪽 boundary

기존 1,000개 baseline은 유지하고 hard-negative 결과를 별도 metric으로 보고한다.

### P2. 성능 측정을 반복 가능하게 만든다 (완료)

- backend별 1회 warm-up 뒤 최소 5회 반복한다.
- median, p95와 run 간 min/max를 기록한다.
- embedded/full-POS profile의 RSS와 처리량을 분리한다.
- CI에서는 작은 smoke set, 수동 benchmark에서는 전체 set을 사용한다.

## 데이터 누수 방지

현재 test failure 목록이 문서와 JSON에 노출되었다. 이후 구현을 이 목록에 맞춰 조정하면 이
test split은 더 이상 독립 검증 집합이 아니다.

- 규칙 개발과 threshold 선택은 Kaist/KSL dev split에서 수행한다.
- 현재 test split은 regression baseline으로만 유지한다.
- 품질 개선 주장에는 아직 보지 않은 source 또는 고정 blind subset을 추가한다.
- blind 결과를 확인한 뒤 case별 예외를 추가하지 않는다.

## 변경 시 필수 검증

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo fmt --manifest-path tools/morph-compare/runner/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-compare/runner/Cargo.toml \
  --all-targets -- -D warnings
scripts/benchmark-morphology.sh
```

report의 fixture SHA-256, source hash, case 수, class/source/POS quota가 바뀌면 의도된 dataset
변경인지 먼저 확인한다. 품질 개선은 전체 F1만 보지 말고 POS별 recall, hard-negative
precision, initialization, p95, RSS를 함께 비교한다.
