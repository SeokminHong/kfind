# 쿼리 언어와 검색 계획

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

## 0. 제품 계약

이 절은 뒤의 일반 설명보다 우선한다.

### 0.2 토큰 경계와 phrase 거리

- 토큰 문자는 Unicode 문자·숫자·결합 문자와 `_`다. 한글 완성형과 자모도 토큰 문자에 포함한다.
  분류기는 ASCII 영숫자와 전체가 문자로 할당된 한글 완성형 음절 범위를 먼저 판정할 수 있지만,
  나머지는 Unicode 영숫자·일반 범주 판정으로 fallback해 같은 집합을 유지한다. 인접 scalar 해독은
  선두 byte의 UTF-8 폭을 확인한 뒤 정확히 한 scalar인지 검증하며 잘못된 byte는 토큰 경계로 취급한다.
- `smart`는 program의 consumption이 허용된 조사·어미를 소비한 token span의 바깥 경계를
  검사한다. 체언, literal, 한 음절 atom은 core 시작도 토큰 경계여야 한다. 단, 조사를 직접
  검색할 때는 붙은 조사를 찾을 수 있도록 core 왼쪽 경계 대신 바로 앞 host와 조사 이형태
  조건을 검증한다. 무품사 입력은 사용자가 쓴 조사 표면형만 찾고, 조사 이형태 묶음 확장은
  명시적 조사 품사 입력에서만 사용한다.
- 명시적 체언 품사의 `smart` program에서 query core가 조사 없는 token 전체와 정확히 같으면,
  compact component resource에 같은 whole 분석이 없어도 완성된 체언 token으로 인정한다.
  이 경로는 token 내부에서 시작한 core, 조사나 다른 문자를 소비한 candidate와 component
  경계를 가로지르는 substring에는 적용하지 않는다.
- 일반 용언의 `smart` token span은 core에서 시작한다. 구조 판정도 선택된 체언+조사 path보다
  임의의 용언 graph path를 우선하거나 명사형 활용의 왼쪽 경계를 완화하지 않는다. 따라서 `가다`
  검색은 `친구가`·`문서가`의 조사 `가`, `되감기`·`민감`의 내부 `감`을 활용형으로 인정하지
  않는다. 지정사처럼 앞 host에 붙는 분석만 별도 왼쪽 환경 검증을 사용한다.
- 지정사 `smart` candidate는 token 전체가 부사로 분석되지 않고, token 왼쪽 경계부터 VCP core
  직전까지 완성된 체언 host 또는 `체언 + 검증된 조사 연쇄`가 있을 때 host에 붙은 VCP runtime
  component를 유지한다. 생성 branch가 token 끝까지 직접 소비하지 못해도 같은 VCP source node가
  core 시작부터 token 끝까지 이어지면 완성된 continuation으로 인정한다. 모음으로 끝나는 host 뒤의
  `다`, `였-`, `여-` branch는 지정사 탈락·축약의 왼쪽 음운 조건과 완결된 활용을 함께 검증한다.
  따라서 `상표다`, `구경거리였다`, `대학뿐이다`의 지정사를 지원하지만, 체언 host가 없거나
  whole-token 부사인 `매일`, 받침 뒤에서 축약한 `대학다`는 이 경로로 열지 않는다.
- 명시적 동사·형용사 품사의 `ending.connective-ji` program은 `smart`에서도 core 왼쪽 token 경계를
  요구하지 않고 완성된 token span의 오른쪽 경계는 유지한다. 이는 gold 어절의 오른쪽 끝과
  일치하는 suffix candidate만 복구한다. 무품사 `smart`, `token`, `any`와 `ending.connective-ji`
  뒤에 문자가 더 남는 left-edge candidate는 바꾸지 않는다.
- 용언의 `ending.past`와 `ending.future` consumption state는 `ending.connective-eudoe`의 `으되`를
  소비한다. `치렀으되`, `하겠으되`처럼 선어말어미 뒤의 완성된 token만 복구하며, bare stem에
  `으되`를 붙이는 별도 경로는 이 규칙으로 추측하지 않는다. Compact resource가 선어말어미 뒤의
  source 어미 경로를 보완할 때도 `Past`, `Future`, `Eu` continuation에서만 완성된 어미 경로를
  사용한다. `으`로 시작하는 표면은 구현된 `으` 이형태 집합과 일치해야 하므로 `으데`처럼
  source 품사만 어미로 분류된 표면은 거부한다. `바꾸었음을`처럼 명사형 전성 어미 뒤에 조사가
  오는 표면은 `용언 + E+ + ETN + J+` 경로와 조사 이형태를 모두 검증한다.
- `-아/어` program이 직접 소비하지 않은 보조용언 연쇄를 compact resource로 보완할 때는 연결
  어미 바깥의 `VX + E*`가 완성되어야 한다. 연결 어미 표면이 query core 바깥에 있거나, 같은
  음절로 축약된 core의 정확한 source 분석이 용언으로만 해석되거나, core 직후의 `VX + E*`와
  core보다 긴 query 품사의 단일 용언 source edge 뒤 어미 경로가 함께 완결되거나, token 전체의 정확한 분석이
  `용언 + EC + VX + E+`이거나, 후행 경로가 결과 변화를 나타내는 `-아/어지다` 계열인 경우에만
  source 연쇄를 사용한다. 따라서 `빼놓을`, `비춰볼`, `생겨났던`, `극심해지겠지만`과 생성
  program이 직접 소비하는 `해가고`는 유지하지만, query core와 무관한 더 긴 용언으로 시작하는
  `가리키는지`와 `해`의 중의적인 source 분석만 있는 `해가며`는 확장하지 않는다. 생성 program이
  token 전체를 소비해도 정확한 source 분석의 용언 component가 query core보다 길면 서로 다른
  구조로 판정한다. 따라서 `가지/VV + ㄴ/ETM`인 `가진`은 `가다`의 결과 변화 활용으로 인정하지
  않는다.
- full-POS `smart`의 `VX` query는 compact resource가 token 왼쪽 경계부터 일반 용언과
  `EC`로 candidate core 직전까지 이어지고, core에 정렬된 `VX`와 선택적 어미가 token 끝까지
  이어지는 완전한 path를 증명할 때 token 내부 보조용언을 유지한다. 용언 시작은 `VV/VA`
  또는 `XR + XSV/XSA`이며, `EC + VX + E+`가 한 source edge에 묶이거나 여러 edge로 나뉜
  경우를 같은 경로로 조립한다. 선행 일반 용언이나 core 직전 `EC`가 없는 내부 substring은
  이 경로로 열지 않는다.
- 이유·근거·전제를 나타내는 `ending.connective-ni`는 `-니/-으니`, `-니까/-으니까`,
  `-니까는/-으니까는`과 그 준말 `-니깐/-으니깐`을 완성된 predicate token으로
  소비한다. 받침 없는 어간과 `ㄹ` 받침 어간은 `으`가 없는 이형태, 그 밖의 받침
  어간은 `으`가 있는 이형태를 쓴다. `살으니까`, `먹니까`, `먹니깐`처럼 잘못된
  이형태와 `부니깐은`처럼 완성된 어미 뒤의 추가 연쇄는 생성하지 않는다.
- `ending.prospective-quotative` 뒤의 source 조사는 topic `는`만 허용한다. `이기리라고는`은
  `용언 + EC + JX` 경로로 유지하지만 additive `도`를 붙인 `먹으리라고도`는 확장하지 않는다.
- 동작 용언의 `-ㄴ다/는다`와 상태 용언의 `-다` 현재 평서형 program은
  `ending.declarative` consumption state에서 제한된 continuation만 소비한다. 상태 용언은 형용사와
  보조 형용사이며 지정사와 부정 지정사 `아니다`는 포함하지 않는다. 허용 목록은
  `고`(`ending.quotative-go`),
  `는`(`ending.quotative-adnominal`),
  `던`(`ending.quotative-retrospective`), `면`(`ending.conditional`),
  `니`(`ending.quotative-ni`), `며`(`ending.quotative-myeo`),
  `면서`(`ending.quotative-myeonseo`), `는데`(`ending.quotative-neunde`),
  `지`(`ending.quotative-ji`)다. `쓴다고`, `먹는다는`, `받든다는`, `함께한다던`, `좋다는`,
  `나쁘다면`, `어렵다면서`처럼 이 목록으로 끝나는 token과 bare `쓴다`, `먹는다`, `좋다`는
  허용한다. 동작 용언의 사전형 `가다`에는 이 상태를 부여하지 않으므로 `가다면`은 거부한다.
  종결형 뒤의 `거나/든가/든지` 조사와 `니요/던데` 같은 추가 연쇄는 이 상태에서 추측하지 않는다.
- 체언의 접속 조사 `이면/면`은 받침 있는 host 뒤에서 `이면`, 받침 없는 host 뒤에서 `면`을
  소비한다. `백이면 백`, `공부면 공부`처럼 같은 자격의 대상을 잇는 terminal 조사만 허용하고
  `백면`, `공부이면`과 이 조사 뒤의 추가 조사 연쇄는 거부한다.
- `token`은 모든 품사에서 core 시작과 완성된 token span 끝의 토큰 경계를 검사한다.
- `any`는 좌우 경계를 검사하지 않는다.
- phrase의 `max-gap`은 앞 atom의 `token.end`와 다음 atom의 `token.start` 사이에 있는 Unicode scalar 수다. 음수이거나 순서가 뒤집힌 span은 결합하지 않는다.

## 6. 쿼리 언어와 파싱

### 6.1 토큰화

단순 `split_whitespace()`를 사용하지 않는다. 다음을 지원하는 작은 lexer를 둔다.

```text
공백 구분
작은따옴표와 큰따옴표
백슬래시 이스케이프
품사 태그 접두사
literal 강제
`|` disjunction
`(`·`)` 그룹
```

Query 결합 문법은 다음과 같다. `OWS`는 따옴표와 escape 밖의 선택적 공백이다.

```text
query       = alternative
alternative = sequence *(OWS "|" OWS sequence)
sequence    = primary *(OWS primary)
primary     = atom / "(" OWS alternative OWS ")"
```

예:

```bash
kfind 'n:권한 "접근 제어" v:검증하다' src
```

`"접근 제어"`는 하나의 literal atom으로 처리한다.

따옴표와 escape 밖의 `|`는 양쪽 구 중 하나를 찾는다. 공백 구가 `|`보다 먼저 결합하므로
`A B | C D`는 두 구의 대안이다. 괄호는 결합 순서를 바꾼다. `(A | B) C`와
`A (B | C)`는 중첩 그룹에도 같은 규칙을 적용한다. 연산자 앞뒤 공백은 선택 사항이다.
선행·후행 `|`, 연속 `|`, 빈 그룹과 닫히지 않은 괄호는 원문 byte span이 포함된 문법 오류다.
`|`와 괄호 자체를 검색하려면 escape하거나 인용한다.
괄호 안 대안을 연속해서 결합할 때 가능한 완성 경로는 최대 32개다. 초과하면 해당
연산자 또는 그룹의 원문 byte span을 포함한 오류를 반환하며 경로를 전개하지 않는다.

### 6.2 AST 구조

```rust
pub struct QueryAst {
    pub atoms: Vec<QueryAtom>,
    pub composition: QueryComposition,
    pub graph: Option<QueryGraph>,
    pub phrase: PhrasePolicy,
}

pub enum QueryComposition {
    Phrase,
    Disjunction,
    Grouped,
}

pub struct QueryGraph {
    pub starts: Vec<bool>,
    pub ends: Vec<bool>,
    pub predecessors: Vec<Vec<usize>>,
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

제품 query analyzer는 `LexiconQueryAnalyzer`다. 다른 analyzer adapter도 쿼리 atom만 분석하고
결과를 공통 `Analysis`로 변환해야 한다. surface matcher와 파일 검색 계층은 analyzer 종류를
알지 못한다.

## 7. 중간 표현과 검색 계획

### 7.1 상위 구조

```rust
pub struct QueryPlan {
    pub raw_query: Box<str>,
    pub atoms: Vec<AtomPlan>,
    pub composition: QueryComposition,
    pub graph: Option<QueryGraph>,
    pub phrase_policy: PhrasePolicy,
    pub limits: PlanLimits,
}

pub struct AtomPlan {
    pub analyses: Vec<Analysis>,
    pub programs: Vec<CandidateProgram>,
    pub boundary: BoundaryPolicy,
}

pub struct CandidateProgram {
    pub anchor: Vec<u8>,
    pub core_mapping: CoreMapping,
    pub consumption: CandidateConsumption,
    pub decision: CandidateDecision,
    pub origins: Vec<Origin>,
}

pub enum CandidateConsumption {
    Anchor,
    PredicateContinuation { /* DFA state, POS, rule vocabulary, left context */ },
    NominalParticleChain { /* allowed and blocked rule vocabulary */ },
    DirectParticleHost { /* particle rule */ },
}

pub enum CandidateDecision {
    Boundary(BoundaryProof),
    Structural(StructuralConstraint),
}

pub struct StructuralConstraint {
    pub patterns: Vec<QueryMorphPattern>,
    pub boundary: BoundaryProof,
}

pub struct QueryMorphPattern {
    pub lexical_form: Box<str>,
    pub fine_pos: DataFinePos,
    pub continuation: MorphContinuation,
    pub component_capability: ComponentCapability,
    pub adjacent: Vec<AdjacentTokenConstraint>,
}

pub struct Origin {
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
}
```

phrase plan은 source atom마다 하나의 `AtomPlan`을 유지한다. Grouped plan은 atom별 시작·끝
표시와 선행 atom 간선으로 순서 경로를 표현한다. 괄호가 만든 조합을 나열하지 않고 최대 32개
atom과 32개 완성 경로, 그 사이 간선만 허용하며, 모든 후보를 한 번의 anchor scan에서 검증한다. 같은 시작점의
일치는 끝이 가장 긴 경로를 선택하고 같은 span의 경로는 쿼리 순서로 결정한다.
Disjunction plan은 alternative의
분석과 program을 하나의 논리 atom으로 합쳐 모든 anchor를 한 번의 scan으로 찾는다. 합칠 때
`Origin.analysis_index`를 최종 분석 배열에 맞게 다시 매겨 어느 alternative가 match했는지
provenance로 보존한다. 같은 span을 만드는 alternative는 span을 중복 반환하지 않고 origin을
합친다. `phrase_policy`와 `--max-gap`은 순서대로 결합되는 atom 사이에 적용한다.

- `CandidateProgram`은 anchor 탐색·core 투영·후보 범위 열거·anchor 이후 소비·판정 제약을
  한번만 표현하는 query-owned 실행 IR이다. `CandidateConsumption`은 실제 token span을 만드는
  continuation과 rule vocabulary만 선언한다. matcher와 품질 검증기는 같은 program을 실행하며,
  별도 branch를 재구성하거나 consumption 종류에서 `extent`를 추론하지 않는다.
- exact 후보는 `Anchor`, 용언 연속 후보는 `SurroundingToken`, 조사가 없을 수도 있는
  체언은 `AnchorAndSurroundingToken`을 사용한다. 모든 후보는
  `core ⊆ anchor ⊆ consumed ⊆ token` 불변식을 만족한다.
- `QueryMorphPattern`은 표면형 예외 목록이 아니라 어휘·세부 품사·continuation DFA·component
  capability·인접 token 제약을 선언한다. 여러 분석이 같은 anchor를 공유하면 pattern
  합집과 모든 `Origin`을 보존한다.
- `Boundary`는 literal, `token`, `any` 및 구조 판정이 필요 없는 경로에만 사용한다.
  `Structural`은 bounded token graph에서 구조적으로 다른 경쟁 경로를 평가하되,
  같은 structural signature 안의 어휘 의미 차이는 추가로 열거하지 않는다.
- `BranchVerifier`, `ContextRequirement`, 수동 lexical-context surface registry,
  exact-component 비용 마진과 예외 fallback은 query·matcher 실행 경로에 존재하지 않는다.

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

동일 branch가 여러 분석에서 생성되면 origins를 합친다. compiler는 정규화된 anchor로
branch 후보를 먼저 묶고 같은 anchor 안에서 core 투영, consumption, boundary와 decision이
같은지 비교한다. 여러 branch가 공유하는 rule vocabulary 전체를 branch마다 다시 hash하지
않으며, 최초 생성 순서와 origin 정렬은 기존 plan 계약대로 보존한다.
