# kfind 기술 사양서

문서 상태: Draft 1.6
대상 릴리스: v0.1.1
임시 제품명: `kfind`

## 0. v0.1 구현 기준

이 절은 구현에 필요한 선택지가 비어 있던 부분을 고정한다. 아래 결정은 뒤 절의 일반 설명보다 우선한다.

### 0.1 규칙 데이터와 품질 기준

- v0.1의 필수 형태 범위는 9.5절의 활용표, 19.2절의 필수 테스트, 24절의 인수 기준을 모두 포함한다.
- gold corpus에 포함된 현재 평서형 `-ㄴ다/는다`, 회상 관형형 `-던`, 과거 관형 연쇄 `-았/었을`, 과거 의문 종결 연쇄 `-았/었느냐`, `-았/었느냐는`, 인용 연결형 `-다고`, 의도 연결형 `-(으)려고`, 상태 변화 보조 용언 `-아/어지다`, 진행 방향 보조 용언 `-아/어가고`, `-아/어가야`도 v0.1의 제한된 continuation vocabulary에 포함한다.
- 실제 코퍼스에서 확인된 해요체 과거형 `-았어요/-었어요`, 지정사 `이다`의 높임 평서형 `입니다`, 부정 지정사 `아니다`의 연결형 `아니라`도 v0.1의 제한된 continuation vocabulary에 포함한다.
- 어미, 조사 연쇄, 파생 규칙의 정확한 허용 목록과 전이는 저장소의 버전 관리되는 `data/rules` 파일을 규범 데이터로 삼는다. 목록 밖 조합은 생성하지 않는다.
- full POS lexicon은 `mecab-ko-dic 2.1.1-20180720`의 Apache-2.0 데이터를 bootstrap 원본으로 사용한다. 빌드 시 표제어와 품사만 추출하고, 런타임 문장 분석 데이터와 알고리즘은 포함하지 않는다. `Inflect`와 `Preanalysis` 행은 제외하며, 문맥용 지정사 표면형은 표제어로 승격하지 않고 `VCP=이`, `VCN=아니`만 기본형으로 정규화한다.
- full POS lexicon의 용언 품사 후보도 POS 전용 산출물에 보존한다. 동일 표제어에 core 용언 분석이 하나라도 있으면 full POS 용언 분석은 추가하지 않고 core 분석을 우선한다. 그 밖의 용언은 해당 품사와 일치하는 생산적 접미 규칙을 먼저 적용하고, 일치하는 규칙이 없을 때만 제한된 규칙형 분석을 사용한다.
- full POS runtime resource는 검증된 정렬 lookup index로 보존한다. CLI, Rust library와 WASM binding은 초기화할 때 전체 entry를 일반 분석 map으로 전개하지 않으며, query atom의 표제어를 조회할 때 일치하는 품사 후보만 `Analysis`로 만든다.
- 지연 조회에서도 기존 우선순위를 보존한다. core 용언은 같은 표제어의 full POS 용언을 억제하고, core의 같은 세부 품사는 중복하지 않는다. user lexicon의 append는 full POS 후보를 보존하며 `replace = true`는 해당 morphology category의 core와 full POS 후보를 모두 대체한다.
- core lexicon은 전체 표제어 목록이 아니라 불규칙 활용, 품사 중의성, 기능어, 표면형 override를 담는 예외 계층이다. 일반 표제어 coverage는 full POS resource가 담당하고, core entry 수를 corpus recall에 맞춰 무제한 늘리지 않는다.
- full POS 산출물은 전체 entry 수, 고유 표제어 수, 품사별 entry 수를 기계 판독 가능한 통계 파일로 포함한다. source를 추가하거나 갱신할 때는 이 통계와 충돌·제외 건수의 변화를 검토한다.
- 공개 사전은 고정된 전체 내려받기 snapshot만 릴리스 입력으로 사용한다. 원본 URL·버전 또는 생성 일자·SHA-256·라이선스·추출 필드·추출기 버전을 기록하며, 인증키가 필요한 live API 응답은 릴리스 빌드 입력이나 런타임 의존성으로 사용하지 않는다.
- 여러 source의 표제어·품사 후보는 합집합으로 보존하되, 같은 표제어에 core 용언 분석이 있으면 core의 활용 metadata를 우선한다. source 간 품사 충돌과 활용 분류 미확정 항목은 산출물 통계로 보고하고 임의로 한쪽을 삭제하지 않는다.
- 배포 데이터에는 원본 버전, 출처, 라이선스, 추출 명령과 체크섬을 기록한다.
- auto 품사 품질 기준은 300개 이상의 프로젝트 gold case마다 명시된 기대 품사 분석을 포함하고, match case를 auto 계획으로 찾는 것이다. no-match case는 동음이의어 합집합이 다른 품사 경로로 찾을 수 있으므로 fixture 품사를 강제해 해당 분석의 금지 형태를 검증한다. 핵심 불규칙 fixture는 core lexicon만으로도 100% 통과해야 한다.
- full POS lexicon이 없으면 core lexicon으로 계속 실행하되, `--explain-query`와 명시적 사전 진단 요청에서 `preview (core lexicon only)` 상태와 자동 탐색한 모든 후보 경로를 우선순위대로 출력한다. 로드했을 때는 `loaded`와 선택된 경로를 출력한다.
- `--explain-query`는 계획 전체의 Unicode 정규화 모드와 atom별 verifier state 수를 출력한다. verifier state 수는 해당 atom의 branch들이 참조하는 서로 다른 verifier 구성의 수다.

### 0.2 토큰 경계와 phrase 거리

- 토큰 문자는 Unicode 문자·숫자·결합 문자와 `_`다. 한글 완성형과 자모도 토큰 문자에 포함한다.
- `smart`는 품사 verifier가 허용된 조사·어미를 소비한 token span의 바깥 경계를 검사한다. 체언, literal, 한 음절 atom은 core 시작도 토큰 경계여야 한다. 단, 조사를 직접 검색할 때는 붙은 조사를 찾을 수 있도록 core 왼쪽 경계 대신 바로 앞 host와 조사 이형태 조건을 검증한다.
- 일반 용언의 `smart` token span은 core에서 시작한다. 따라서 `가다` 검색은 `친구가`의 붙은 조사 `가`를 활용형으로 인정하지 않는다. 지정사처럼 앞 host에 붙는 분석만 별도 왼쪽 환경 검증을 사용한다.
- `token`은 모든 품사에서 core 시작과 완성된 token span 끝의 토큰 경계를 검사한다.
- `any`는 좌우 경계를 검사하지 않는다.
- phrase의 `max-gap`은 앞 atom의 `token.end`와 다음 atom의 `token.start` 사이에 있는 Unicode scalar 수다. 음수이거나 순서가 뒤집힌 span은 결합하지 않는다.

### 0.3 CLI 세부 정책

- 전역 `--pos`와 atom 태그를 함께 사용하면 같은 품사일 때만 허용하고, 다르면 컴파일 오류를 낸다.
- `--literal`은 `--expand literal --pos literal`의 단축 옵션이며 상충하는 `--expand` 또는 `--pos`와 함께 사용할 수 없다.
- `--column`은 v0.1 정식 옵션이며 1부터 시작하는 Unicode scalar 열을 출력한다.
- `--count`는 파일별로 검증된 span이 하나 이상 있는 줄의 수를 출력한다.
- EUC-KR은 명시적 `--encoding euc-kr`에서 지원한다. `auto`는 BOM 기반 UTF-16과 UTF-8만 판별한다.

### 0.4 Homebrew 대상

- tap은 `SeokminHong/homebrew-brew`, formula는 `Formula/kfind.rb`를 사용한다.
- 사용자 설치 명령은 `brew install seokminhong/brew/kfind`다.
- formula 변경은 tap `main`에 직접 push하지 않는다. 브랜치 PR의 CI가 모두 통과한 뒤 `pr-pull`을 적용한다.
- `vX.Y.Z` tag workflow는 고정 checksum으로 full POS lexicon을 재생성하고 source, full POS, man/completion 산출물을 GitHub release에 올린 뒤 `TAP_GITHUB_TOKEN`으로 tap formula PR을 연다. `pr-pull` label은 CI 확인 뒤 사람이 적용한다.
- full POS resource에는 `lexicon.bin`, 생성 manifest, `mecab-ko-dic`의 `COPYING`을 함께 넣는다. formula는 이를 `share/kfind`와 `share/doc/kfind/LICENSES`에 설치한다.
- kfind 소스 코드와 프로젝트가 직접 작성한 내장 데이터는 MIT 라이선스로 배포한다. 외부 full POS resource의 Apache-2.0 고지는 별도 `LICENSES` 디렉터리에 보존한다.

### 0.5 재현 가능한 성능 기준

- 인수 기준 9의 기준 corpus는 정확히 1 GiB(1,073,741,824 bytes), 한글 line 선택 비율 20%, 한글 line 중 NFD 선택 비율 50%, 고정 seed `0x004b46494e44`를 사용한다.
- 파일 구성은 1,000개의 64 KiB 작은 파일과 남은 bytes를 균등 분배한 24개의 큰 파일로 고정한다. 생성물은 `target/` 아래에 두고 보고서 생성 뒤 기본적으로 삭제한다.
- 낮은 hit 비율 비교는 생성 문장에 없는 고정 literal을 `kfind --literal --quiet --no-ignore`와 `rg -F --quiet --no-ignore`로 각각 전체 scan한다. 두 명령의 종료 코드 1은 정상적인 no-match 결과다.
- 전역 품사가 literal로 확정된 `--literal`과 `--pos literal` 쿼리는 full POS lexicon을 읽거나 디코딩하지 않는다. `--explain-query`는 `not required (literal query)` 상태를 출력하고 full POS lexicon 누락 진단을 내지 않는다.
- full POS startup 측정은 같은 고정 artifact로 native CLI의 빈 입력 auto query와 Node WASM의 `Kfind.withFullPos`를 각각 실행한다. warm process 3회 이상의 초기화 시간과 peak RSS 또는 process RSS 증가량을 기록하며 literal scan benchmark와 분리한다.
- 보고서는 corpus 설정과 checksum, 저장소에서 commit object로 해석되는 Git revision, CPU, memory, storage, OS, 도구 버전, 실제 명령, 각 run의 wall time·throughput·maximum RSS, median 비교값을 기록한다.
- 기본 측정은 한 번의 warm-up 뒤 warm-cache 3회를 수행한다. timer 정밀도를 확보하기 위해 각 run은 동일 scan 10회의 합산 시간을 측정해 1회당 평균을 기록한다. 권한이 필요한 cache purge를 자동 실행하지 않으며 cold-cache 결과를 측정하지 않았으면 보고서에 명시한다.

### 0.6 v0.1.1 릴리스 범위

- v0.1.1은 v0.1.0의 CLI와 형태 검색 계약을 유지하는 안정화 릴리스다.
- 형태 규칙, 품사 사전, 검색·출력 경계의 리뷰 수정과 독립 reference verifier를 포함한다.
- 1 GiB low-hit benchmark는 20절의 wall time, 처리량, RSS 목표를 모두 통과해야 한다.

### 0.7 선택적 국소 형태 추론 준비

- 문자열의 좌우 경계를 판정하는 `boundary`와 가능한 형태 분석을 선택하는
  `disambiguation`은 별도 정책 축이다.
- v0.1.1은 모든 생성 가능한 분석을 인정하는 기존 homonym union을 유지한다. 새 CLI 옵션과
  corpus-side 형태 분석에 따른 결과 필터링은 추가하지 않는다.
- query branch는 향후 어절-local 분석이 필요한지 `None` 또는 `EojeolLattice`로 표시할 수
  있다. `smart`에서 앞 host에 붙는 VCP 지정사 branch는 `EojeolLattice` 대상이며, 이 표시는
  v0.1.1의 match 결과를 바꾸지 않는다.
- `학생일`, `책일`은 사전 표제어가 아니라 각각 체언 host `학생`, `책`과 VCP 관형형 표면
  `일`의 결합을 검증하는 어절 fixture다.
- benchmark shadow 진단은 raw anchor hit, 기존 verifier를 통과한 branch hit,
  `EojeolLattice` 대상 hit, 서로 다른 분석 어절 범위를 성능 측정 구간 밖에서 기록한다.
- shadow 단계의 분석 어절 범위는 대상 hit에서 좌우 Unicode token character가 이어지는 최대
  범위다. 같은 범위를 가리키는 여러 branch hit는 한 번만 센다.
- local 분석으로 결과를 필터링하기 전에는 스펙을 먼저 갱신해 정책 이름, 기본값, resource
  누락·손상·상한 초과 동작과 JSON/explain 출력을 확정한다.
- query-side `full POS`와 corpus-side `morphology index`는 같은 고정 source snapshot에서
  생성하되 별도 산출물로 유지한다. `full POS`는 정규화된 표제어와 품사를 저장하고,
  `morphology index`는 원본 표면형별 품사·좌/우 연결 ID·단어 비용을 손실 없이 보존한다.
- `morphology index`는 표면형 prefix index와 분석 payload table을 분리한다. 같은 표면형의
  복수 분석은 하나의 key가 가리키는 payload group에 모두 보존한다.
- `morphology index` container는 schema version, source archive SHA-256, entry·고유 표면형·
  품사별 통계, 각 section의 길이·SHA-256을 포함한다. loader는 내용을 노출하기 전에 이 값을
  모두 검증하고 손상, schema 불일치, source digest 불일치를 구분해 보고한다.
- P1은 packed Double-Array trie와 FST를 동일한 key·payload로 비교하는 개발용 측정이다.
  자료구조 선택과 container 추가만으로 query 분석이나 검색 결과를 변경하지 않는다.
- resident 측정은 container 전체를 읽어 검증한 뒤 조회하고, mmap 측정은 읽기 전용으로
  고정된 artifact를 mapping해 동일한 검증을 수행한다. cold와 warm 실행은 별도 프로세스로
  측정하며 exact lookup, common-prefix 열거, 초기화 시간과 peak RSS를 함께 기록한다.
- P1의 729,173개 표면형 측정 결과에 따라 full morphology index는 packed Double-Array trie를
  사용한다. FST보다 큰 artifact를 허용하는 대신 exact lookup과 common-prefix 열거 지연을
  줄이며, 읽기 전용 full resource는 mmap으로 공유한다. source 확장 뒤 peak RSS가 40 MiB를
  넘거나 index 크기가 배포 병목이 되면 동일 benchmark로 FST 선택을 다시 검토한다.
- P2의 첫 구현 단위는 benchmark 자료구조를 `kfind-data`의 재현 가능한 resource 생성·검증
  경로로 옮긴다. resource는 index와 분석 payload에 더해 같은 source의 `matrix.def`,
  `char.def`, `unk.def`를 보존한다. 생성기는 CSV의 모든 context ID, 완전한 연결 비용 행렬과
  미등록어 정의를 검증하고, 생성 결과를 다시 decode해 검증한 뒤 artifact와 manifest를 쓴다.
  이 단위에서는 CLI와 matcher가 resource를 로드하지 않으며 query 분석과 검색 결과를 바꾸지
  않는다.
- P2의 두 번째 구현 단위는 검증된 non-empty target span을 포함하는 Unicode token 범위를
  분석 어절로 추출한다. 원문 범위는 최대 256 bytes, NFC 문자열은 최대 64 Unicode scalar로
  제한하며 잘못된 범위, UTF-8 오류와 각 상한 초과를 구분한다.
- 분석 어절은 NFC의 안정된 경계마다 원문 상대 byte offset을 보존한다. 원문 절대 byte
  span과 NFC byte span은 양방향으로 변환할 수 있어야 하며, 합성·결합 중간처럼 안정되지 않은
  경계와 범위 밖 입력은 변환하지 않는다. 이 단위에서는 resource 조회, lattice 판정과 검색
  결과 변경을 추가하지 않는다.
- P2의 세 번째 구현 단위는 NFC 분석 어절에 morphology resource의 사전 node와 HANGUL
  미등록어 node를 구성한다. lattice node는 중복 제거 뒤 최대 4,096개이며, 비용은 source의
  단어 비용과 BOS/EOS를 포함한 연결 비용만 합산한다. fixture 전용 가중치와 corpus 단어
  목록은 사용하지 않는다.
- query 포함 경로는 query 품사와 같은 사전 node가 NFC query span을 덮는 완전 경로다. 포함·
  미포함 최저 비용을 각각 계산하고 낮은 쪽을 `accept` 또는 `reject`, 동률을 `ambiguous`로
  기록한다. 한쪽 경로만 있으면 그 경로의 판정을 사용하고, 완전 경로가 없으면 명시적 오류다.
- shadow evidence는 cost margin과 최대 4개의 최저 비용 완전 경로를 보존한다. 포함·미포함
  경로가 모두 있으면 각 최저 경로를 반드시 포함하고 남은 자리를 전체 비용 순으로 채운다.
  window·node 상한 초과, resource 누락·손상·source 불일치와 평가 오류를 구분하며 threshold와
  검색 결과 필터링은 적용하지 않는다.
- P2의 네 번째 구현 단위는 corpus-side morphology resource를 schema 3으로 갱신한다.
  query tag용 `DataFinePos`는 corpus CSV 행의 필터로 사용하지 않는다. 유효한 context ID와
  비용을 가진 모든 source 행을 NFC 표면형별로 보존하고, 단일·복합 POS 열과 type·start POS·
  end POS·expression을 함께 저장한다. NFC 표면형과 이 분석 metadata가 모두 같은 행만
  중복 제거한다.
- schema 3 lattice는 `char.def`의 모든 문자 class와 각 class에 대응하는 `unk.def` 분석을
  사용한다. class의 invoke·group·length 설정을 따르되 분석 어절과 node 상한을 넘지 않는다.
  source 정의가 없거나 잘못된 문자 class와 unknown 분석은 명시적 resource 오류다.
- 단일 사전 node는 POS가 query 품사와 같고 node span이 NFC query span을 덮으면 query를
  포함한다. 복합 node는 source POS 열의 component 중 query 품사가 있고 node span이 query
  span을 덮으면 포함한다. 축약으로 component 내부 byte 경계가 NFC의 안정 경계와 일치하지
  않아도 source가 선언한 node 전체를 해당 component의 근거로 사용하며 내부 경계를 추정하지
  않는다.
- schema 3 전환은 shadow 판정만 바꾼다. query 분석, union 검색 결과, CLI와 성능 측정 구간은
  변경하지 않으며 새 비용이나 threshold를 추가하지 않는다.
- P2 lattice 구현 전에 고정 UD 2.18 Korean-Kaist·Korean-KSL dev 원문에서 지정사 판별
  slice를 생성한다. 양성은 정렬된 gold `JP=이`, `VCP=이`, `VCN=아니` 분석을 occurrence별로
  모두 보존한다. 다른 VCP/VCN 표면형은 양성으로 바꾸지 않고 제외 사유와 수를 기록한다.
- 지정사 음성은 완전히 정렬된 dev 문장 중 스펙의 지정사 활용에서 독립적으로 고정한
  surface cue를 포함하지만 같은 표제어·품사 gold 분석이 없는 문장을 source·raw tag·
  표제어별로 전수 선택한다. 도구의 예측이나 컴파일된 anchor 목록은 fixture 선택에 사용하지
  않는다.
- 지정사 판별 fixture는 source data SHA-256, 고정 seed, fixture SHA-256, source·raw tag·
  positive/negative별 case 수를 기록한다. 이 slice는 성능 측정에서 제외하고 union 결과와
  shadow counter의 confusion matrix를 별도 보고한다.
- 지정사 판별 slice에서 accept/reject 비용이나 threshold를 조정하지 않는다. P2 shadow가
  양성 경로와 어휘 내부 음성을 구분하는 판별력을 보인 뒤 별도 blind source로 확인한다.
- P3 전 blind 평가는 19.8절의 고정 Korean-GSD fixture를 사용한다. 최초 결과를 읽기 전에는
  schema 3 판정 규칙, 비용과 threshold를 변경하지 않는다.

### 0.8 Rust 라이브러리와 WASM 대상

- `kfind` 파사드 crate는 내장 core lexicon으로 초기화하거나 호출자가 제공한 full POS
  binary를 함께 로드하고, 쿼리를 재사용 가능한 matcher로 컴파일하는 공개 Rust API를
  제공한다.
- 라이브러리 matcher는 UTF-8 byte slice에서 겹치지 않는 match와 형태 분석 provenance를
  반환한다. 파일 순회, 인코딩 판별, 출력 형식과 CLI locale 처리는 라이브러리 API에
  포함하지 않는다.
- `kfind`, `kfind-wasm`, `kfind-data`, `kfind-morph`, `kfind-query`, `kfind-matcher`는
  Rust 1.85에서 `wasm32-unknown-unknown` 대상으로 빌드되어야 한다.
- `kfind-wasm`은 `wasm-bindgen` JavaScript glue와 TypeScript declaration을 생성한다.
  npm package metadata와 게시 workflow는 별도 작업 단위다.
- JavaScript API는 내장 core lexicon을 사용하는 `Kfind` 생성자, full POS binary를 받는
  `Kfind.withFullPos`, 재사용 가능한 `Matcher`를 만드는 `compile`, UTF-16 JavaScript
  문자열을 검색하는 `findAll`을 제공한다.
- `compile`은 선택적 camelCase 객체로 `expand`, `boundary`, `pos`, `normalization`,
  `maxGap`, `literal`을 받는다. 값 집합과 충돌 규칙은 CLI compile option과 동일하며
  알 수 없는 필드, 잘못된 값과 컴파일 실패는 JavaScript `Error`로 드러낸다.
- match와 atom의 `start`, `end` offset은 JavaScript `String.prototype.slice`에 바로
  사용할 수 있는 UTF-16 code unit 기준이다. 각 atom은 core·token span과 모든
  `analysisIndex`, `rulePath` provenance를 보존한다.
- 기본 CI는 Linux와 Apple Silicon macOS에서 네이티브 테스트를 실행하고, Linux에서
  MSRV의 `kfind-wasm` build를 검사한다.

### 0.9 npm package

- npm package 이름은 unscoped `kfind`다. `wasm-pack`의 `bundler` target으로 ESM
  JavaScript glue, WASM binary와 TypeScript declaration을 생성한다.
- npm 산출물은 브라우저 bundler용 release package로 생성한다. 별도의 Node target
  산출물로 같은 공개 API를 smoke test하고 `npm pack --dry-run`으로 게시 파일과 metadata를
  검증한다.
- npm package 검증은 package version과 Cargo version의 일치, TypeScript declaration의
  공개 signature, JavaScript 오류와 UTF-16 offset 계약을 확인한다.
- 기본 CI는 npm package build, Node smoke test와 pack 검사를 실행한다.

## 1. 문서 목적

`kfind`는 입력한 한국어 표제어 또는 짧은 구를 조사 결합, 어미 결합, 불규칙 활용과 일부 생산적 파생 규칙에 따라 검색 계획으로 컴파일하고, 소스 코드와 문서 파일을 빠르게 탐색하는 CLI 도구다.

이 도구는 코퍼스 전체를 형태소 분석하지 않는다. 입력 쿼리 쪽에서만 형태 정보를 해석하고, 원문에서는 빠른 문자열 앵커 검색과 국소 검증만 수행한다.

제품 설명은 다음 문구를 기준으로 한다.

> 한국어 표제어와 활용형을 빠르게 찾는 코드·문서 검색 CLI

영문 설명:

> Fast Korean lemma and inflection search for code and documents.

사용자용 영문 Markdown 문서는 같은 디렉터리에 `.ko.md` 한국어 문서를 함께 두고,
두 문서 상단에서 서로 연결한다. 이미 한국어로 작성된 사양서와 벤치마크 보고서는
언어별 사본을 만들지 않는다.

## 2. 설계 결론

기본 아키텍처는 다음과 같다.

```text
입력 쿼리
  → 정규화
  → 품사 및 사전 항목 조회
  → 어휘 교체 규칙 적용
  → 활용·조사·어미 프로그램 생성
  → 검색 앵커 선택
  → 단일 또는 다중 문자열 matcher 구성

파일 코퍼스
  → ignore 규칙 기반 병렬 순회
  → 바이트 단위 검색
  → 앵커 hit 주변만 형태 규칙 검증
  → phrase span 결합
  → bounded streaming output
```

기본 실행 경로에는 Kiwi, Lindera, MeCab 계열 분석기를 포함하지 않는다. 런타임 모델 다운로드도 하지 않는다.

동음이의어와 동형이의어는 문맥으로 구분하지 않는다. 한 표제어에서 생성 가능한 표면형이면 모두 검색 결과로 인정한다.

예:

```text
검색어: 걷다

길을 걸어 갔다.     match
전화를 걸어 봤다.   match
```

두 번째 결과는 의미상 `걸다`지만, 문맥 판별은 이 제품의 범위가 아니다.

## 3. 이전 설계에서 변경할 사항

### 3.1 `Vec<Seed>` 중심 설계를 폐기한다

완성된 표면형 문자열을 전부 나열한 `Vec<Seed>`를 유일한 중간 표현으로 쓰지 않는다. 표면형 수가 늘어날수록 메모리와 matcher 구성 시간이 증가하고, `걸었습니다`, `걸었지만`, `걸으셨다` 같은 연쇄 어미를 모두 전개하기 어렵다.

대신 검색 앵커와 국소 검증 상태를 분리한다.

```rust
pub struct SurfaceBranch {
    pub anchor: Box<[u8]>,
    pub verifier: VerifierId,
    pub core_mapping: CoreMapping,
    pub origins: SmallVec<[Origin; 2]>,
}
```

`걸었`을 앵커로 찾은 뒤 verifier가 `습니다`, `지만`, `는데` 등을 확인한다.

어미와 조사 continuation은 쿼리마다 복제하지 않는다. 빌드 시 생성한 전역 suffix DFA 또는 trie를 공유하고, 각 branch는 시작 상태만 참조한다. Aho-Corasick에는 완성 활용형 전체가 아니라 고유 앵커만 등록한다.

### 3.2 용언 분류와 활용 생성을 분리한다

`pred_class("걷다") -> "d_irregular"` 같은 함수는 사전 조회에만 해당한다. 활용형은 특정 단어 결과를 하드코딩하지 않고 입력 어간으로 계산해야 한다.

잘못된 구조:

```rust
"d_irregular" => ["걸어", "걸었", "걸은"]
```

올바른 구조:

```text
걷 + ㄷ→ㄹ + 어 → 걸어
듣 + ㄷ→ㄹ + 어 → 들어
싣 + ㄷ→ㄹ + 어 → 실어
```

### 3.3 단일 `PredClass` 대신 합성 가능한 어휘 특성을 사용한다

한국어 활용은 한 개의 문자열 class로 모두 설명하기 어렵다. 다음과 같이 어휘적 교체와 환경 의존 규칙을 분리한다.

```rust
pub struct PredicateEntry {
    pub lemma: Box<str>,
    pub pos: PredicatePos,
    pub alternation: LexicalAlternation,
    pub flags: PredicateFlags,
    pub overrides: Box<[SurfaceOverride]>,
}

pub enum LexicalAlternation {
    Regular,
    DToL,
    DropS,
    BToWa,
    BToWo,
    DropH,
    ReuDoubleL,
    Reo,
    Ha,
    UToEo,
    Copula,
    Suppletive,
}
```

`ㄹ 탈락`, `ㅡ 탈락`, 모음 축약, 자음 어미 결합은 가능한 한 어간과 어미 환경에서 계산한다. `ㄷ`, `ㅂ`, `ㅅ`, `ㅎ`처럼 같은 철자 끝에서도 규칙형과 불규칙형이 갈리는 경우는 사전으로 판별한다.

### 3.4 한 표제어의 복수 분석을 보존한다

사전은 하나의 표제어에 여러 항목을 허용한다.

```text
묻다  VV  DToL
묻다  VV  Regular
```

검색 결과는 두 분석의 합집합이다.

```text
물어, 물었다, 물으면
묻어, 묻었다, 묻으면
묻고, 묻는, 묻지
```

같은 표면형이 여러 규칙에서 생성되면 결과 span은 한 번만 출력하되, `--explain-match`와 JSON에는 모든 생성 근거를 보존한다.

### 3.5 `다` 종결만으로 용언을 판별하지 않는다

`바다`, `마다`, `솟대` 등과 같은 입력을 고려하면 `ends_with('다')`는 품사 판별 규칙으로 사용할 수 없다.

`auto` 해석 우선순위는 다음과 같다.

1. 내장 사전의 정확한 표제어 조회
2. 사용자 사전 조회
3. 생산성이 높은 접미 패턴 조회: `하다`, `되다`, `시키다`, `스럽다`, `답다`, `롭다`
4. 알려진 조사·수식언 조회
5. 미등록 한글 입력은 체언 후보와 literal 후보
6. 사용자가 `--pos` 또는 쿼리 태그를 지정하면 그 해석만 사용

미등록 `다` 종결어를 자동으로 용언 처리하지 않는다. 사용자가 `v:커스텀하다` 또는 `--pos verb`로 지정할 수 있다.

자동 품사 판별의 범위는 알고리즘보다 사전 데이터의 범위에 좌우된다. 이를 휴리스틱으로 숨기지 않는다. 배포 데이터는 두 계층으로 나눈다.

```text
core lexicon: 불규칙 용언, 고빈도 중의어, 조사와 수식언
full POS lexicon: 폭넓은 표제어와 품사, Homebrew 기본 설치에 포함
```

full POS lexicon을 찾지 못한 경우에도 검색은 가능하지만, 미등록 `다` 종결어는 literal로만 처리하고 `--explain-query`에 진단을 남긴다. 제품 릴리스에서는 full POS lexicon의 출처, 라이선스, 품사 정확도 검증을 완료해야 한다.

### 3.6 `strict`, `normal`, `loose`를 분리된 축으로 대체한다

기존 모드는 서로 다른 의미를 한 옵션에 섞었다. 다음 세 축으로 분리한다.

```text
--expand literal|inflection|derivation
--boundary smart|token|any
--pos auto|noun|pronoun|numeral|verb|adjective|determiner|adverb|particle|interjection|literal
```

기본값:

```text
--expand inflection
--boundary smart
--pos auto
```

경계 정책은 다음과 같이 정의한다.

```text
smart: 품사별 verifier가 조사·어미를 소비한 뒤 바깥 토큰 경계를 검사
token: 입력 core 자체가 독립 토큰에서 시작하도록 더 엄격하게 검사
any: 왼쪽과 오른쪽 경계를 검사하지 않는 부분 문자열 검색
```

`smart`는 임의의 한글 연속 문자열을 형태 변화로 보지 않는다. 예를 들어 `권한` 검색이 `사용자권한` 안쪽까지 들어가야 한다면 `--boundary any`를 사용한다. 한 음절 쿼리는 `smart`에서도 `token`에 가까운 경계를 적용한다.

`derivation`은 `inflection`을 포함하며 `-적`, `-하다`, `-되다`, `-시키다` 같은 생산적 파생을 추가한다.

## 4. 사용자 사용법

### 4.1 기본 검색

```bash
kfind 걷다 .
kfind 사용자 src docs
kfind 예쁘다 README.md
```

활용 확장을 끄는 단축 옵션도 제공한다.

```bash
kfind --literal 걸어 .
```

`걷다`는 표제어 검색으로 해석하고 활용형을 확장한다.

### 4.2 품사 강제

```bash
kfind --pos verb 걷다 .
kfind --pos noun 새 .
kfind --pos determiner 새 .
kfind --pos literal 걸어 .
```

짧은 태그 문법도 지원한다.

```bash
kfind 'v:걷다' .
kfind 'n:권한 v:검증하다' src
kfind 'det:새 n:기능' docs
kfind 'lit:걸어' .
```

지원 태그:

```text
n:    noun
pro:  pronoun
num:  numeral
v:    verb
adj:  adjective
det:  determiner
adv:  adverb
j:    particle
intj: interjection
lit:  literal
```

### 4.3 구 검색

```bash
kfind 'n:권한 v:검증하다' src --max-gap 24
```

다음과 같은 문장을 찾는다.

```text
권한을 검증했다.
권한 검증하는 코드를 확인한다.
권한을 먼저 확인한 뒤 검증한다.
```

원자 순서는 유지한다. 기본적으로 줄을 넘지 않으며, atom 사이 최대 거리는 Unicode scalar 기준으로 계산한다. `v:검증하다`는 `검증을 수행했다` 같은 의미적 바꿔쓰기를 검색하지 않는다. 그 경우 `n:검증`을 별도 atom으로 지정해야 한다.

### 4.4 검색 범위 제어

```bash
kfind 걷다 src --glob '*.rs' --glob '*.md'
kfind 걷다 . --hidden
kfind 걷다 . --no-ignore
kfind 걷다 . --type-add 'docs:*.{md,mdx,txt}' --type docs
```

### 4.5 해석 확인

```bash
kfind 걷다 --explain-query
kfind 걷다 src --explain-match
kfind 걷다 src --json
```

## 5. 범위와 비범위

### 5.1 v0.1.0 지원 범위

지원하는 큰 품사 범주는 다음과 같다.

| 범주 | 세부 범주 | 기본 동작 |
|---|---|---|
| 체언 | 명사, 대명사, 수사, 의존명사 | 기본형, 복수 표지, 조사 연쇄 검증 |
| 용언 | 동사, 형용사, 지정사, 일부 보조용언 | 어간 교체, 어미 결합, 축약 검증 |
| 수식언 | 관형사, 부사 | literal과 품사별 경계, 선택적 보조사 |
| 관계언 | 조사 | 이형태 묶음과 경계 검증 |
| 독립언 | 감탄사 | literal과 토큰 경계 |
| 기타 | 코드 식별자, 외국어, 숫자 | literal |

### 5.2 비범위

v0.1.0에서는 다음을 제공하지 않는다.

- 문서 전체 형태소 분석
- 문맥 기반 동음이의어 구분
- 임의 활용형의 표제어 역분석
- 동의어, 유의어, 의미 검색
- 사용자 정규식
- 치환 기능
- 방언, 고어, 비표준 활용의 포괄적 지원
- 문장 단위 구문 분석
- 무제한 어미 연쇄 생성

입력 `걸어`는 기본적으로 literal 또는 미등록 체언 후보로 처리한다. `걸어`에서 `걷다`와 `걸다`를 역추론하는 기능은 별도 후속 범위다.

## 6. 쿼리 언어와 파싱

### 6.1 토큰화

단순 `split_whitespace()`를 사용하지 않는다. 다음을 지원하는 작은 lexer를 둔다.

```text
공백 구분
작은따옴표와 큰따옴표
백슬래시 이스케이프
품사 태그 접두사
literal 강제
```

예:

```bash
kfind 'n:권한 "접근 제어" v:검증하다' src
```

`"접근 제어"`는 하나의 literal atom으로 처리한다.

### 6.2 AST

```rust
pub struct QueryAst {
    pub atoms: Vec<QueryAtom>,
    pub phrase: PhrasePolicy,
}

pub struct QueryAtom {
    pub raw: Box<str>,
    pub forced_pos: Option<CoarsePos>,
    pub quoted_literal: bool,
}
```

### 6.3 분석 결과

```rust
pub struct Analysis {
    pub lemma: Box<str>,
    pub coarse_pos: CoarsePos,
    pub fine_pos: FinePos,
    pub morphology: Morphology,
    pub source: AnalysisSource,
}

pub enum AnalysisSource {
    BuiltinLexicon,
    UserLexicon,
    ProductiveSuffix,
    Heuristic,
    Forced,
}
```

`auto` 모드에서 복수 분석이 가능하다. 예를 들어 `새`는 관형사와 명사 분석을 함께 가질 수 있다.

### 6.4 query analyzer 인터페이스

품사 판별과 검색 계획 생성을 결합하지 않는다.

```rust
pub trait QueryAnalyzer: Send + Sync {
    fn analyze(&self, atom: &QueryAtom) -> Result<Vec<Analysis>, AnalyzeError>;
}

pub struct LexiconQueryAnalyzer {
    builtin: Arc<Lexicons>,
    user: Arc<UserLexicon>,
}
```

기본 구현은 `LexiconQueryAnalyzer`다. 향후 Kiwi를 지원하더라도 쿼리 atom만 분석하는 선택형 adapter로 두고, 기본 Homebrew 패키지와 hot path에는 포함하지 않는다. 분석기 결과는 반드시 공통 `Analysis`로 변환하며, surface matcher와 파일 검색 계층은 분석기 종류를 알지 못하게 한다.

## 7. 중간 표현과 검색 계획

### 7.1 상위 구조

```rust
pub struct QueryPlan {
    pub raw_query: Box<str>,
    pub atoms: Vec<AtomPlan>,
    pub phrase_policy: PhrasePolicy,
    pub limits: PlanLimits,
}

pub struct AtomPlan {
    pub analyses: Vec<Analysis>,
    pub branches: Vec<SurfaceBranch>,
    pub boundary: BoundaryPolicy,
}

pub struct SurfaceBranch {
    pub anchor: Vec<u8>,
    pub left: LeftVerifier,
    pub right: RightVerifier,
    pub core_mapping: CoreMapping,
    pub origins: Vec<Origin>,
    pub context_requirement: ContextRequirement,
}

pub enum ContextRequirement {
    None,
    EojeolLattice,
}

pub struct Origin {
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
}
```

### 7.2 핵심 span과 토큰 span

검색 결과는 두 범위를 구분한다.

```rust
pub struct VerifiedSpan {
    pub core: Range<usize>,
    pub token: Range<usize>,
    pub origins: SmallVec<[Origin; 2]>,
}
```

예:

```text
사용자들에게
^^^^^^         core: 사용자
^^^^^^^^       token: 사용자들에게
```

기본 터미널 강조는 token span을 사용한다. JSON에는 core와 token을 모두 제공한다.

### 7.3 표면형 provenance

표면 문자열 하나만 `BTreeSet`으로 중복 제거하지 않는다. 다음과 같이 검색 키와 생성 근거를 분리한다.

```rust
HashMap<BranchKey, SmallVec<[Origin; 2]>>
```

동일 branch가 여러 분석에서 생성되면 origins를 합친다.

## 8. 한국어 음절 처리

### 8.1 내부 정규화

쿼리, 사전 표제어, 규칙 파일은 NFC로 정규화한다.

코퍼스 전체를 매번 복사해 정규화하지 않는다. 기본값은 NFC 바이트 검색이다.

```text
--unicode-normalization nfc        기본값
--unicode-normalization canonical NFC와 NFD 패턴을 모두 생성
--unicode-normalization none       입력 바이트를 그대로 사용
```

`canonical`은 완전한 임의 혼합 정규화 비교가 아니라, 쿼리 branch의 NFC·NFD 두 형태를 검색하는 모드다. Exact branch는 선택된 형태의 anchor bytes 자체가 검증 결과이므로 anchor 뒤 입력을 NFC로 변환하지 않는다. 형태 continuation을 소비하는 branch만 bounded suffix를 NFC로 변환하고 원문 byte offset으로 다시 매핑한다.

### 8.2 필요한 음절 연산

```rust
pub struct Syllable {
    pub choseong: u8,
    pub jungseong: u8,
    pub jongseong: u8,
}

pub fn decompose_syllable(c: char) -> Option<Syllable>;
pub fn compose_syllable(s: Syllable) -> Option<char>;
pub fn replace_final(c: char, jong: u8) -> Option<char>;
pub fn drop_final(c: char) -> Option<char>;
pub fn replace_last_final(s: &str, jong: u8) -> Option<String>;
pub fn drop_last_final(s: &str) -> Option<String>;
pub fn add_final(s: &str, jong: u8) -> Option<String>;
pub fn replace_last_vowel(s: &str, jung: u8) -> Option<String>;
pub fn has_final(c: char) -> bool;
pub fn has_rieul_final(c: char) -> bool;
```

한글 완성형 음절은 Unicode 산술 분해와 조합으로 처리한다. 별도의 대형 테이블은 필요하지 않다.

## 9. 형태 규칙 엔진

### 9.1 세 계층

형태 규칙은 다음 세 계층으로 분리한다.

1. 어휘적 교체: 표제어별 예외 사전
2. 어미 이형태 선택: 받침, ㄹ 받침, 모음 시작 여부 등
3. 표면 조합과 축약: `보아 → 봐`, `되어 → 돼`, `하여 → 해`

이 구분을 유지해야 규칙형과 불규칙형을 같은 generator에서 안정적으로 다룰 수 있다.

### 9.2 어미 모델

```rust
pub struct EndingSpec {
    pub id: EndingId,
    pub category: EndingCategory,
    pub initial: EndingInitial,
    pub surface: Box<str>,
    pub required: MorphFeatureMask,
    pub forbidden: MorphFeatureMask,
    pub continuation: ContinuationState,
    pub terminal: bool,
}

pub enum EndingInitial {
    Consonant,
    AOrEo,
    Eu,
    AttachNieun,
    AttachRieul,
    AttachBieup,
    Other,
}
```

예시 범주:

```text
-고, -지, -게, -다
-는, -(으)ㄴ, -(으)ㄹ
-아/-어, -아서/-어서
-았/-었, -았을/-었을, -았/었느냐(는), -겠, -시. 존대 선어말어미는 받침 어간뿐 아니라 모음·ㄹ·불규칙 어간의 올바른 교체형에도 결합한다.
-아요/-어요와 -았어요/-었어요
-면/으면, -며/으며, -려고/으려고
-아/어가고, -아/어가야
-ㅂ니다/습니다
-기, -음/ㅁ
```

선어말어미와 종결·연결어미는 작은 유한 상태 그래프로 표현한다. verifier는 `next`, `required`, `forbidden`을 모두 만족하는 경로만 소비하고, 허용 깊이를 제한해 무제한 조합을 방지한다.

어미 결합 가능성은 용언별 문자열 분기로 작성하지 않고 feature bitset으로 판정한다. 최소 feature는 다음을 포함한다.

```text
action verb
descriptive verb
copula
vowel-final
consonant-final
rieul-final
light-vowel
dark-vowel
special-ha, special-i, special-ani, special-o, special-itda
```

이 구조를 사용하면 어미 목록이 늘어나도 `match lemma` 코드가 증가하지 않고, 규칙 데이터와 테스트 fixture만 확장할 수 있다.

### 9.3 공통 규칙

다음은 사전 class가 아니라 환경 규칙으로 처리한다.

- 받침 유무에 따른 `은/는`, `이/가`, `을/를`, `과/와`
- `로/으로`의 ㄹ 받침 예외
- 조사 연쇄는 `data/rules/particles.toml`의 `next` 전이만 허용한다.
- `-(으)면`, `-(으)며`, `-(으)ㄴ`, `-(으)ㄹ`
- 의도 연결형 `-(으)려고`는 동작 용언에만 결합하고, 기존 불규칙 교체 뒤의 모음형 어간을 사용한다.
- 진행 방향 보조 용언 `-아/어가다`는 `-아/어` branch 뒤의 `가고`, `가야`만 continuation으로 소비한다. `가` 자체나 목록 밖 후속 어미는 허용하지 않는다.
- 과거 `-았/었` branch는 의문 종결형 `-느냐`와 이 종결형에 직접 붙는 주제 보조사 `는`까지 소비한다. 다른 조사나 추가 어미는 허용하지 않는다.
- `-기` 명사형은 어휘적 교체 없이 사전 어간에 직접 결합
- ㄹ 받침 뒤 특정 자음 어미에서의 ㄹ 탈락
- 어간 말음 `ㅡ`와 `-아/-어` 결합
- 모음 축약과 준말. `ㅕ` 말음 규칙 어간은 `-어`의 축약형도 보존한다 (`켜어`, `켜`).
- 자음 어미의 종성 결합

v0.1.0의 `-기` 명사형 branch는 token 경계에서 끝나며 체언 조사 verifier로 전이하지 않는다.
따라서 `걷다`는 `걷기`, `걷기 운동`을 찾지만 `걷기가`, `걷기를`은 찾지 않는다. 명사형 뒤
조사 연쇄는 predicate nominalizer에서 nominal particle verifier로 전이하는 별도 후속 범위다.

### 9.4 어휘 사전이 필요한 교체

다음은 철자만으로 안정적으로 판별하지 않는다.

- ㄷ 불규칙과 ㄷ 규칙
- ㅂ 불규칙과 ㅂ 규칙
- ㅅ 불규칙과 ㅅ 규칙
- ㅎ 불규칙과 규칙형
- 르 불규칙과 러 불규칙
- 기타 보충법과 개별 예외
- `아니다`처럼 일반적인 `-이어 → -여` 축약을 허용하지 않는 개별 어휘 제약

### 9.5 v0.1.0 필수 활용 범위

| 분류 | 예 | 기대 표면형 |
|---|---|---|
| 규칙 자음 어간 | 먹다 | 먹어, 먹었다, 먹는, 먹은, 먹을 |
| 규칙 모음 어간 | 가다 | 가, 갔다, 가는, 간, 갈 |
| ㅏ/ㅓ 축약 | 보다 | 보아, 봐, 보았다, 봤다 |
| ㅚ/ㅣ 계열 축약 | 되다 | 되어, 돼, 되었다, 됐다 |
| ㄷ 불규칙 | 걷다, 듣다, 싣다 | 걸어, 들어, 실어 |
| ㅅ 불규칙 | 짓다, 낫다, 잇다 | 지어, 나아, 이어 |
| ㅂ 불규칙 | 돕다, 눕다, 아름답다 | 도와, 누워, 아름다워 |
| ㅎ 불규칙 | 파랗다, 그렇다 | 파래, 파란, 그래, 그런 |
| 르 불규칙 | 빠르다, 부르다, 모르다 | 빨라, 불러, 몰라 |
| 러 불규칙 | 푸르다, 이르다 일부 | 푸르러 |
| ㅡ 탈락 | 쓰다, 크다, 예쁘다 | 써, 커, 예뻐 |
| 우 불규칙 | 푸다 | 퍼 |
| 하다 | 하다, 검증하다 | 하여, 해, 하였다, 했다, 검증하여, 검증해, 검증하였다, 검증했다 |
| ㄹ 탈락 | 살다, 알다, 만들다 | 사는, 압니다, 만듭니다 |
| 진행 방향 보조 용언 | 망하다, 만들다 | 망해가고, 만들어가야 |
| 과거 의문 종결 | 하다, 먹다 | 했느냐는, 먹었느냐 |
| 지정사 | 이다 | 이고, 이어, 여서, 인, 일, 입니다 |
| 부정 지정사 | 아니다 | 아니고, 아니어서, 아니라, 아닌, 아닐 |

## 10. 품사별 컴파일 규칙

### 10.1 체언

체언은 모든 완성형을 미리 생성하지 않는다.

```text
anchor: 사용자
right verifier:
  plural: 들?
  particle chain: 조사와 보조사 제한 조합
  optional VCP predicate: 이다 계열, 설정 시
```

예:

```text
사용자
사용자는
사용자들에게
사용자들로부터
```

`--expand derivation`에서는 다음을 추가할 수 있다.

```text
기술 → 기술적
검증 → 검증하다, 검증되다
단순 → 단순화
```

생산적 파생 규칙은 별도 목록으로 관리하며 기본 `inflection`에는 포함하지 않는다.

### 10.2 대명사와 수사

대명사와 수사는 체언 verifier를 공유하되, 사전에 표면 교체를 둘 수 있다.

```text
나 + 가 → 내가
너 + 가 → 네가
저 + 가 → 제가
```

v0.1.0에서 해당 축약을 지원하려면 override로 명시한다. 미지원 항목은 사양의 known limitation에 기록한다.

### 10.3 동사와 형용사

공통 predicate generator를 사용하되 종결형과 관형형 가능 범위를 품사별로 구분한다.

```rust
pub enum PredicatePos {
    Verb,
    Adjective,
    AuxiliaryVerb,
    AuxiliaryAdjective,
    Copula,
}
```

검색 도구이므로 실제 문법에서 드문 형태를 일부 허용할 수 있다. 다만 규칙으로 생성한 비표준형을 기본 결과에 포함해서는 안 된다. 확장 여부는 gold corpus로 결정한다.

### 10.4 관형사

관형사는 활용하지 않는다.

```text
left boundary: 토큰 시작
surface: literal
right condition: 토큰 경계 또는 다음 한국어 토큰 시작
```

`새`의 명사와 관형사 분석이 모두 사전에 있으면 auto 모드에서 두 분석을 합친다.

### 10.5 부사

기본은 literal과 스마트 경계다. `--expand derivation`에서만 규칙 데이터에 등록된 보조사 결합을 허용하고 격조사는 허용하지 않는다.

```text
빨리
빨리도
잘만
```

### 10.6 조사

조사를 직접 검색할 때는 이형태 묶음을 사용할 수 있다.

```text
으로 ↔ 로
은 ↔ 는
이 ↔ 가
을 ↔ 를
과 ↔ 와
```

한 음절 조사 검색은 hit가 많으므로 `smart`에서 바로 앞 host의 받침 조건과 조사 뒤 토큰 경계를 검증한다. `token`은 독립 토큰 경계를 요구하고, `--boundary any`에서만 host 검증 없는 임의 부분 문자열을 허용한다.

### 10.7 감탄사

literal과 토큰 경계만 적용한다.

## 11. 앵커 계획

### 11.1 앵커 선택 원칙

각 branch에서 가능한 가장 긴 고정 바이트열을 앵커로 선택한다.

우선순위:

1. 어간 교체 이후 첫 어미까지 포함한 문자열
2. 어간 전체
3. 짧은 어간이면 다음 고정 요소와 결합
4. 한 음절 앵커는 경계 verifier 없이는 허용하지 않음

예:

```text
걷다
  걷고
  걷는
  걷지
  걷겠
  걸어
  걸었
  걸으
  걸은
  걸을
```

이 문자열은 특정 단어 목록이 아니라 규칙 계산 결과다.

### 11.2 적응형 matcher

```text
branch 1개: Box에 보관한 memchr::memmem::Finder의 owned variant
branch 2개 이상: Aho-Corasick standard match kind의 overlapping search
```

단일 앵커 Finder는 `Finder::new(needle).into_owned()`로 구성하고 platform별 Finder 내부 크기가 `AnchorEngine` 전체 크기를 키우지 않도록 Box에 보관한다. 후보가 겹칠 수 있으므로 Aho-Corasick에서는 overlapping hit를 받고, 검증 후 가장 왼쪽의 가장 긴 token span을 선택한다.

### 11.3 branch 제한

기본 제한:

```text
쿼리 길이: 최대 256 Unicode scalar
atom 수: 최대 32
atom당 분석 수: 최대 32
전체 branch 수: 최대 4096
matcher 예상 메모리: 최대 64 MiB
어미 continuation 깊이: 최대 4
```

초과 시 조용히 잘라내지 않고 오류를 낸다. `--explain-query`에는 제한에 가까운 항목과 제외된 규칙을 표시한다.

## 12. 검색 실행 엔진

### 12.1 파일 순회

`ignore::WalkParallel`을 사용한다.

기본 정책:

- `.gitignore`, `.ignore`, 전역 ignore 반영
- hidden 파일 제외
- 바이너리 파일 제외
- symlink는 기본적으로 따라가지 않음
- 명시된 파일은 ignore 여부와 관계없이 검색

### 12.2 파일 읽기와 줄 검색

`grep-searcher`를 파일 읽기 계층으로 사용한다.

담당 범위:

- buffered search
- 줄 종결 처리
- 바이너리 감지
- mmap 사용 여부
- context 출력 지원
- 인코딩 변환 설정

형태 matcher는 `grep_matcher::Matcher`를 구현한다. 기본 터미널 출력과 요약 출력은 `grep-printer`를 우선 재사용하고, 형태 생성 근거가 필요한 JSON과 explain 출력만 확장한다.

검색 계획의 anchor가 LF를 포함하지 않으면 matcher는 LF line terminator를 선언한다. `grep-searcher`는 multi-line 기능을 켠 상태에서도 이 선언을 보고 전체 buffer에서 raw anchor가 있는 줄만 후보로 고르고, 후보 줄을 분리한 뒤 형태·경계 검증을 수행한다. LF를 포함하는 literal 계획은 line terminator를 선언하지 않아 multi-line 경로에서 검색한다.

```rust
pub struct MorphMatcher {
    pub plan: Arc<QueryPlan>,
    pub anchor_engine: AnchorEngine,
}
```

`grep_matcher::Matcher::find_at`은 다음 검증된 token span의 바이트 범위만 반환하며 origin과 rule path를 복제하거나 병합하지 않는다. 설명용 메타데이터는 매칭된 줄에 대해서만 `find_all_with_meta`로 다시 계산한다.

### 12.3 검증 단계

```text
anchor hit
  → UTF-8 경계 확인
  → 왼쪽 경계 검사
  → 어간·어미 branch verifier 실행
  → 오른쪽 경계 검사
  → core/token span 계산
  → origins 병합
```

후보 없는 buffer 구간에는 줄별 matcher 호출, Unicode scalar 순회, 형태 규칙 실행을 하지 않는다.
v0.1.1의 shadow 진단은 위 검증이 끝난 후보의 계측만 추가하며 검색 결과를 거부하지 않는다.

### 12.4 phrase 결합

각 atom의 검증된 span 목록을 구한 뒤 순서 결합한다.

```text
atom 0 spans
atom 1 spans
atom 2 spans
  → two-pointer 또는 제한된 DP
  → 순서 유지
  → max-gap 검사
```

표면형 후보들의 데카르트 곱을 정규식으로 만들지 않는다.

`find_all_with_meta`의 phrase 경로는 입력의 anchor와 atom span을 한 번 수집하고 phrase 후보를 한 번 결합한 뒤 leftmost-longest 순서로 non-overlapping 결과를 선택한다. match 하나를 반환할 때마다 남은 전체 입력의 anchor와 span 결합을 다시 계산하지 않는다.

### 12.5 병렬 출력

각 worker는 독립된 `Searcher`, scratch buffer, matcher cursor를 가진다.

```text
WalkParallel workers
  → bounded per-file record stream
  → bounded file-stream channel
  → single writer thread
  → BufWriter<StdoutLock>
```

기본 출력은 match와 context record를 검색 중에 bounded stream으로 전달한다. record callback은 파일 EOF와 검색 완료 이전에 시작할 수 있어야 하며, 출력 종료를 관찰하면 남은 입력을 더 읽지 않고 취소한다. 전체 record를 `Vec`에 모은 뒤 callback으로 재생하는 구현은 기본 경로에서 허용하지 않는다. writer는 stream이 소유한 line bytes와 match metadata를 다시 복제하지 않고 borrowed record로 직렬화한다. writer는 선택한 file stream을 끝까지 비운 뒤 다음 stream을 처리하므로 한 파일의 결과는 연속 블록으로 출력한다. 대기 중인 worker는 bounded stream에 backpressure를 받으며, 기본 경로의 결과 메모리는 corpus 또는 전체 match 수가 아니라 worker 수와 channel capacity에 의해 제한된다. 한 줄의 형태 분석 metadata는 최대 65,536개까지만 수집하고 초과하면 해당 입력을 오류로 보고하여 비정상 종료나 무제한 메모리 증가를 막는다. 기본 출력 순서는 파일 시스템 순회 순서를 보장하지 않는다.

```text
--sort path
```

정렬 옵션만 모든 file stream을 완성된 결과로 버퍼링한 뒤 path로 정렬한다. 따라서 기본값이 아니며, 결과 수에 비례해 메모리를 사용하고 병렬 성능이 낮아질 수 있음을 도움말에 명시한다.

broken pipe는 정상 종료로 처리한다.

## 13. 인코딩과 바이너리 정책

기본 인코딩은 UTF-8이다.

```text
--encoding auto
--encoding utf-8
--encoding utf-16le
--encoding utf-16be
--encoding euc-kr   선택 지원
```

`auto`는 BOM이 있는 UTF-16을 감지한다. EUC-KR 자동 추정은 하지 않는다.

잘못된 UTF-8이 섞인 파일은 바이트 검색 자체는 가능하지만, 한국어 branch 검증은 유효 UTF-8 구간에서만 수행한다.

JSON Lines 출력에서 원문 줄을 UTF-8로 표현할 수 없으면 다음 중 하나를 사용한다.

```json
{"text":null,"text_base64":"...","encoding":"bytes"}
```

## 14. CLI 사양

### 14.1 기본 구문

```text
kfind [OPTIONS] <QUERY> [PATH]...
```

PATH를 생략하면 현재 디렉터리를 검색한다. stdin이 pipe이면 기본 검색 대상을 stdin으로 전환한다. `-`는 stdin을 명시한다.

### 14.2 주요 옵션

| 옵션 | 값 | 기본값 | 설명 |
|---|---|---:|---|
| `--pos` | 품사 | `auto` | 쿼리 전체 품사 강제 |
| `--expand` | `literal`, `inflection`, `derivation` | `inflection` | 확장 수준 |
| `--boundary` | `smart`, `token`, `any` | `smart` | 경계 정책 |
| `--max-gap` | 정수 | `24` | phrase atom 사이 최대 거리 |
| `--unicode-normalization` | `nfc`, `canonical`, `none` | `nfc` | Unicode 검색 모드 |
| `--encoding` | 인코딩 | `auto` | 원문 인코딩 |
| `--glob` | glob | 없음 | 파일 포함·제외 규칙 |
| `--type`, `--type-add` | 파일 유형 | 없음 | 파일 유형 필터 |
| `--hidden` | flag | false | hidden 파일 포함 |
| `--no-ignore` | flag | false | ignore 규칙 무시 |
| `--threads` | 정수 | 자동 | worker 수 |
| `--count` | flag | false | 파일별 match 수 |
| `--files-with-matches` | flag | false | 파일명만 출력 |
| `--json` | flag | false | JSON Lines 출력 |
| `--color` | `auto`, `always`, `never` | `auto` | 터미널 색상 |
| `--explain-query` | flag | false | 쿼리 계획 출력 |
| `--explain-match` | flag | false | 생성 근거 출력 |
| `--sort` | `path` | 없음 | 결과 정렬 |
| `--data-dir` | 경로 | 자동 | 외부 데이터 디렉터리 |
| `--user-lexicon` | 경로 | 자동 | 사용자 사전 |

### 14.3 context와 출력 호환 옵션

다음은 익숙한 CLI 사용성을 위해 지원한다.

```text
-n, --line-number
-H, --with-filename
-h, --no-filename
-C, --context
-B, --before-context
-A, --after-context
-l, --files-with-matches
-c, --count
-q, --quiet
```

정규식 호환을 의미하지 않으며, 출력과 파일 검색 UX만 비슷하게 제공한다.

### 14.4 종료 코드

```text
0: 하나 이상의 match
1: match 없음
2: 사용법, I/O, 데이터, 컴파일 오류
```

### 14.5 표시 언어

사람이 읽는 도움말, 인수 파싱 오류, 런타임 오류, 검색 진단과
`--explain-query`·`--explain-match`의 설명 레이블은 영어와 한국어를 지원한다.
표시 언어는 비어 있지 않은 첫 환경 변수를 다음 순서로 선택한다.

```text
LC_ALL
LC_MESSAGES
LANG
```

선택한 locale의 언어 구성 요소가 대소문자 구분 없이 `ko`이면 한국어를 사용한다.
`ko`, `ko_KR`, `ko-KR`, `ko_KR.UTF-8`, `ko_KR.UTF-8@modifier`를 같은 언어로
처리한다. `C`, `POSIX`, 미설정 값, 지원하지 않거나 해석할 수 없는 locale은 영어로
대체한다. 우선순위가 높은 값이 비어 있지 않으면 지원하지 않는 locale이더라도 낮은
우선순위 변수로 내려가지 않는다.

옵션명, 옵션 값, 파일 경로, 규칙 ID, JSON Lines의 필드명과 값, 종료 코드는 locale과
무관하게 유지한다. 운영체제와 외부 라이브러리가 제공하는 상세 오류 문구는 kfind가
생성한 현지화된 오류 문맥 뒤에 원문으로 붙일 수 있다. man page와 shell completion은
빌드 환경의 locale에 영향받지 않도록 영어 명령 정의에서 재현 가능하게 생성한다.

## 15. 출력 사양

### 15.1 기본 출력

```text
src/walk.rs:42: 길을 걸어 갔다.
```

열 번호는 기본적으로 생략할 수 있다. `--column`에서만 match 줄의 앞부분을 Unicode scalar로 세어 계산한다.

### 15.2 쿼리 설명

```text
query: 걷다
atom[0]:
  analyses:
    - lemma: 걷다
      pos: verb
      alternation: DToL
      source: builtin-lexicon
  branches: 12
  anchors:
    - 걷고
    - 걷는
    - 걷지
    - 걸어
    - 걸었
    - 걸으
  verifier_states: 8
  normalization: nfc
  estimated_matcher_bytes: 4288
```

### 15.3 match 설명

```text
sample.txt:3: 길을 걸었습니다.
  token: 걸었습니다
  core: 걸
  generated_from: 걷다
  rules:
    - lexical.d-to-l
    - ending.past
    - ending.polite-declarative
```

### 15.4 JSON Lines

```json
{"type":"match","path":"sample.txt","line":3,"text":"길을 걸었습니다.","spans":[{"core":{"start":7,"end":10},"token":{"start":7,"end":22},"surface":"걸었습니다","origins":[{"lemma":"걷다","pos":"verb","rules":["lexical.d-to-l","ending.past","ending.polite-declarative"]}]}]}
```

유효한 UTF-8 text의 offset은 `utf8-bytes`, raw byte text의 offset은 `bytes`로 명시한다. 선택적으로 scalar column도 제공한다.

## 16. 데이터 사양

### 16.1 저장소 구조

```text
data/
  lexicon/
    predicates.tsv
    nominals.tsv
    modifiers.tsv
    particles.tsv
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

내장 데이터는 `include_bytes!`로 실행 파일에 포함해도 된다. 이 데이터는 프로젝트가 직접 관리하고 라이선스를 명확히 할 수 있어야 한다. 사용자가 교체할 사전은 외부 파일로 추가 로딩한다.

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

Lindera에서의 문장 분석 품질 문제와 정적 표제어 lookup의 품질은 별도로 평가해야 한다. 따라서 `mecab-ko-dic`을 쓰더라도 Viterbi 분석, 비용 행렬, 미등록어 처리는 제품에 포함하지 않는다.

### 16.6 외부 사전 데이터 정책

개발 초기에는 프로젝트가 직접 작성한 예외 사전과 테스트 fixture로 시작한다. 공개 v0.1.0 Homebrew 패키지는 검증된 full POS lexicon을 함께 설치하는 것을 원칙으로 한다. 해당 데이터의 출처와 재배포 조건이 확정되지 않았다면 `auto` 품사 판별은 preview 기능으로 표시하고, 명시적 품사 태그 사용을 안내한다.

우리말샘 등 외부 데이터를 활용할 경우 다음을 분리한다.

- 원본 라이선스와 출처 표시
- 예문 등 제3자 권리 가능성이 있는 필드 제외
- 소스 코드와 사전 데이터의 라이선스 구분
- 파생 데이터가 기본 바이너리에 포함되는지 별도 검토
- API 키나 네트워크 접속을 런타임 요구사항으로 만들지 않음

대규모 외부 사전은 코드와 분리된 데이터 산출물로 만들되, Homebrew 기본 formula에서는 resource로 함께 설치할 수 있다. 최소 바이너리 배포에서는 이를 제외하고 `--pos` 중심으로 동작하게 한다.

활용 정보가 있는 source는 표제어·품사와 분리해 다음 절차로 처리한다.

1. 공개된 활용형 중 현재 alternation을 구분하는 진단형만 추출한다.
2. 하나의 alternation으로 설명되는 항목만 enriched 후보로 만든다.
3. 여러 규칙이 가능하거나 source가 충돌하면 자동 승격하지 않고 review 목록에 남긴다.
4. core fixture 또는 독립 dev case로 확인된 후보만 활용 metadata 계층에 반영한다.

사전 의존도는 품사 추측을 넓혀 낮추지 않는다. `하다`, `되다`, `시키다`, `스럽다`,
`답다`, `롭다`처럼 경계가 명확한 생산 접미 규칙과 어미 continuation을 우선 보강한다.
미등록 `다` 종결어 전체를 용언으로 추측하는 fallback은 추가하지 않는다.

## 17. Rust 기술 스택

| 목적 | crate |
|---|---|
| CLI | `clap`, `clap_complete`, `clap_mangen` |
| 파일 순회 | `ignore` |
| 검색 I/O | `grep-searcher`, `grep-matcher` |
| 단일 앵커 | `memchr::memmem` |
| 다중 앵커 | `aho-corasick` |
| 바이트 문자열 | `bstr` |
| Unicode 정규화 | `unicode-normalization` |
| 인코딩 | `encoding_rs` 또는 `grep-searcher` 연동 계층 |
| 출력 | `grep-printer`, `serde`, `serde_json` |
| 오류 | `thiserror` |
| 병렬 결과 채널 | `crossbeam-channel` |
| 작은 벡터 | `smallvec` 선택 |
| 벤치마크 | `criterion` |
| 속성 테스트 | `proptest` |
| fuzz | `cargo-fuzz` |

`ignore::WalkParallel`이 파일 단위 병렬 처리를 담당하므로 별도 `rayon` 의존성은 기본 구조에 필요하지 않다.

`memmap2`를 직접 다루기보다 `grep-searcher`의 mmap 정책을 우선 사용한다.

## 18. crate 구조

```text
crates/
  kfind/
    public engine, compiled matcher, library errors

  kfind-wasm/
    wasm-bindgen API, JavaScript option parsing, match serialization

  kfind-cli/
    args, output, exit status, shell completion

  kfind-query/
    lexer, AST, POS inference, query plan

  kfind-morph/
    Hangul operations, lexicon, endings, alternations, verifier

  kfind-matcher/
    anchor planning, memmem/Aho-Corasick, grep_matcher adapter

  kfind-search/
    ignore walk, grep-searcher integration, parallel output

  kfind-data/
    data validation and binary compilation

  kfind-testkit/
    fixture loader, reference backend, corpus generator
```

## 19. 참조 구현과 검증 전략

reference backend는 production anchor 계획과 결과 타입만 공유한다. 서술어 continuation과 조사 연쇄 판정은 production verifier를 호출하지 않고 별도 순회 구현으로 계산해 동일 결함을 공유하지 않게 한다.

### 19.1 최적화 엔진과 참조 엔진을 분리한다

프로덕션 엔진은 앵커와 verifier를 사용한다.

테스트용 참조 엔진은 동일 규칙 AST를 작은 정규 언어 또는 후보 문자열 집합으로 변환해 `regex-automata`로 실행할 수 있다.

두 엔진의 결과를 작은 corpus에서 비교한다.

```text
optimized(query, corpus) == reference(query, corpus)
```

정규식은 사용자 기능이 아니라 구현 검증 도구로만 사용한다.

### 19.2 단위 테스트

필수 테스트:

```text
걷다 → 걸어, 걸었, 걸으면, 걸으셨다
듣다 → 들어, 들었, 들으면
듣다 → 걸어 아님
묻다 → 물어와 묻어 모두
예쁘다 → 예뻐, 예뻤다, 예쁜, 예쁠
예쁘다 → 예쁘어 아님
좋다 → 좋아요, 좋았어요
아니다 → 아니고, 아니라, 아닌, 아닐
부르다 → 불러
푸르다 → 푸르러
보다 → 보아와 봐
되다 → 되어와 돼
살다 → 사는, 삽니다, 살고
사용자 → 사용자들에게
길 → 길로, 길으로 아님
```

동음이의어 정책 테스트:

```text
query: 걷다
text: 전화를 걸어 봤다.
expected: match
```

### 19.3 속성 테스트

- 음절 분해 후 조합하면 원래 음절과 같음
- 유효한 종성 교체 결과는 다시 분해 가능
- branch verifier는 anchor 밖을 읽지 않음
- 동일 span의 origin 병합은 순서와 무관
- phrase join 결과는 atom 순서를 항상 보존

### 19.4 fuzz

target과 경계:

| target | 경계 |
| --- | --- |
| `query_lexer` | 잘못된 UTF-8을 포함한 임의 query, 매우 긴 combining sequence, lexer와 compile limit |
| `matcher_bytes` | 임의 byte 입력의 anchor 탐색, suffix verifier, match span 범위 |
| `user_lexicon` | malformed 사용자 사전 TOML의 구문·의미 검증 |
| `json_output` | 임의 byte line과 검증된 match metadata의 JSON Lines 직렬화 |
| `binary_detection` | 임의 위치의 최초 NUL과 NUL이 없는 입력의 binary 판별 경계 |

### 19.5 gold corpus

공식 어문 규정의 활용 예와 프로젝트가 직접 작성한 문장을 기반으로 fixture를 만든다.
실제 사용 양상은 재배포 조건이 명확한 공개 코퍼스의 짧은 문장으로 함께 검증한다.
실제 코퍼스 항목의 `feature`는 `corpus.<source>.<split>.<id>` 형식으로 원문을 식별하고,
fixture 디렉터리의 README에 원본 revision, 라이선스, 추출 경로를 기록한다.
뉴스·대화·리뷰에서 나타나는 합성 용언, 띄어쓰기 생략, 비표준 철자는 v0.1 범위와
경계 정책에 따라 기대 결과를 정하며 형태 규칙이 지원하는 것처럼 완화하지 않는다.

각 항목:

```tsv
query	pos	text	expected	feature
걷다	verb	길을 걸어 갔다.	match	d-irregular
걷다	verb	전화를 걸어 봤다.	match	homonym-union
예쁘다	adjective	예쁘어 보인다.	no-match	eu-drop
```

문맥상 다른 표제어인 결과는 현재 정책상 false positive로 계산하지 않는다. 형태 규칙이 만들 수 없는 문자열만 false positive다.

### 19.6 외부 분석기 비교

Kiwi와 Lindera 비교는 저장소의 개발 전용 검증으로 실행하며 제품 바이너리, Homebrew
의존성, 기본 검색 경로에 포함하지 않는다. 제품 fixture는 `kfind` 자체 회귀 검증에만
사용하고 외부 분석기와의 우열 점수에는 사용하지 않는다. 비교 환경은 버전을 고정한
단일 Docker 이미지로 구성하고, 이미지 빌드 이후에는 네트워크 없이 실행할 수 있어야
한다. adapter 오류와 실행 실패는 성공 결과로 대체하지 않는다.

### 19.7 독립 형태소 벤치마크

기성 분석기와의 품질 비교는 제품 fixture와 분리한 held-out corpus로 수행한다. 기본
데이터는 Universal Dependencies 2.18의 Korean-Kaist와 Korean-KSL test split이며, 원문과
라이선스 파일의 URL·SHA-256·라이선스를 manifest에 고정한다. 다운로드와 fixture 생성은
이미지 빌드 단계에서 끝내고 실제 벤치마크는 네트워크 없이 실행한다.

fixture는 도구 출력과 무관한 고정 seed로 생성한다. 각 corpus에서 명사 90, 동사 60,
형용사 40, 부사 25, 대명사 15, 관형사 10, 수사 10개를 positive로 선택한다. 같은 corpus의
다른 문장 중 동일 표제어·품사가 gold에 없는 문장을 deterministic negative로 하나씩
대응시켜 총 1,000개와 positive/negative 1:1 균형을 유지한다. 정렬과 샘플링은 원본 파일
순서가 아니라 case 식별자의 SHA-256 순서를 사용한다.

gold 후보는 CoNLL-U의 정렬된 lemma/XPOS 형태소 쌍에서 추출하고, lemma가 축약된 KAIST
어절은 `OrigLemma`를 우선 사용한다. 지원 품사에 속하고 표제어가 한글 음절로만 구성된
형태소만 포함한다. VV·VA·VX·VCP·VCN과 이에 대응하는 KAIST 용언 태그는 어간에 `다`를
붙여 사전형으로 정규화한다. 형태소 수와 XPOS 수가 끝까지 다른 어절, 접사·조사·어미,
외국어·숫자·기호는 제외한다. negative는 모든 어절의 lemma/XPOS가 정렬된 문장에서만
선택한다. 이 필터와 제외 건수는 metadata에 기록한다.

세 도구는 동일한 `(문장, 표제어, 품사)` 존재 여부를 예측한다. positive는 예측 span이
gold 어절의 UTF-8 byte span과 겹쳐야 true positive이고, negative는 문장 어디에서든 같은
표제어·품사를 반환하면 false positive다. 도구마다 accuracy, precision, recall, F1과
TP·FP·TN·FN을 계산하고 corpus별·품사별 결과 및 실패 case를 함께 보존한다.

성능 측정은 데이터 준비를 제외하고 backend별 warm-up 1회를 버린 뒤 동일한 case
순서로 최소 5회 반복한다. 각 run은 초기화를 한 번만 수행하고 해당 프로세스에서
전체 case를 처리한다. 초기화 시간, 전체 처리 시간, case/s, p50·p95 latency,
peak RSS의 median과 run 간 min/max를 보고한다. `kfind`는 질의 컴파일과 검색, Kiwi와
Lindera는 문장 분석과 표제어·품사 조회를 포함한 end-to-end 검색 경로를 측정한다.
이 수치는 서로 다른 검색 전략의 제품 작업량 비교이며 순수 형태소 tokenizer
처리량으로 표현하지 않는다.

최종 보고서는 fixture SHA-256, seed, source별 case 수, 도구와 데이터 버전, 전체·source별·
품사별 품질 지표, 성능 지표, adapter 오류를 JSON과 Markdown으로 기록한다. 같은 JSON에서
전체 품질과 성능 trade-off SVG를 재현하고 분석 문서에 포함한다. 1,000개 미만,
class/source/POS quota 불충족, source hash 불일치, adapter 오류가 있으면 실행을 실패시킨다.

`kfind` 결과는 `embedded`와 `full-pos` 프로필을 같은 fixture·case 순서로
각각 측정한다. 보고서의 버전 메타데이터에 profile과 full POS lexicon artifact
SHA-256을 기록하고, `embedded`는 artifact가 없음을 명시한다. `full-pos` 실행에서
artifact가 없거나 디코딩하지 못하면 `embedded`로 대체하지 않고 실패시킨다.
프로필별 품질·초기화·처리량·지연·peak RSS를 병렬로 보고하고, `embedded`의
false negative 중 `full-pos`에서 회복된 case와 계속 실패한 case를 별도 목록으로
저장한다.

failure 원인 분류는 성능 측정 구간 밖에서 수집한 질의 계획·anchor·경계 증거를
사용한다. 각 kfind 프로필의 false negative는 다음 우선순위로 하나의 원인을 갖는다.
호환용 `primary_cause`는 embedded 원인을 유지하고, `profile_causes`와
`profile_cause_evidence`에 embedded/full-POS 결과를 모두 기록한다.

1. Kiwi와 Lindera도 같은 gold를 놓치면 `gold-or-adapter`
2. auto 질의 계획에 기대 품사 분석이 없으면 `lexicon-missing`
3. smart 결과는 있지만 gold span과 겹치지 않으면 `span-mismatch`
4. `boundary=any`만 gold span을 찾으면 `boundary-rejected`
5. gold 어절 내부에 core anchor가 있지만 검증 span이 없으면 `continuation-rejected`
6. 그 밖은 `surface-missing`

분류 증거와 profile별 primary cause는 JSON failure record에 저장하고, 분류를 위한 추가
컴파일·검색 비용은 backend 성능에 포함하지 않는다.

규칙 개발은 Korean-Kaist·KSL dev split을 test split과 독립된 seed·fixture
SHA-256로 생성해 사용한다. test 1,000개 baseline은 변경하지 않는다. hard-negative는
도구 출력과 무관한 버전 관리 fixture로 두고 slice별 precision을 전체 품질과 분리해
보고한다. CI smoke set은 dev fixture에서 source·품사·class별 고정 case를
deterministic하게 추출하고, 수동 벤치마크는 dev·test·hard-negative 전체를 사용한다.

### 19.8 지정사 lattice blind 평가

P3 전 지정사 lattice 판별력은 기존 Korean-Kaist·KSL과 별개인 UD Korean-GSD의 고정
test split에서 한 번 확인한다.

| 항목 | 값 |
| --- | --- |
| source | UD Korean-GSD r2.18 (`02c343e4e1e3180069f637e68a791ec6b96dd33a`) test split |
| data URL | `https://raw.githubusercontent.com/UniversalDependencies/UD_Korean-GSD/r2.18/ko_gsd-ud-test.conllu` |
| data SHA-256 | `3d1df99bda4800235e14bcfd915baf706eafa1a3935a75ffd32420a51e57f5aa` |
| license | CC BY-SA 4.0 |
| license URL | `https://raw.githubusercontent.com/UniversalDependencies/UD_Korean-GSD/r2.18/LICENSE.txt` |
| license SHA-256 | `899b1804a12ebc090b96339614eede1b64b686721b650a71430b55b5235f7f79` |
| seed | `kfind-vcp-vcn-blind-v1` |
| fixture SHA-256 | `4be12e060c4bc3faf35b78bb3c9189cafb49e7c885108383c0dd1fb5aeb1b188` |

fixture는 dev 지정사 판별 slice와 같은 정규화와 case schema를 사용한다. 양성은 gold
`VCP=이`, `VCN=아니` 분석을 occurrence별로 전수 보존한다. 음성은 완전히 정렬된 문장 중
각 분석에 고정된 surface cue가 있지만 같은 표제어·품사 gold가 없는 문장을 전수 보존한다.
도구 출력, query anchor와 비용은 선택에 사용하지 않는다. quota sampling은 하지 않는다.

| raw tag | positive | negative |
| --- | ---: | ---: |
| VCP | 311 | 460 |
| VCN | 10 | 0 |
| 합계 | 321 | 460 |

case는 UTF-8 JSON Lines로 기록하고 object key를 사전순으로 직렬화한다. 각 줄은 LF로 끝난다.
순서는 `SHA-256(seed + NUL + "blind-context-order" + NUL + case_id)` byte 순이다. source
hash, 위 그룹별 case 수, 전체 781개와 fixture hash가 다르면 생성을 실패시킨다.

중복 검사는 Korean-GSD test와 manifest에 고정된 Korean-Kaist·KSL dev/test의 모든
`# text`를 NFC로 정규화한 UTF-8 SHA-256 집합으로 수행한다. 교집합은 0개여야 하며 하나라도
있으면 해당 문장을 조용히 제외하지 않고 생성을 실패시킨다.

fixture 생성 단계는 source·parsing 통계, case 수와 digest만 노출한다. backend 예측, lattice
비용과 path는 최초 blind report 전에는 출력하지 않는다. 최초 report는 변경하지 않은 schema 3
판정의 비용 분포와 판별력만 기록한다. 결과를 확인한 뒤 이 fixture는 regression baseline으로만
사용하며, 그 결과에 맞춰 비용·threshold·fixture 가중치를 바꾼 구현은 별도 unseen source에서
다시 검증해야 한다. blind report만으로 union 검색 결과나 기본 정책을 변경하지 않는다.

`tools/morph-compare/sources.json` schema 3은 전체 source 목록과 기본 품질 benchmark에 참여하는
source 이름을 분리한다. Korean-GSD를 추가해도 기존 Kaist·KSL dev/test 1,000-case fixture의
구성·digest는 바뀌지 않아야 한다. 지정사 생성기는 config 이름을 받아 dev와 blind fixture에
같은 선택·직렬화 검증을 적용한다. blind config는 비교할 기존 source·split과 예상 fixture
digest를 함께 선언하며 중복이나 digest 불일치를 생성 오류로 처리한다.

Docker corpus build는 blind fixture와 metadata를 `/opt/morph-benchmark/data`에 포함하되 기본
`benchmark.py`에는 입력하지 않는다.

최초 평가는 `KFIND_MORPH_BLIND=1 scripts/benchmark-morphology.sh
target/morph-blind-report`로만 실행한다. 전용 entrypoint는 blind metadata의 split·case 수·
fixture digest를 다시 검증하고 각 backend를 warm-up 없이 한 번 평가한다. JSON은 case별
prediction, span, lattice 비용·경로를 보존하고 Markdown은 품질과 shadow 판정을 요약한다.
기본 benchmark 명령은 blind fixture를 계속 읽지 않는다. 최초 성공 report의 결과를 문서에
기록한 뒤 fixture를 regression baseline으로 전환한다.

2026-07-13 최초 report는 7d233a6에서 생성했다. JSON SHA-256은
`fab077bc4d9b76a0d4e75977e8af0e8ffea8f702612e9c2a8e280ac56c1f076a`다. 문장 안의 모든 gold
occurrence를 합쳐 candidate를 중복 제거하면 gold target 142개 중 127개를 수용하고 non-gold
target 101개 중 97개를 거절한다. gold target 15개를 거절하므로 P3는 계속 보류한다. 이 결과로
비용·threshold·검색 결과를 변경하지 않으며 Korean-GSD fixture는 regression baseline으로만
사용한다.

## 20. 성능 사양

### 20.1 목표

기준 장비와 corpus는 벤치마크 보고서에 고정한다. 예시는 Apple Silicon의 최근 세대 장비로 두되, 결과에는 CPU, 메모리, 저장장치, OS를 반드시 기록한다.

v0.1.0 목표:

```text
단일 atom query compile p95: 15 ms 이하
8 atom phrase compile p95: 50 ms 이하
낮은 hit 비율의 scan: rg -F wall time의 1.5배 이내
낮은 hit 비율의 처리량: rg -F의 70% 이상
기본 RSS: 40 MiB 이하
corpus 크기에 비례하는 결과 버퍼링 없음
```

`rg -F`와 기능이 동일하지 않으므로 절대 우열이 아니라 I/O 경로의 성능 회귀 감시 기준으로 사용한다.

### 20.2 corpus

```text
100 MiB source corpus
1 GiB mixed corpus
한글 비율 5%, 20%, 80%
작은 파일 다수 corpus
큰 파일 소수 corpus
NFC corpus
NFD corpus
UTF-16 fixture
```

corpus 생성기는 전체 bytes, 파일 수, 작은 파일 수와 크기, 한글 line 선택 비율, 한글 line의 NFD 선택 비율, seed를 명시적으로 받는다. 같은 설정과 seed는 byte 단위로 동일한 파일 tree를 생성해야 한다. NFC/NFD와 한글 비율은 완전한 line을 선택하는 비율이며, 파일 끝의 exact-size padding은 ASCII로 채운다.

### 20.3 측정 구간

다음 시간을 분리한다.

```text
startup
lexicon load
query compile
filesystem walk
scan
verification
output
```

query compile 목표는 lexicon을 미리 로드한 같은 analyzer를 재사용하고 다음 두 입력을 각각
`query_compile/single_atom`과 `query_compile/phrase_8_atoms` Criterion benchmark로 측정한다.

```text
single_atom: 걷다
phrase_8_atoms: n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다
```

`matcher/phrase_find_all`은 1,024개 line 중 4개마다 `n:길 v:걷다`가 일치하는 고정 corpus를 메모리 입력으로 사용한다. 전체 phrase match를 반환하는 한 번의 호출을 측정해 match 수에 따른 반복 anchor scan과 span 결합 회귀를 감시한다.

p95는 Criterion `new/sample.json`의 각 sample에 대해 `times[i] / iters[i]`로 계산한
1회당 nanoseconds를 오름차순으로 정렬하고 nearest-rank 방식으로 선택한다. 정식 목표 판정은
기본 sample 설정으로 수행한다. `--quick` 결과는 benchmark가 실행되는지만 확인하는 smoke
측정이며 목표 판정에 사용하지 않는다.

`--count`, `--quiet`, 기본 출력, JSON을 별도로 측정한다. cold cache와 warm cache 결과를 구분한다.

인수 기준 9의 `rg -F` 비교 runner는 동일 corpus와 no-match literal을 대상으로 `--quiet` warm-cache scan을 측정한다. 처리량은 정확한 corpus bytes를 wall time으로 나눈 값이고, maximum RSS의 단위와 수집 도구를 보고서에 함께 쓴다.
각 scan은 새 프로세스의 startup을 포함하되, literal 쿼리에 필요하지 않은 full POS lexicon 로드는 수행하지 않는다.

### 20.4 회귀 정책

동일 CI runner에서 main 기준 다음 중 하나면 경고한다.

```text
query compile 20% 이상 악화
scan throughput 10% 이상 악화
RSS 20% 이상 증가
branch 수 2배 이상 증가
```

## 21. Homebrew 배포

### 21.1 배포 형태

초기에는 공식 core가 아니라 custom tap으로 배포한다.

```bash
brew install seokminhong/brew/kfind
```

릴리스 구성:

```text
kfind source tarball
Cargo.lock
내장 규칙과 사전 소스
생성된 man page
shell completions
checksums
```

런타임 모델 다운로드는 없다. full POS lexicon을 별도 파일로 배포하면 formula의 resource 또는 별도 release artifact로 함께 설치하고, 코드와 데이터의 라이선스를 각각 표시한다.

### 21.2 formula 설치 항목

```text
bin/kfind
share/man/man1/kfind.1
share/zsh/site-functions/_kfind
share/fish/vendor_completions.d/kfind.fish
etc/bash_completion.d/kfind
share/doc/kfind/LICENSES/
```

내장 규칙과 프로젝트 자체 사전은 실행 파일에 포함한다. 선택형 대규모 사전만 `share/kfind` 아래에 둘 수 있다.

### 21.3 bottle

최소 검증 대상:

```text
macOS arm64
macOS x86_64
```

CI에서 tagged release의 bottle을 생성한다. formula test는 임시 파일을 만들고 실제 형태 검색을 확인한다.

```ruby
test do
  (testpath/"sample.txt").write("길을 걸어 갔다.\n")
  output = shell_output("#{bin}/kfind 걷다 #{testpath}/sample.txt")
  assert_match "걸어", output
end
```

## 22. 보안과 견고성

- 모든 파일 크기와 branch 수에 상한을 둔다.
- 사용자 사전 파싱 오류에는 파일명과 줄 번호를 표시한다.
- symlink 순환을 방지한다.
- 검색 결과, 검색 중 issue, 초기화 오류를 포함한 모든 사람이 읽는 출력에 escape 정책을 적용해 제어 문자가 터미널 동작을 바꾸지 않게 한다.
- JSON에는 원문 제어 문자를 정상 escape한다.
- 파일 경로가 유효 UTF-8이 아니어도 처리한다.
- broken pipe에서 panic하지 않는다.
- matcher와 verifier는 unsafe 없이 구현하는 것을 기본 원칙으로 한다.

## 23. 구현 단계

### M0. 의미 고정과 corpus 준비

완료 조건:

```text
CLI 옵션명 확정
동음이의어 합집합 정책 문서화
full POS lexicon 출처·라이선스·추출기 확정
자동 품사 판별 품질 기준 확정
gold corpus 300개 이상
벤치마크 corpus 생성기
기준 rg -F 측정
```

### M1. literal 검색과 I/O 골격

```text
ignore::WalkParallel
custom grep_matcher
memmem/Aho-Corasick adapter
기본 출력과 종료 코드
binary, encoding, context
```

### M2. query parser와 체언

```text
태그 lexer
AST
auto POS lookup
체언 anchor와 조사 verifier
로/으로 ㄹ 예외
사용자 사전
```

### M3. 한글 연산과 규칙 활용

```text
음절 분해·조합
규칙 자음·모음 어간
자음 어미 결합
ㅡ 탈락
모음 축약
하다와 지정사
```

### M4. 어휘적 불규칙 활용

```text
ㄷ, ㅂ, ㅅ, ㅎ, 르, 러, 우
복수 분석 합집합
override
provenance
```

### M5. 수식언, 조사, phrase

```text
관형사·부사 경계
조사 직접 검색
span join
max-gap
복수 atom explain
```

### M6. 품질과 배포

```text
reference backend differential test
property test와 fuzz
criterion benchmark
man page와 completions
Homebrew tap과 bottles
```

## 24. v0.1.0 인수 기준

다음 조건을 모두 만족해야 한다.

1. 코퍼스 전체 형태소 분석기 없이 동작한다.
2. `걷다`에서 불규칙 분석의 `걸어`, `걸었다`, `걸으면`, `걸으셨다`와 규칙 분석의 `걷어`, `걷었다`를 모두 찾는다.
3. `듣다`에서 `들어`를 만들고 `걸어`를 만들지 않는다.
4. `묻다`에서 규칙형과 ㄷ 불규칙형을 모두 검색한다.
5. `예쁘다`에서 `예뻐`, `예쁜`, `예쁠`을 찾고 `예쁘어`를 만들지 않는다.
6. 체언에서 모든 조사 문자열을 미리 전개하지 않고 verifier로 처리한다.
7. phrase query를 후보 문자열 데카르트 곱 없이 span 결합으로 처리한다.
8. 동일 표면형의 모든 생성 근거를 JSON에서 보존한다.
9. 1 GiB corpus benchmark와 rg -F 비교 보고서가 있다.
10. Homebrew로 설치한 뒤 네트워크 접속 없이 실행된다.
11. macOS arm64와 x86_64에서 formula test가 통과한다.
12. 사용자 사전 없이도 핵심 불규칙 fixture가 통과한다.
13. Homebrew 기본 설치에서 full POS lexicon이 로드되고, 사전 누락 시 명확한 진단을 출력한다.
14. 공개 Rust 라이브러리가 동일한 query plan과 matcher를 사용해 메모리 입력을 검색한다.
15. 공개 라이브러리와 핵심 의존 crate가 Rust 1.85의 `wasm32-unknown-unknown` target에서
    빌드된다.
16. `kfind` npm 산출물의 Node smoke test, TypeScript declaration 검사와
    `npm pack --dry-run`이 통과한다.

## 25. 권장 초기 코드 인터페이스

```rust
pub fn compile_query(
    source: &str,
    options: &CompileOptions,
    lexicons: &Lexicons,
    rules: &RuleSet,
) -> Result<QueryPlan, CompileError>;

pub fn analyze_atom(
    atom: &QueryAtom,
    options: &CompileOptions,
    lexicons: &Lexicons,
) -> Result<Vec<Analysis>, CompileError>;

pub fn compile_analysis(
    analysis: &Analysis,
    options: &CompileOptions,
    rules: &RuleSet,
) -> Result<Vec<SurfaceBranch>, CompileError>;

pub trait LocalVerifier: Send + Sync {
    fn verify(
        &self,
        haystack: &[u8],
        anchor_span: Range<usize>,
        scratch: &mut VerifyScratch,
    ) -> Option<VerifiedSpan>;
}
```

matcher 쪽:

```rust
pub enum AnchorEngine {
    One(Box<memchr::memmem::Finder<'static>>), // into_owned()로 구성
    Many(aho_corasick::AhoCorasick),
}

pub struct MorphMatcher {
    plan: Arc<QueryPlan>,
    anchors: AnchorEngine,
}
```

사전 조회:

```rust
pub trait Lexicon {
    fn lookup(&self, surface: &str) -> &[LexiconEntry];
}
```

초기 구현은 정렬된 static slice와 binary search로 충분하다. query compile은 검색당 한 번이므로 복잡한 perfect hash 최적화는 측정 후 결정한다.

## 26. 최종 제품 원칙

`kfind`의 핵심은 “모든 문장을 분석하는 것”이 아니라 다음 세 가지다.

```text
표제어를 정확히 해석한다.
검색 가능한 형태 규칙을 유한한 계획으로 컴파일한다.
원문에서는 긴 고정 앵커를 찾고 필요한 위치만 검증한다.
```

이 원칙을 지키면 형태 품질은 사전과 규칙 fixture로 개선할 수 있고, 검색 성능은 기존의 검증된 파일 순회·바이트 검색 계층을 활용해 유지할 수 있다.

## 27. 참고 자료

- [Unicode Standard Annex #15, Unicode Normalization Forms](https://www.unicode.org/reports/tr15/)
- [Unicode 17.0 Core Specification, Chapter 3](https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-3/)
- [국립국어원 한국어 어문 규범](https://korean.go.kr/kornorms/regltn/regltnView.do)
- [Rust `aho-corasick` documentation](https://docs.rs/aho-corasick/)
- [Rust `memchr::memmem::Finder` documentation](https://docs.rs/memchr/latest/memchr/memmem/struct.Finder.html)
- [Rust `ignore` documentation](https://docs.rs/ignore/)
- [Rust `grep-searcher` documentation](https://docs.rs/grep-searcher/)
- [Rust `grep-matcher` documentation](https://docs.rs/grep-matcher/)
- [Rust `grep-printer` documentation](https://docs.rs/grep-printer/)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Bottles](https://docs.brew.sh/Bottles)
- [Homebrew Taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [우리말샘 저작권 정책](https://opendict.korean.go.kr/service/copyrightPolicy)
- [우리말샘 Open API 안내](https://opendict.korean.go.kr/service/openApiInfo)
- [KoParadigm repository](https://github.com/Kyubyong/KoParadigm)
- [KoParadigm paper](https://arxiv.org/abs/2004.13221)
- [mecab-ko-dic repository](https://bitbucket.org/eunjeon/mecab-ko-dic)
