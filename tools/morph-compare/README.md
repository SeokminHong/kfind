# 독립 형태 품질·성능 벤치마크

동일한 held-out 사례에서 kfind의 embedded/full-POS profile과 Kiwi, Lindera,
MeCab-ko, KOMORAN의 고정 adapter 결과를 비교합니다. 외부 분석기와 corpus는 제품
binary와 기본 검색 경로에 포함되지 않습니다.

## 평가 fixture

fixture는 Universal Dependencies 2.18의 Korean-Kaist와 Korean-KSL test/dev
split에서 생성합니다. URL, SHA-256과 CC BY-SA 4.0 라이선스는 `sources.json`에
고정합니다.

- Canonical은 검토한 표준 맞춤법 문장으로 구성한 positive 500개와 negative 500개입니다.
- Human untagged는 같은 규모에서 품사를 생략한 사람용 검색 질의를 평가합니다.
- Query matrix는 canonical positive 문장마다 정렬된 존재 질의와 같은 품사의 부재 질의를 함께 적용합니다.
- Robust는 검토한 Korean-KSL 실제 오류 문장의 positive 250개와 negative 250개입니다.
- Real corpus는 고정 revision의 한국어 README, source comment와 기술 문서 excerpt를 사용합니다.

Canonical, Query matrix, Robust와 Human untagged는 기대값과 입력 계약이 다르므로
점수를 합치지 않습니다.

## Raw와 contract-adjusted

모든 품질 backend에는 raw와 contract-adjusted confusion matrix, precision,
recall과 F1을 함께 기록합니다. 이 계약은 kfind뿐 아니라 Kiwi, Lindera, MeCab-ko와
KOMORAN에도 동일하게 적용합니다.

Raw의 TP, TN, FP, FN은 원본 corpus gold를 그대로 사용합니다. Contract-adjusted의
TPc, TNc, FPc, FNc는 실행 전에 고정한 review registry를 같은 예측에 적용합니다.
Registry는 의미로 구분할 수 없는 동형이의, source에 정렬된 내부 성분과 gold span
오류를 재분류하며 제품 입력 계약 밖의 비표준 사례를 제외할 수 있습니다. 구현이
어렵거나 지원하지 않는 표준 문법은 제외하지 않습니다.

Review registry가 없는 fixture도 두 지표를 모두 기록합니다. 이 경우 raw confusion
matrix를 contract-adjusted 값으로 사용하고 `reviewed_cases=0`을 명시합니다.
Contract-adjusted 값만 표시하거나 raw 오류를 숨기지 않습니다.

예를 들어 raw FN이 4이고 FNc가 0이면 네 누락이 관측되었지만 review 결과 모두 제품
목표 밖이라는 뜻입니다. 실행 결과에서 네 오류가 사라졌다는 뜻이 아니며, 계약 안의
false negative가 0이라는 뜻입니다.

## 공식 실행

저장소 root에서 wrapper를 실행합니다.

```sh
scripts/benchmark-morphology.sh
```

기본 실행은 각 kfind profile을 fresh process에서 warm-up 1회 뒤 5회 측정합니다.
외부 분석기는 실행하지 않고 test fixture SHA-256에 묶인 version-controlled
snapshot을 읽습니다. 결과는 `target/morph-benchmark/report.json`과 `report.md`에
생성됩니다. stdout은 phase와 결과 경로만 사용하고 실패·진단은 stderr에
출력합니다. 상세 로그에는 `KFIND_MORPH_VERBOSE=1`을 사용합니다.

```sh
KFIND_MORPH_SMOKE=1 KFIND_MORPH_RUNS=1 scripts/benchmark-morphology.sh
```

Smoke set은 dev fixture의 source/POS/expected 조합마다 첫 사례를 선택합니다.
Docker image를 만든 뒤 benchmark container는 `--network none`으로 실행합니다.
full-POS artifact의 checksum이 맞지 않으면 embedded profile로 대체하지 않고
실패합니다.

로컬 Python test는 이 디렉터리에서 실행합니다.

```sh
cd tools/morph-compare
python3 -m unittest discover --start-directory python --pattern 'test_*.py'
```

## 외부 snapshot

fixture, 평가 schema 또는 고정 외부 도구·adapter 설정이 바뀐 경우에만 외부
snapshot을 갱신합니다.

```sh
scripts/refresh-morph-baselines.sh
```

일반 benchmark는 외부 analyzer를 빌드하지 않습니다. fixture 또는 schema가
snapshot과 맞지 않으면 갱신 명령을 안내하고 실패합니다.

## 보고서 계약

품질 보고서는 backend와 평가군마다 다음 항목을 기록합니다.

- raw TP, TN, FP, FN, precision, recall, F1
- contract-adjusted TPc, TNc, FPc, FNc, precision, recall, F1
- reviewed, reclassified, excluded case 수와 registry SHA-256
- source/POS/noise scope별 결과와 case-level failure cause
- kfind resource version과 artifact SHA-256

성능 보고서는 fresh process의 initialization, cases/s, p95 latency와 peak RSS를
기록합니다. warm-up 수, 측정 수, median/min/max를 함께 남깁니다. 품질과 성능을
하나의 점수로 합치지 않습니다. Agent, User와 외부 adapter의 입력 조건도 각 행에
명시합니다.

Query matrix raw FN disposition은 report의 fixture hash, backend, case ID, 질의,
품사, gold byte span과 failure cause가 모두 맞아야 합니다.

```sh
python3 tools/morph-compare/validate_fnc_dispositions.py \
  target/morph-benchmark/report.json \
  docs/benchmarks/query-matrix-fnc-dispositions.tsv
```

사이트 snapshot은 승인한 JSON report에서 생성합니다.

```sh
python3 tools/morph-compare/export_site_snapshot.py \
  target/morph-benchmark/report.json \
  docs/benchmarks/site-morphology.json \
  --revision "$(git rev-parse HEAD)"
```

사이트는 snapshot의 raw와 contract-adjusted 값을 D3로 렌더링합니다. 정적 SVG를
별도 생성하지 않습니다.

## 실제 기술 corpus

원본 파일과 excerpt를 검증합니다.

```sh
python3 tools/morph-compare/real_corpus/verify_sources.py

KFIND_BENCH_REVISION=$(git rev-parse HEAD) \
  tools/morph-compare/real_corpus/run.sh
```

결과는 기본적으로 `target/real-corpus-blind`에 생성합니다. 이 fixture는 Canonical
회귀 fixture를 대체하거나 제품 규칙 선택에 사용하지 않습니다.

세부 측정·보고 규칙은 [`docs/benchmarks/README.md`](../../docs/benchmarks/README.md)를
따릅니다.
