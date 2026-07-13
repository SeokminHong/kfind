# 독립 형태소 벤치마크

[English](README.md)

이 개발 도구는 동일한 held-out 사례에서 `kfind` embedded/full-POS 프로필을 실행하고,
Kiwi·Lindera·MeCab-ko·KOMORAN의 고정 품질·성능 스냅샷과 비교한다. 외부 분석기와 corpus는
제품 바이너리나 기본 검색 경로에 포함되지 않는다.

fixture는 Universal Dependencies 2.18의 Korean-Kaist와 Korean-KSL test/dev split에서
생성한다. URL, SHA-256, CC BY-SA 4.0 라이선스는 `sources.json`에 고정되어 있다. split별로
각 source에서 품사별 250개 positive를 선택하고 같은 source의 deterministic negative를
대응시켜 총 1,000개를 만든다. 개발은 dev split으로 수행하고 test split은 regression
baseline으로 유지한다.
이미지는 사람의 무품사 사용을 위한 별도 1,000-case fixture도 만든다. 쿼리는 품사를
생략하며, negative 문장에는 해당 표제어가 지원하는 어떤 품사로도 존재하지 않는다.

```sh
scripts/benchmark-morphology.sh
```

기본 실행은 kfind 프로필마다 1회 warm-up 뒤 5회 측정하고 외부 분석기는 실행하지 않는다.
test fixture SHA-256에 맞는 version-controlled 스냅샷만 읽는다. 결과는
`target/morph-benchmark/report.json`과 `report.md`에 생성된다. 이미지를 빌드한 뒤
컨테이너는 `--network none`으로 실행된다. `scripts/compare-morphology.sh`도 같은
벤치마크를 실행한다. 이미지 빌드는 고정 checksum의 full-POS artifact를 생성하며,
artifact가 없거나 검증에 실패하면 벤치마크를 중단한다.

CI용 deterministic smoke set은 dev fixture의 source/POS/expected 조합마다 첫 case를
선택한다.

```sh
KFIND_MORPH_SMOKE=1 KFIND_MORPH_RUNS=1 scripts/benchmark-morphology.sh
```

test fixture, 성능 schema나 고정한 외부 도구·어댑터 설정을 바꿀 때만 외부 스냅샷을
명시적으로 갱신한다. 기본 벤치마크는 fixture 또는 schema가 맞지 않으면 갱신 명령을
안내하고 실패한다.

```sh
scripts/refresh-morph-baselines.sh
```

같은 JSON에서 문서용 차트를 재현한다.

```sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets \
  --prefix smart-component-
```

[현재 제품 근거](../../docs/benchmarks/2026-07-13-smart-component-evidence.md)와
[개선 핸드오프](../../docs/benchmarks/morphology-handoff.md)에 결과와 후속 작업을
정리한다.

직접 실행하려면 다음 명령을 사용한다.

```sh
docker build -f tools/morph-compare/Dockerfile -t kfind-morph-benchmark:local .
mkdir -p target/morph-benchmark
docker run --rm --network none \
  --user "$(id -u):$(id -g)" \
  -v "$PWD/target/morph-benchmark:/output" \
  kfind-morph-benchmark:local
```

각 결과는 문장 안에 gold 표제어·품사가 존재하는지 판정한다. positive는 예측 span이
gold 어절 span과 겹쳐야 하며, negative는 같은 표제어·품사를 반환하면 false positive다.
보고서는 accuracy, precision, recall, F1, source/POS별 결과, 실패 span과 초기화·처리량·
지연·peak RSS를 기록한다. test 결과에는 dev 결과와 5개 slice의 version-controlled
hard-negative 결과가 함께 포함된다. kfind false negative에는 자동 판정한
`primary_cause`와 근거를 남긴다. `kfind` 프로필별 버전·artifact SHA-256와 full-POS에서
회복된 false negative, 계속 실패한 false negative, 새로 발생한 false negative를 별도
목록으로 남긴다.
shadow 검증은 성능 측정 구간 밖에서 case별 raw anchor hit, verifier 통과 branch hit,
local-lattice 대상과 고유 분석 어절 수를 기록한다.

현재 성능 수치는 kfind의 질의부터 결과 판정까지 처리한 제품 작업량이다. 보고서는 측정 run의
median과 min/max를 기록한다. 제품 persona 비교는 Agent, User와 외부 분석기 4종에 같은
explicit-POS fixture와 gold를 사용한다. Agent와 외부 adapter는 품사를 명시하고 User는 같은
query에서 품사를 제거한 `full-POS + smart`로 실행한다. 동일 입력의 backend 순위가 아니라 실제
persona 입력을 반영한 제품 비교다. 전체 test 보고서는 두 kfind lexicon profile의 smart,
token, any도 비교하며 smart만 component resource를 로드한다.
별도 startup 표는 resource 없는 embedded/full-POS engine과 같은 engine에서 component를
명시적으로 로드한 경우를 비교한다. 각 profile은 새 process에서 1회 warm-up 뒤 초기화 시간과
peak RSS를 최소 3회 측정한다.

`Human untagged search` 절은 embedded/full-POS와 smart/any를 별도로 비교한다. 품질·성능과
함께 기대 품사 plan 포함률, multi-POS plan 비율, literal fallback 비율을 기록한다. negative
정의가 다른 명시적 품사 task와 F1 순위를 합치지 않는다.

`User precision shadow` 절은 query POS ambiguity와 corpus homonym을 분리하고 dev·test의
baseline 대비 projected precision·recall을 기록한다. whole-token lexical projection은 기존
strict-subspan predicate candidate만 제거하며 제품 결과와 성능 측정 구간에는 영향을 주지 않는다.

`Product workflows` 절은 에이전트용 `embedded + any + 명시적 품사`의 recall·처리량과
false-positive 후보 수, 사람용 `full-POS + smart + 무품사`의 precision·recall·plan 포함률을
먼저 보여 준다. 라이브러리는 resource 없는 embedded engine을 기본값으로 두고 full-POS와
component resource를 선택 비용으로 분리한다. 이 workflow들을 하나의 점수로 합치지 않는다.

`Product CLI use cases` 절은 100 MiB·1,000파일 고정 코퍼스에서 두 workflow를 독립 CLI
process로 실행한다. 시작, query compile, 파일 순회, scan, verification과 출력 직렬화를 포함한
wall time·처리량·peak RSS를 기록한다. 라이브러리 resource 조합의 초기화 비용은 이 측정과
합산하지 않는다. 같은 JSON의 `product-use-cases.svg`가 두 비용을 분리해 보여 준다.
`product-workflows.svg`는 profile별 precision·recall·F1·false-positive 후보 수와 실제 CLI
wall time·처리량·peak RSS를 함께 표시하되 서로 다른 fixture와 corpus임을 명시한다.
`product-external-comparison.svg`는 Agent, User, Kiwi, Lindera, MeCab-ko, KOMORAN의
precision·recall·F1, 초기화, 처리량, p95와 peak RSS를 비교한다. 행 label에는 persona 또는
backend명만 표시하고 입력 조건은 차트 옆 문서에 둔다.
