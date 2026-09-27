# 규칙 데이터와 사전

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

## 0. 제품 계약

이 절은 뒤의 일반 설명보다 우선한다.

### 0.1 규칙 데이터와 품질 기준

- v0.1의 필수 형태 범위는 [9.5절의 활용표](matching.md), [19.2절의 필수 테스트](verification.md), [23절의 인수 기준](kfind.md)을 모두 포함한다.
- gold corpus에 포함된 현재 평서형 `-ㄴ다/는다`와 상태 용언의 `-다`, 회상 관형형 `-던`, 양보 연결형 `-더라도`, 과거 관형 연쇄 `-았/었을`, 과거 의문 종결 연쇄 `-았/었느냐`, `-았/었느냐는`, 이유 연결형 `-(으)니`, 인용 연결형 `-다고`, 현재 서술형 인용·회상·조건 연쇄, 전망 종결형 `-(으)리라`와 인용 연쇄 `-(으)리라고`, 의도 연결형 `-(으)려고`, 상태 변화 보조 용언 `-아/어지다`, 진행 방향 보조 용언 `-아/어가고`, `-아/어가야`도 v0.1의 제한된 continuation vocabulary에 포함한다.
- 실제 코퍼스에서 확인된 해요체 과거형 `-았어요/-었어요`, 지정사 `이다`의 높임 평서형
  `입니다`와 인용·관형·대조·나열형 `이라고`·`이라는`·`이지`·`이며`, 부정 지정사 `아니다`의
  연결형 `아니라`도 v0.1의 제한된 continuation vocabulary에 포함한다. 지정사 확장은 이 네
  완성형만 직접 생성하며 무표면 축약 `겁니다`와 비표준 `이예요`를 합치지 않는다.
- 어미, 조사 연쇄, 파생 규칙은 저장소에서 버전을 관리하는 `data/rules` 파일의 목록과 전이를
  기준으로 삼는다. 목록 밖 조합은 생성하지 않는다.
- full POS lexicon은 `mecab-ko-dic 2.1.1-20180720`의 Apache-2.0 데이터를 bootstrap 원본으로 사용한다. 빌드 시 표제어와 품사만 추출하고, 런타임 문장 분석 데이터와 알고리즘은 포함하지 않는다. `Inflect`와 `Preanalysis` 행은 제외하며, 문맥용 지정사 표면형은 표제어로 승격하지 않고 `VCP=이`, `VCN=아니`만 기본형으로 정규화한다.
- full POS lexicon의 용언 품사 후보도 POS 전용 산출물에 보존한다. 동일 표제어와 coarse 품사에
  core 또는 enriched 용언 분석이 하나라도 있으면 그 coarse 품사의 full POS 규칙형 분석은
  추가하지 않는다. 다른 coarse 품사는 보존한다. 그 밖의 용언은 해당 품사와 일치하는 생산적
  접미 규칙을 먼저 적용하고, 일치하는 규칙이 없을 때만 제한된 규칙형 분석을 사용한다.
- full POS runtime resource는 검증된 정렬 lookup index로 보존한다. CLI, Rust library와 WASM
  binding은 초기화할 때 전체 entry를 일반 분석 map이나 entry별 소유 문자열로 전개하지 않는다.
  Front-compressed 표제어는 하나의 재사용 문자열 scratch에서 복원·검증한 뒤 packed lemma
  bytes와 offset·품사 index로 보존하고, query atom의 표제어를 조회할 때 일치하는 품사
  후보만 `Analysis`로 만든다. 진단 API가 전체 entry를 명시적으로 요청할 때만 소유 entry
  view를 지연 생성한다. 새 suffix의 UTF-8을 검증한 뒤 이미 검증된 prefix에 붙여 전체
  표제어의 UTF-8을 보장한다. ASCII와 완성형 한글 음절만 있는 표제어는 그 구성으로 NFC를
  증명하고 그 밖의 표제어는 일반 NFC 검사를 수행한다. 엄격한 정렬 순서, entry 수와 누적
  decoded byte 상한 검증도 유지한다. Encoded full POS resource는 128 MiB를 초과할 수 없으며,
  decoder는 entry 저장 공간을 예약하기 전에 이 상한을 검사한다.
- 지연 조회에서도 기존 우선순위를 보존한다. core와 enriched 용언은 같은 표제어·coarse 품사의
  full POS 용언을 억제하고, 동일한 분석은 중복하지 않는다. user lexicon의 append는 full POS
  후보를 보존하며 `replace = true`는 해당 morphology category의 core, enriched와 full POS
  후보를 모두 대체한다.
- 명시한 coarse 품사와 일치하는 사전 분석이 없으면 해당 coarse 품사가 지원하는 세부 품사를
  모두 fallback 분석으로 만든다. `noun`은 보통명사·고유명사·의존명사를 보존하며 하나의
  보통명사 분석으로 축소하지 않는다. 같은 anchor·verifier를 만드는 분석은 branch를 합치되
  세부 품사 provenance는 모두 유지한다.
- 명시한 coarse `noun`에 full POS 사전 분석이 있으면 그 분석과 누락된 보통명사·고유명사·
  의존명사 fallback을 합집합으로 보존한다. full POS의 단일 세부 품사가 명시적 coarse 품사의
  다른 component 근거를 억제하지 않으며, user lexicon의 `replace = true`는 이 합집합보다
  우선한다.
- 명시한 coarse `verb`의 주동사 분석이 있으면 같은 표제어의 보조동사 후보도 보존한다. 이
  후보는 임의의 내부 동사 substring을 허용하지 않고 compact component resource가
  `용언 + 연결 어미 + VX + 선택적 어미`의 완성 경로를 증명할 때만 매치된다. full POS의
  주동사 분석 하나가 coarse `verb`에 포함되는 보조동사 구조를 억제하지 않는다.
- core lexicon은 전체 표제어 목록이 아니라 불규칙 활용, 품사 중의성, 기능어, 표면형 override를 담는 예외 계층이다. embedded workflow의 검증된 주요 불규칙은 core에 유지한다. 자동 승격 기준을
  충족하지 못한 review 항목은 표준국어대사전과 우리말샘의 고정 snapshot이 같은 진단형을
  지지하고 충돌하는 규칙형 record가 없으며 독립 fixture가 활용과 오활용을 함께 검증한 경우에만
  수동 core 예외로 둘 수 있다. 공개 사전에서 일괄 승격한 활용 metadata는 별도 enriched 계층으로
  관리하며, core entry 수를 corpus recall에 맞춰 무제한 늘리지 않는다.
- core lexicon의 `DropH` 형용사는 검증된 ㅎ 불규칙 표제어를 명시한다. `어떻다`, `이렇다`,
  `커다랗다`는 각각 `어떤`, `이런`, `커다란` 관형형을 만들고 규칙형 `어떻은`, `이렇은`,
  `커다랗은`은 만들지 않는다.
- full POS 산출물은 전체 entry 수, 고유 표제어 수, 품사별 entry 수를 기계 판독 가능한 통계 파일로 포함한다. source를 추가하거나 갱신할 때는 이 통계와 충돌·제외 건수의 변화를 검토한다.
- 공개 사전은 고정된 전체 내려받기 snapshot만 릴리스 입력으로 사용한다. 원본 URL·버전 또는 생성 일자·SHA-256·라이선스·추출 필드·추출기 버전을 기록하며, 인증키가 필요한 live API 응답은 릴리스 빌드 입력이나 런타임 의존성으로 사용하지 않는다.
- 여러 source의 표제어·품사 후보는 합집합으로 보존하되, 같은 표제어에 core 용언 분석이 있으면 core의 활용 metadata를 우선한다. source 간 품사 충돌과 활용 분류 미확정 항목은 산출물 통계로 보고하고 임의로 한쪽을 삭제하지 않는다.
- 배포 데이터에는 원본 버전, 출처, 라이선스, 추출 명령과 체크섬을 기록한다.
- 국립국어원 사전에서 추출·정규화·선별한 enriched 용언, 현대 어미·조사와 명사 결합 접사
  catalog는 CC BY-SA 2.0 대한민국 라이선스를 적용한다. 이 데이터가 source, native
  binary 또는 WebAssembly에 포함돼 배포되는 경우 국립국어원과 한국어기초사전·
  표준국어대사전·우리말샘을 표시하고, 라이선스 링크·가공 내용·적용 파일을 함께 고지한다.
  독립적으로 작성한 source code의 MIT License와 데이터 라이선스의 적용 범위를 구분한다.
- 저장소, npm package, GitHub release source, Homebrew 설치 문서와 site의 라이선스
  페이지는 같은 국립국어원 유래 데이터 고지를 제공한다. Site footer는 코드 전용 MIT
  문서가 아니라 코드와 데이터의 통합 라이선스 페이지를 가리킨다.
- auto 품사 coverage 기준은 300개 이상의 프로젝트 gold case마다 명시된 기대 품사 분석을 포함하는 것이다. 품사별 형태 match와 no-match는 fixture 품사를 강제해 해당 분석의 허용·금지 형태를 검증하고, 품사를 생략한 제품 동작은 [0.6절](matching.md)의 사람용 fixture와 persona profile로 분리한다. 핵심 불규칙 fixture는 core lexicon만으로도 100% 통과해야 한다.
- full POS lexicon이 없으면 core lexicon으로 계속 실행하되, `--explain-query`와 명시적 사전 진단 요청에서 `preview (core lexicon only)` 상태와 자동 탐색한 모든 후보 경로를 우선순위대로 출력한다. 로드했을 때는 `loaded`와 선택된 경로를 출력한다.
- `--explain-query`는 계획 전체의 Unicode 정규화 모드와 atom별 program 수, structural
  program 수와 consumption state 수를 출력한다. consumption state 수는 해당 atom의
  program들이 참조하는 서로 다른 조사·어미 소비 구성의 수다.

## 16. 데이터 사양

### 16.1 저장소 구조

```text
data/
  lexicon/
    predicates.tsv
    nominals.tsv
    modifiers.tsv
    particles.tsv
  enriched/
    predicates.tsv
    MANIFEST.toml
    NOTICE.md
  rules/
    endings.toml
    alternations.toml
    contractions.toml
    derivations.toml
  fixtures/
    morphology_cases.tsv
  generated/
    lexicon.bin
    rules.bin
```

### 16.2 용언 사전

```tsv
lemma	pos	alternation	flags
걷다	VV	DToL
걷다	VV	Regular
듣다	VV	DToL
묻다	VV	DToL
묻다	VV	Regular
믿다	VV	Regular
돕다	VV	BToWa
눕다	VV	BToWo
짓다	VV	DropS
벗다	VV	Regular
파랗다	VA	DropH
좋다	VA	Regular
빠르다	VA	ReuDoubleL
푸르다	VA	Reo
쓰다	VV	Regular	EU_DROP
하다	VV	Ha
이다	VCP	Copula
```

### 16.3 빌드 산출물

개발용 TSV·TOML을 빌드 스크립트에서 검증하고 compact binary로 변환한다.

검증 항목:

- 중복 항목은 허용하되 완전히 같은 행은 경고
- 존재하지 않는 rule id 거부
- NFC가 아닌 표제어 거부 또는 자동 정규화 후 경고
- 용언 표제어의 기본형 형식 검증
- override 충돌 검증
- fixture에서 모든 사전 class가 최소 한 번 사용되는지 확인

enriched 용언 데이터는 core 용언 schema에 선택적 `derivations` 열을 더해 별도 파일과 라이선스로
관리한다. 각 항목은 `rule.id=target-lemma` 형식이며 core 용언 데이터에는 이 열을 요구하지 않는다.
동일한 `lemma`, `pos`, `alternation`, `flags`, `overrides`, `derivations`가 core에 있으면 core만 보존하고,
alternation이 다르면 같은 세부 품사라도 모두 보존한다. full POS의 규칙형 fallback은 core와
enriched를 합친 결과에 같은 coarse 품사가 없을 때만 추가한다.

내장 데이터는 `include_bytes!`로 실행 파일에 포함해도 된다. 이 데이터는 프로젝트가 직접 관리하고 라이선스를 명확히 할 수 있어야 한다. 사용자가 교체할 사전은 외부 파일로 추가 로딩한다.
Agent skill은 source tree의 `skills/kfind/SKILL.md`를 유일한 원본으로 삼는다. CLI fallback
문자열과 distribution asset은 이 파일에서 생성하며 내용이 서로 달라지지 않아야 한다.

### 16.4 사용자 사전

기본 위치:

```text
$XDG_CONFIG_HOME/kfind/lexicon.toml
$HOME/.config/kfind/lexicon.toml
```

예:

```toml
[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"
```

사용자 사전은 내장 사전에 추가된다. 동일 lemma의 분석을 덮어쓰려면 `replace = true`를 명시한다.

### 16.5 사전 bootstrap 전략

런타임 분석기를 쓰지 않더라도 `auto` 품사 판별에는 폭넓은 표제어 데이터가 필요하다. 다음 자료는 런타임 엔진이 아니라 릴리스 데이터 생성 단계에서만 평가한다.

- `mecab-ko-dic`: 표제어·품사 후보의 bootstrap 자료. MeCab이나 Lindera의 문장 분석 알고리즘은 사용하지 않는다. 독립 어휘 행의 headword와 품사만 추출·정규화·중복 제거하며, `Inflect`·`Preanalysis`와 문맥용 지정사 표면형은 제외한다. 상세 활용 정보는 core 사전과 gold corpus로 관리한다.
- KoParadigm: 용언·어미 분류와 활용 결과의 오프라인 참조 자료. Python 런타임 의존성으로 두지 않고, 규칙 설계와 differential fixture 생성에만 사용한다.
- 국립국어원 사전 자료: 라이선스와 재배포 조건을 충족하는 범위에서 품사와 활용 검증 자료로 사용한다. 전체 내려받기 snapshot은 릴리스 후보 데이터고, Open API는 snapshot 갱신 후보를 조사하는 개발 도구로만 사용한다.

정적 표제어 lookup과 corpus-side component 판정은 별도 resource와 품질 지표로 평가한다.
full POS 경로와 제품 compact component 경로는 Viterbi 분석, 비용 행렬과 미등록어 처리를
사용하지 않는다. 비용 기반 비교가 필요한 진단 도구만 별도 full morphology artifact를 읽는다.

### 16.6 외부 사전 데이터 정책

Homebrew 패키지는 프로젝트가 작성한 core 예외 사전과 검증된 full POS lexicon을 함께 설치한다.
full POS를 로드하지 않은 `auto` 품사 판별은 preview 상태로 표시하고 명시적 품사 태그 사용을
안내한다.

우리말샘 등 외부 데이터를 활용할 경우 다음을 분리한다.

- 원본 라이선스와 출처 표시
- 예문 등 제3자 권리 가능성이 있는 필드 제외
- 소스 코드와 사전 데이터의 라이선스 구분
- 파생 데이터가 기본 바이너리에 포함되는지 별도 검토
- API 키나 네트워크 접속을 런타임 요구사항으로 만들지 않음

대규모 외부 사전은 코드와 분리된 데이터 산출물로 만든다. full POS lexicon은 최소 배포에서
제외하고 `--pos` 중심으로 동작할 수 있지만, component 판정을 제공하는 `smart` 배포는 compact
component resource를 바이너리 밖의 필수 정적 asset으로 함께 제공해야 한다.

활용 정보가 있는 source는 표제어·품사와 분리해 다음 절차로 처리한다.

1. 공개된 활용형 중 현재 alternation을 구분하는 진단형만 추출한다.
2. 하나의 alternation으로 설명되는 항목만 enriched 후보로 만든다.
3. 여러 규칙이 가능하거나 source가 충돌하면 자동 승격하지 않고 review 목록에 남긴다.
4. core fixture 또는 독립 dev case로 확인된 후보만 활용 metadata 계층에 반영한다.

자동 분류 대상은 사전 판별이 필요한 `DToL`, `DropS`, `BToWa`, `BToWo`, `DropH`,
`ReuDoubleL`, `Reo`, `UToEo`다. 같은 종성의 규칙형과 `RegularEuDrop`은 자동 승격하지 않고
분류기의 대조군으로 기록한다. 단, 같은 `(lemma, fine_pos)`에서 독립 사전 합의가 있는 불규칙형과
규칙형 source record가 각각 확인되면 불규칙형이 full POS fallback을 가리지 않도록 규칙형도
companion 분석으로 함께 보존한다. 진단형은 런타임 `generate_predicate_branches`가 해당 lexical
rule id로 생성한 anchor를 사용하며, importer에서 별도의 한글 교체 규칙을 복제하지 않는다.

자동 승격은 한국어기초사전과 표준국어대사전의 독립된 source record가 같은 `(lemma, fine_pos,
alternation, flags)`를 지지할 때만 허용한다. 우리말샘은 추가 근거와 review 자료로만 사용한다.
서로 다른 source record가 같은 `(lemma, fine_pos)`에 규칙형과 불규칙형을 각각 지지하면 두 분석을
보존한다. 하나의 source record가 둘 이상의 분류 진단형을 동시에 포함하면 자동 집계에서 제외한다.
수동 core 예외는 이 자동 승격 조건을 바꾸지 않는다. 해당 항목은 고정 snapshot의 source record id,
선택 이유와 fixture 결과를 benchmark 보고서에 남기며, core 중복으로 바뀐 상태를 생성 report와
통계에 반영한다.

core와 완전히 같은 분석 및 `derivations.toml`의 생산 접미 규칙으로 이미 생성되는 분석은 enriched
출력에서 제외하고 report에 중복 상태로 남긴다. `UToEo`처럼 독립 사전 합의가 있어도 이미 core에
있는 유형은 신규 행을 만들지 않는다. 승격 건수와 분류별 대조군·중복·review 건수는 생성 통계에
기록한다.

importer의 원시 레코드 grain은 `(source, source_id, raw_homonym, lemma, fine_pos)`다. 동형어
식별자를 제거한 `(lemma, fine_pos)`는 집계 키로만 사용하며, 서로 다른 source record에서
확인된 복수 alternation은 충돌로 간주하지 않고 합집합으로 보존한다. redirect, 비표준어,
방언과 옛말은 자동 승격하지 않는다.

구조화된 사전 표면형은 기존 enriched predicate TSV 안의 `SurfaceOnly` 분석으로 저장한다.
별도 전체 활용형 사전이나 런타임 문장 분석기는 추가하지 않는다. `SurfaceOnly`는 같은 품사의
core·enriched 분석이나 full POS fallback을 가리지 않으며, 기본형과 사전에 기록된 정확한
표면형만 만든다. provenance-only rule id `lexical.dictionary-conjugation`,
`lexical.dictionary-adverbial-i`, `lexical.dictionary-related-adverb`는 rule registry의 생산
규칙이 아니며 이 분석에서만 허용한다.

사전 피·사동 파생 관계도 같은 enriched TSV의 `SurfaceOnly` 분석에
`lexical.dictionary-voice=target-lemma`로 저장한다. 이 행은 base의 full POS 분석을 가리지 않고
target lemma의 일반 활용기를 재사용한다. 한국어기초사전의 일반어 동사 source가 일반어 동사
target을 `파생어`로 직접 가리키고, target이 source 어간에 `이·히·리·기`와 `다`를 붙인 형태이며,
표준국어대사전에도 source와 target이 모두 일반어 동사로 있을 때만 자동 승격한다. Source record와
target record ID를 report에 보존하고 정의·예문 문자열은 사용하지 않는다. `smart`는 생성된 target
활용 표면 전체가 source의 완성된 용언+어미 구조와 일치할 때만 승인한다. 관계가 없는 동사에
voice 접미사를 추측하거나 target 표면을 base의 단순 alias로 취급하지 않는다. 구조화 필드가
피동과 사동 의미 역할을 구분하지 않으므로 정의·예문을 읽어 둘 중 하나로 임의 분류하지 않는다.

사전 활용형은 한국어기초사전과 표준국어대사전의 `일반어` record가 같은
`(lemma, fine_pos, surface)`를 지지할 때만 후보로 삼는다. core, 자동 승격된 alternation과
품사가 확인된 `하다`, `스럽다`, `답다`, `롭다`의 생산 규칙으로 이미 생성되는 surface는
저장하지 않는다. 남은 surface만 `lexical.dictionary-conjugation`으로 기록하며
`inflection`과 `derivation`에서 사용할 수 있다.

형용사 `-이` 부사형은 importer가 `-없다`, `-같다` 계열의 `어간 + 이`와 `르 → ㄹ리` 후보만
계산한다. 일반 정규 어간 전체에는 적용하지 않는다.
한국어기초사전과 표준국어대사전의 `일반어` record가 각각 원형을 형용사로, 같은 후보 표면을
부사로 독립 등재한 경우에만 `lexical.dictionary-adverbial-i`로 승격한다. 이 표면형은
`inflection`과 `derivation`에서 사용할 수 있다. 원형·결과 표제어의 source record ID를 함께
보존하며, 사전에서 확인되지 않은 후보와 자유 텍스트에서 추출한 형태는 저장하지 않는다.
`smart`는 이 표면형의 양쪽 token 경계와 전체 `MAG` 구조 근거를 요구하며 다른 어휘나 조사에
붙은 내부 span으로 확장하지 않는다.
같은 `(lemma, fine_pos, surface)`가 아래의 양방향 관계에도 포함되면 두 사전 합의를 우선해
`lexical.dictionary-adverbial-i` 하나로 기록한다.

한국어기초사전 `RelatedForm`은 source가 동사·형용사이고 target이 부사이며, 양쪽 entry가 서로의
ID를 가리키고 각 `writtenForm`이 참조한 entry의 표제어와 일치하는 `파생어` 관계만 사용한다.
이 surface는 `lexical.dictionary-related-adverb`로 기록하고 `--expand derivation`에서만 연다.
예문과 정의에서 문자열을 추출하지 않는다.

분류 생성기는 한도와 무관하게 candidate, report와 통계를 한 번 생성한다. 별도 validator가
surface-only 행 수를 보고하고 배포 `predicates.tsv`의 64 KiB 한도를 판정한다. 행 수는 UTF-8 길이가
다른 활용형·관계형의 실제 parse·배포 비용을 대표하지 않으므로 hard limit으로 쓰지 않는다.
검증에 실패하면 candidate 디렉터리를 보존해 생성 없이 validator를 다시 실행할 수 있어야 한다.
Source snapshot 갱신으로 byte 한도를 넘으면 중복 생성 규칙과 분류 누락을 먼저 해소하고, 한도 변경은
별도 성능·배포 크기 검토로 결정한다. report에는 생략된 생성형, 배포 surface-only 활용형·파생형,
source record ID와 artifact byte 수를 구분해 기록한다.

한국어기초사전 snapshot을 XML로 읽기 전에 XML 1.0에서 허용하지 않는 바이트를 검사한다.
고정 snapshot에서 사전에 기록한 값과 위치만 제거할 수 있으며, 종류·개수·위치가 달라지면
생성을 실패시킨다. manifest에는 원본 파일명·생성일·SHA-256, 정제 내역, source별 입력·후보·
충돌·제외 건수와 생성기 version을 기록한다.
검증된 ZIP의 XML은 `${XDG_CACHE_HOME:-~/.cache}/kfind/nikl/<source>/<sha256>`에 한 번만
추출하고 이후 생성에서 재사용한다. `KFIND_NIKL_CACHE`로 cache root를 바꿀 수 있으며,
SHA-256이 달라지면 별도 디렉터리에 다시 추출한다.

사전 의존도는 품사 추측을 넓혀 낮추지 않는다. `하다`, `되다`, `시키다`, `스럽다`,
`답다`, `롭다`처럼 경계가 명확한 생산 접미 규칙과 어미 continuation을 우선 보강한다.
미등록 `다` 종결어 전체를 용언으로 추측하는 fallback은 추가하지 않는다.
