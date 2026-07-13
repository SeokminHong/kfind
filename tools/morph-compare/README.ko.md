# 독립 형태소 벤치마크

[English](README.md)

이 개발 도구는 동일한 held-out 사례를 `kfind` embedded/full-POS 프로필,
Kiwi, Lindera로 처리한다. 외부 분석기와
corpus는 제품 바이너리나 기본 검색 경로에 포함되지 않는다.

fixture는 Universal Dependencies 2.18의 Korean-Kaist와 Korean-KSL test/dev split에서
생성한다. URL, SHA-256, CC BY-SA 4.0 라이선스는 `sources.json`에 고정되어 있다. split별로
각 source에서 품사별 250개 positive를 선택하고 같은 source의 deterministic negative를
대응시켜 총 1,000개를 만든다. 개발은 dev split으로 수행하고 test split은 regression
baseline으로 유지한다.
이미지는 사람의 무품사 사용을 위한 별도 1,000-case fixture도 만든다. 쿼리는 품사를
생략하며, negative 문장에는 해당 표제어가 지원하는 어떤 품사로도 존재하지 않는다.
이미지 빌드는 밀봉된 Korean-GSD blind local-context fixture도 생성·검증한다. 기본
벤치마크는 이 fixture를 로드하거나 평가하지 않는다.
이 fixture는 regression 확인에만 사용한다.

```sh
KFIND_MORPH_BLIND=1 scripts/benchmark-morphology.sh target/morph-blind-report
```

```sh
scripts/benchmark-morphology.sh
```

기본 실행은 backend별 1회 warm-up 뒤 5회 측정한다. 결과는
`target/morph-benchmark/report.json`과 `report.md`에 생성된다. 이미지를 빌드한 뒤
컨테이너는 `--network none`으로 실행된다. `scripts/compare-morphology.sh`도 같은
벤치마크를 실행한다. 이미지 빌드는 고정 checksum의 full-POS artifact를 생성하며,
artifact가 없거나 검증에 실패하면 벤치마크를 중단한다.

CI용 deterministic smoke set은 dev fixture의 source/POS/expected 조합마다 첫 case를
선택한다.

```sh
KFIND_MORPH_SMOKE=1 KFIND_MORPH_RUNS=1 scripts/benchmark-morphology.sh
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

세 도구 모두 문장 안에 gold 표제어·품사가 존재하는지 판정한다. positive는 예측 span이
gold 어절 span과 겹쳐야 하며, negative는 같은 표제어·품사를 반환하면 false positive다.
보고서는 accuracy, precision, recall, F1, source/POS별 결과, 실패 span과 초기화·처리량·
지연·peak RSS를 기록한다. test 결과에는 dev 결과와 5개 slice의 version-controlled
hard-negative 결과가 함께 포함된다. kfind false negative에는 자동 판정한
`primary_cause`와 근거를 남긴다. `kfind` 프로필별 버전·artifact SHA-256와 full-POS에서
회복된 false negative, 계속 실패한 false negative, 새로 발생한 false negative를 별도
목록으로 남긴다.
shadow 검증은 성능 측정 구간 밖에서 case별 raw anchor hit, verifier 통과 branch hit,
local-lattice 대상과 고유 분석 어절 수를 기록한다.

성능 수치는 각 도구의 질의부터 결과 판정까지 처리한 제품 작업량이다. 보고서는 측정 run의
median과 min/max를 기록한다. 순수 tokenizer 처리량 비교가 아니다. 전체 test 보고서는 두
kfind lexicon profile의 smart, token, any도 비교하며 smart만 component resource를 로드한다.
별도 startup 표는 resource 없는 embedded/full-POS engine과 같은 engine에서 component를
명시적으로 로드한 경우를 비교한다. 각 profile은 새 process에서 1회 warm-up 뒤 초기화 시간과
peak RSS를 최소 3회 측정한다.

`Human untagged search` 절은 embedded/full-POS와 smart/any를 별도로 비교한다. 품질·성능과
함께 기대 품사 plan 포함률, multi-POS plan 비율, literal fallback 비율을 기록한다. negative
정의가 다른 명시적 품사 task와 F1 순위를 합치지 않는다.
