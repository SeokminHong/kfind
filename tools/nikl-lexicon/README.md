# 국립국어원 용언 사전 importer

고정한 국립국어원 사전 snapshot에서 검토된 `DToL`, `DropS`, `BToWa`, `BToWo`,
`DropH`, `ReuDoubleL`, `Reo`, `UToEo` 용언 계층을 생성합니다. 같은 모양의 규칙
활용과 규칙 `EU_DROP`은 control로 보존합니다. 동일 표제어·세부 품사에서 독립된
record가 불규칙 분석과 규칙 분석을 함께 지지하는 경우에만 규칙 분석을 냅니다.

Python adapter는 세 ZIP snapshot을 읽고 source record와 동형이의 identity를 보존한
정규화 용언 record를 만듭니다. 형용사에서 부사로의 후보는 `-없다`·`-같다`와 `이`,
`르`와 `ㄹ리`, ㅂ 받침이 탈락한 어간과 `이`를 대조합니다. 한국어기초사전과
표준국어대사전이 형용사와 결과 부사를 모두 독립적으로 등재한 경우에만
기록합니다.

한국어기초사전의 양방향 파생 관계 중 target 동사가 `-이/-히/-리/-기-` 형태인
피동·사동 관계도 읽습니다. Rust classifier는 kfind 용언 generator로 진단 활용을
만들고 두 사전의 합의를 확인합니다. 우리말샘은 audit evidence로 보존합니다. Core
중복과 생산적 접사 규칙으로 만들 수 있는 분석은 report에만 기록하고 배포
artifact에서는 제외합니다.

```sh
scripts/build-enriched-predicates.sh
```

생성은 `target` 아래 reusable candidate를 먼저 만들고 배포 크기를 검증한 뒤
설치합니다. 실패한 candidate는 보존합니다. 사전 데이터를 다시 만들지 않고 검증
정책만 적용하려면 다음 명령을 사용합니다.

```sh
scripts/install-enriched-predicates.sh target/kfind-enriched-candidate.XXXXXX
```

고정 ZIP이 `~/Downloads` 밖에 있으면 `KFIND_NIKL_DOWNLOADS`를 지정합니다. Snapshot은
`${KFIND_NIKL_CACHE:-${XDG_CACHE_HOME:-~/.cache}/kfind/nikl}`에 한 번 풀고 SHA-256이
같은 동안 재사용합니다. Raw snapshot과 사전 용례는 저장소에 복사하지 않습니다.

## 어미 catalog

`scripts/audit-nikl-endings.sh`는 같은 snapshot에서
`data/rules/nikl-modern-endings.tsv`를 생성합니다. 한국어기초사전과
표준국어대사전의 어미 표제어를 runtime catalog로 사용하고, 우리말샘 ID는
provenance로 보존합니다. 생성 파일에는 정규화 표면형, 원 표제어, 문법 범주와
source record ID만 들어 있습니다.

Runtime compiler는 catalog를 어휘 목록으로 사용합니다. 모든 어미를 모든 어간에
결합하는 허가 목록으로 사용하지 않습니다. 종성 조건, 불규칙 교체, 어미 순서,
보조 용언 경로와 전체 표제어 충돌은 별도 구조 조건입니다.

### 검색 범위 검사

`audit_ending_runtime.py`는 catalog의 모든 표면형을 순회합니다. 고정 사전의
용례에서 어미 위치와 용언 표제어가 정렬되는 사례를 runner 입력으로 만듭니다.
사전이 본말을 명시한 준말은 본말 용례에서 검사 입력을 만들 수 있습니다.
`--kiwi`를 지정하면 외부 형태 분석으로 용언 후보를 보강합니다. 이 경우
`tools/morph-compare/external/Dockerfile`에 지정된 Kiwi와 모델 버전을 사용합니다.
결과는 `target` 아래에 저장하며, 자동 정렬 결과는 어미 결합 규칙을 검토할 후보로 사용합니다.
Kiwi가 지정사 `이`의 원문 위치를 식별하면 체언 뒤의 지정사부터 어절 끝까지 비교합니다.
자동 정렬이 어려운 용례는 `--reviewed`로 추가합니다. TSV 열은 `ending`, `query`,
`pos`, `surface`, `source_id`, `alignment`, `text` 순서이며 `surface`의 첫 출현을 표적으로
사용합니다.

```sh
downloads=${KFIND_NIKL_DOWNLOADS:-"$HOME/Downloads"}
cache=${KFIND_NIKL_CACHE:-"${XDG_CACHE_HOME:-"$HOME/.cache"}/kfind/nikl"}
output=target/ending-runtime
python3 tools/nikl-lexicon/audit_ending_runtime.py \
  --krdict "$downloads/전체 내려받기_한국어기초사전_xml_20260619.zip" \
  --stdict "$downloads/전체 내려받기_표준국어대사전_xml_20260705.zip" \
  --opendict "$downloads/전체 내려받기_우리말샘_xml_20260702.zip" \
  --cache-dir "$cache" --catalog data/rules/nikl-modern-endings.tsv \
  --output-dir "$output"

cargo build --release --locked --manifest-path tools/morph-compare/runner/Cargo.toml
runner=tools/morph-compare/runner/target/release/morph-benchmark-runner
for boundary in smart any; do
  "$runner" boundary full-pos "$boundary" "$output/cases.jsonl" "$output/$boundary.json"
done
python3 tools/nikl-lexicon/report_ending_runtime.py \
  --directory "$output" --revision "$(git rev-parse HEAD)"
```

Runner에는 `KFIND_FULL_POS_LEXICON`, `KFIND_ENRICHED_PREDICATES`와
`KFIND_COMPONENT_RESOURCE`로 사용하는 사전과 component 파일을 전달합니다.
`report.json`은 어미별 전체 일치, 부분 일치, 누락, 용례 정렬이 필요한 항목과
표준어 참조 표제어를 기록합니다. 표적 범위의 일부만 반환하는 결과도 검색 근거와 함께
남습니다.

## 조사 catalog

`scripts/audit-nikl-particles.sh`는 같은 snapshot에서
`data/rules/nikl-modern-particles.tsv`와 runtime coverage report를 생성합니다.
구조화한 표제어, 품사, 상태와 source ID만 사용하며 정의와 용례는 수집하지
않습니다.

Catalog는 원자 조사 어휘와 host class를 기록합니다. 조사 결합은
`data/rules/particles.toml`의 role, host와 transition으로만 허용합니다. 예를 들어
`까지도`는 새 원자형이 아니라 `까지 → 도` 연쇄입니다.

## 어휘 항목 audit

`audit_lexemes.py`는 명시한 표제어를 세 snapshot에서 찾아 구조화 품사와 lexical
relation을 JSON으로 기록합니다. 사전 기반 표제어·파생·준말 근거와 corpus 용례나
자유 text 정의를 구분해야 하는 failure triage에 사용합니다.
