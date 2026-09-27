# kfind 기술 사양서

워크스페이스 버전: 1.1.0
문서 역할: 현재 구현과 호환성 계약의 색인

이 문서는 현재 제품 계약만 유지한다. 완료한 작업 순서, 폐기한 대안과 배포 운영 상태는
누적하지 않는다. 재현 가능한 시점별 측정값은 `docs/benchmarks`의 보고서에 둔다.

이 문서는 각 분야의 현행 사양을 가리키는 규범적 색인이다.

- [쿼리 언어와 검색 계획](query-language.md)
- [형태 규칙과 검색 실행](matching.md)
- [규칙 데이터와 사전](resources.md)
- [사용법과 CLI 계약](cli.md)
- [Rust와 JavaScript API](bindings.md)
- [웹과 패키지 배포](distribution.md)
- [검증과 성능](verification.md)

## 0. 제품 계약

이 절은 현재 구현에 적용되는 세부 계약이다. 아래 내용은 뒤 절의 일반 설명보다 우선한다.

### 0.0 제품 목적과 우선순위

- `kfind`의 주 목적은 에이전트와 사람이 한국어 표제어·활용형을 파일 또는 메모리 text에서
  빠르게 찾아 후속 판단에 사용할 후보 span과 생성 근거를 얻는 것이다. 형태 지식은 query를
  유한한 검색 계획으로 만드는 데 사용하며 corpus 전체를 분석하는 제품으로 확장하지 않는다.
- 에이전트 workflow는 명시적 품사, 넓은 경계, 구조화된 출력으로 recall과 scan 처리량을
  우선하고 caller가 문맥으로 false positive를 제거한다. 사람 workflow는 자동 품사와 `smart`
  경계로 precision과 사용성을 우선한다. CLI와 Rust·JavaScript library는 두 workflow에서 같은
  query·span·provenance 계약을 제공한다.
- 명시적 품사와 `smart`를 함께 사용하는 검색은 false negative를 false positive보다 우선해
  줄인다. 고정 development fixture에서 FN을 먼저 최소화하고, 같은 FN이면 FP가 적은 후보를
  선택한다. precision 99.00% 하한과 version-controlled hard-negative의 신규 FP 0은 유지한다.
- 무품사 검색은 query 의도를 하나의 품사로 확정할 수 없는 한계를 결과에 포함한다. 이 한계를
  감추기 위해 fixture, gold, negative 선택이나 지표 정의를 현재 구현 결과에 맞춰 바꾸지 않는다.
- 제품 goal은 짧은 query의 bounded compile, 대규모 text의 빠른 scan, 지원하는 형태 범위의
  검증된 recall·precision, 재현 가능한 결과와 offline 실행이다.
- 일반 목적 문장 형태소 분석·tokenization, 형태소 분석기 자체의 최고 처리량 경쟁, 문맥 의미
  판별, 동음이의어·동형이의어 해소, semantic search와 임의 표면형의 완전한 역분석은
  non-goal이다. 외부 형태소 분석기 비교는
  제품 workflow의 품질·비용을 보정하는 근거이며 동일한 tokenizer backend 순위를 뜻하지 않는다.
- `걸음을 걷다`, `팔을 걷다`, `발을 걸다`의 각 표제어 query는 형태·표면 규칙이
  허용하면 모두 `걸었고`에 match할 수 있다. corpus 문맥으로 어느 의미인지
  고르지 않으며, 의미적 중의성은 false positive로 계산하지 않는다.
- `smart`는 corpus 전체를 분석하지 않지만, compact component resource가 증명하는
  바로 인접한 어절의 형태 구조를 bounded evidence로 사용할 수 있다. 이 근거는
  품사·component·continuation 가능성을 검증하지 의미를 판별하지 않는다.

### 0.1 규칙 데이터와 품질 기준

[규칙 데이터와 사전](resources.md)

### 0.2 토큰 경계와 phrase 거리

[쿼리 언어와 검색 계획](query-language.md)

### 0.3 CLI 세부 정책

[사용법과 CLI 계약](cli.md)

### 0.4 Web 문서와 playground

[웹과 패키지 배포](distribution.md)

### 0.5 Homebrew 대상

[웹과 패키지 배포](distribution.md)

### 0.6 구조 기반 국소 형태 판정

[형태 규칙과 검색 실행](matching.md)

### 0.7 Rust 라이브러리와 WASM 대상

[Rust와 JavaScript API](bindings.md)

### 0.8 npm 패키지

[웹과 패키지 배포](distribution.md)

## 1. 문서 목적

`kfind`는 에이전트와 사람이 입력한 한국어 표제어 또는 짧은 구(句)를 조사 결합, 어미 결합,
불규칙 활용과 일부 생산적 파생 규칙에 따라 검색 계획으로 컴파일하고, 소스 코드·문서 파일과
메모리 text에서 후보 span을 빠르게 찾는 CLI·library다.

이 도구는 코퍼스 전체를 형태소 분석하지 않는다. 입력 쿼리 쪽에서만 형태 정보를 해석하고, 원문에서는 빠른 문자열 앵커 검색과 국소 검증만 수행한다.

형태 분석은 제품 자체가 아니라 빠른 text matching을 위한 query planning 수단이다. 결과는
후속 문맥 판단에 사용할 span과 생성 근거이며, 완전한 문장 형태 분석이나 의미 해석이 아니다.

제품 설명은 다음 문구를 기준으로 한다.

> 한국어 표제어와 활용형을 빠르게 찾는 코드·문서 검색 CLI

저장소 Markdown 문서는 한국어 정본 하나만 유지한다. 웹사이트는 한국어 원문과
영어 번역을 같은 path의 query별 URL에서 제공하고 사용자가 언어를 전환할 수 있어야 한다.

## 2. 아키텍처

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

같은 품사에서 형태 구조가 같은 동음이의어와 동형이의어는 문맥 의미로 구분하지 않는다.
한 표제어에서 생성 가능한 표면형이면 모두 검색 결과로 인정한다. 다만 whole/component
분해, 품사 또는 인접 문장 성분 배치가 다른 경우에는 bounded 구조 근거로 구분한다.

예:

```text
검색어: 걷다

길을 걸어 갔다.     match
전화를 걸어 봤다.   match
```

두 번째 결과는 의미상 `걸다`지만, 문맥 판별은 이 제품의 범위가 아니다.

```text
검색어: v:박다

값이 한 박자 늦게 저장된다.   full-POS smart: no match
못을 박자 바로 고정됐다.      full-POS smart: match
```

첫 번째 `박자`는 앞 관형사와 whole 명사 분석이 만드는 체언 frame을 선택한다.
`boundary=any`는 두 표면을 모두 후보로 반환하고 caller가 문맥을 판별한다.

```text
검색어: v:주다

주지 스님이 법회를 열었다.   full-POS smart: no match
사탕을 주지 말자.            full-POS smart: match
```

`지/EC`로 끝나는 용언 분석이 현재 어절 전체의 명사 분석과 겹치고 다음 어절에 경쟁 없는
완성 체언 경로가 있으면 `smart`는 충돌하는 용언 후보를 제외한다. 다음 어절에 완성 용언
경로가 있거나 다른 연결 어미를 쓰는 용언 continuation, 다른 품사와 어절 내부 component는
이 규칙으로 막지 않는다.

## 3. 핵심 구현 계약

[형태 규칙과 검색 실행](matching.md)

### 3.1 검색 앵커와 후보 판정을 분리한다

[형태 규칙과 검색 실행](matching.md)

### 3.2 용언 분류와 활용 생성을 분리한다

[형태 규칙과 검색 실행](matching.md)

### 3.3 합성 가능한 어휘 특성을 사용한다

[형태 규칙과 검색 실행](matching.md)

### 3.4 한 표제어의 복수 분석을 보존한다

[형태 규칙과 검색 실행](matching.md)

### 3.5 사전과 명시적 품사로 용언을 판별한다

[형태 규칙과 검색 실행](matching.md)

### 3.6 확장, 경계와 품사를 분리한다

[형태 규칙과 검색 실행](matching.md)

## 4. 사용자 사용법

[사용법과 CLI 계약](cli.md)

### 4.1 기본 검색

[사용법과 CLI 계약](cli.md)

### 4.2 품사 강제

[사용법과 CLI 계약](cli.md)

### 4.3 구(句) 검색

[사용법과 CLI 계약](cli.md)

### 4.4 검색 범위 제어

[사용법과 CLI 계약](cli.md)

### 4.5 해석 확인

[사용법과 CLI 계약](cli.md)

### 4.6 사람과 에이전트의 권장 경로

[사용법과 CLI 계약](cli.md)

## 5. 범위와 비범위

### 5.1 지원 범위

지원하는 큰 품사 범주는 다음과 같다.

| 범주   | 세부 범주                           | 기본 동작                            |
| ------ | ----------------------------------- | ------------------------------------ |
| 체언   | 명사, 대명사, 수사, 의존명사        | 기본형, 복수 표지, 조사 연쇄 검증    |
| 용언   | 동사, 형용사, 지정사, 일부 보조용언 | 어간 교체, 어미 결합, 축약 검증      |
| 수식언 | 관형사, 부사                        | literal과 품사별 경계, 선택적 보조사 |
| 관계언 | 조사                                | 이형태 묶음과 경계 검증              |
| 독립언 | 감탄사                              | literal과 토큰 경계                  |
| 기타   | 코드 식별자, 외국어, 숫자           | literal                              |

### 5.2 비범위

현재 제품은 다음을 제공하지 않는다.

- 일반 목적 문장 형태소 분석기 또는 tokenizer API
- 형태소 분석기 자체의 최고 처리량·정확도 경쟁
- 문서 전체 형태소 분석
- 문맥 의미 기반 동음이의어 구분
- 임의 활용형의 표제어 역분석
- 동의어, 유의어, 의미 검색
- 사용자 정규식
- 치환 기능
- 방언, 고어, 비표준 활용의 포괄적 지원
- 문장 단위 구문 분석
- 무제한 어미 연쇄 생성

입력 `걸어`는 기본적으로 literal 또는 미등록 체언 후보로 처리한다. `걸어`에서 `걷다`와 `걸다`를 역추론하는 기능은 별도 후속 범위다.

## 6. 쿼리 언어와 파싱

[쿼리 언어와 검색 계획](query-language.md)

### 6.1 토큰화

[쿼리 언어와 검색 계획](query-language.md)

### 6.2 AST 구조

[쿼리 언어와 검색 계획](query-language.md)

### 6.3 분석 결과

[쿼리 언어와 검색 계획](query-language.md)

### 6.4 query analyzer 인터페이스

[쿼리 언어와 검색 계획](query-language.md)

## 7. 중간 표현과 검색 계획

[쿼리 언어와 검색 계획](query-language.md)

### 7.1 상위 구조

[쿼리 언어와 검색 계획](query-language.md)

### 7.2 핵심 span과 토큰 span

[쿼리 언어와 검색 계획](query-language.md)

### 7.3 표면형 provenance

[쿼리 언어와 검색 계획](query-language.md)

## 8. 한국어 음절 처리

[형태 규칙과 검색 실행](matching.md)

### 8.1 내부 정규화

[형태 규칙과 검색 실행](matching.md)

### 8.2 필요한 음절 연산

[형태 규칙과 검색 실행](matching.md)

## 9. 형태 규칙 엔진

[형태 규칙과 검색 실행](matching.md)

### 9.1 세 계층

[형태 규칙과 검색 실행](matching.md)

### 9.2 어미 모델

[형태 규칙과 검색 실행](matching.md)

### 9.3 공통 규칙

[형태 규칙과 검색 실행](matching.md)

### 9.4 어휘 사전이 필요한 교체

[형태 규칙과 검색 실행](matching.md)

### 9.5 필수 활용 범위

[형태 규칙과 검색 실행](matching.md)

## 10. 품사별 컴파일 규칙

[형태 규칙과 검색 실행](matching.md)

### 10.1 체언

[형태 규칙과 검색 실행](matching.md)

### 10.2 대명사와 수사

[형태 규칙과 검색 실행](matching.md)

### 10.3 동사와 형용사

[형태 규칙과 검색 실행](matching.md)

### 10.4 관형사

[형태 규칙과 검색 실행](matching.md)

### 10.5 부사

[형태 규칙과 검색 실행](matching.md)

### 10.6 조사

[형태 규칙과 검색 실행](matching.md)

### 10.7 감탄사

[형태 규칙과 검색 실행](matching.md)

## 11. 앵커 계획

[형태 규칙과 검색 실행](matching.md)

### 11.1 앵커 선택 원칙

[형태 규칙과 검색 실행](matching.md)

### 11.2 적응형 matcher

[형태 규칙과 검색 실행](matching.md)

### 11.3 program 제한

[형태 규칙과 검색 실행](matching.md)

## 12. 검색 실행 엔진

[형태 규칙과 검색 실행](matching.md)

### 12.1 파일 순회

[형태 규칙과 검색 실행](matching.md)

### 12.2 파일 읽기와 줄 검색

[형태 규칙과 검색 실행](matching.md)

### 12.3 검증 단계

[형태 규칙과 검색 실행](matching.md)

### 12.4 phrase 결합

[형태 규칙과 검색 실행](matching.md)

### 12.5 병렬 출력

[형태 규칙과 검색 실행](matching.md)

## 13. 인코딩과 바이너리 정책

[사용법과 CLI 계약](cli.md)

## 14. CLI 사양

[사용법과 CLI 계약](cli.md)

### 14.1 기본 구문

[사용법과 CLI 계약](cli.md)

### 14.2 주요 옵션

[사용법과 CLI 계약](cli.md)

### 14.3 context와 출력 호환 옵션

[사용법과 CLI 계약](cli.md)

### 14.4 종료 코드

[사용법과 CLI 계약](cli.md)

### 14.5 표시 언어

[사용법과 CLI 계약](cli.md)

### 14.6 Agent 통합 관리

[사용법과 CLI 계약](cli.md)

## 15. 출력 사양

[사용법과 CLI 계약](cli.md)

### 15.1 기본 출력

[사용법과 CLI 계약](cli.md)

### 15.2 쿼리 설명

[사용법과 CLI 계약](cli.md)

### 15.3 match 설명

[사용법과 CLI 계약](cli.md)

### 15.4 JSON Lines 출력

[사용법과 CLI 계약](cli.md)

## 16. 데이터 사양

[규칙 데이터와 사전](resources.md)

### 16.1 저장소 구조

[규칙 데이터와 사전](resources.md)

### 16.2 용언 사전

[규칙 데이터와 사전](resources.md)

### 16.3 빌드 산출물

[규칙 데이터와 사전](resources.md)

### 16.4 사용자 사전

[규칙 데이터와 사전](resources.md)

### 16.5 사전 bootstrap 전략

[규칙 데이터와 사전](resources.md)

### 16.6 외부 사전 데이터 정책

[규칙 데이터와 사전](resources.md)

## 17. Rust 기술 스택

[웹과 패키지 배포](distribution.md)

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
    Hangul operations, lexicon, endings, alternations, consumption

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

[검증과 성능](verification.md)

### 19.1 최적화 엔진과 참조 엔진을 분리한다

[검증과 성능](verification.md)

### 19.2 단위 테스트

[검증과 성능](verification.md)

### 19.3 속성 테스트

[검증과 성능](verification.md)

### 19.4 퍼징

[검증과 성능](verification.md)

### 19.5 정답 corpus

[검증과 성능](verification.md)

#### 19.5.1 현실 기술 코퍼스 blind fixture

[검증과 성능](verification.md)

### 19.6 외부 분석기 비교

[검증과 성능](verification.md)

### 19.7 독립 형태소 벤치마크

[검증과 성능](verification.md)

### 19.8 형태 질의와 정규식 검색 기준선

[검증과 성능](verification.md)

## 20. 성능 사양

[검증과 성능](verification.md)

### 20.1 목표

[검증과 성능](verification.md)

### 20.2 검색 corpus

[검증과 성능](verification.md)

### 20.3 측정 구간

[검증과 성능](verification.md)

### 20.4 회귀 정책

[검증과 성능](verification.md)

## 21. Native package 배포

[웹과 패키지 배포](distribution.md)

### 21.1 배포 형태

[웹과 패키지 배포](distribution.md)

### 21.2 formula 설치 항목

[웹과 패키지 배포](distribution.md)

### 21.3 Homebrew bottle 배포

[웹과 패키지 배포](distribution.md)

### 21.4 Chocolatey 배포

[웹과 패키지 배포](distribution.md)

### 21.5 릴리스 자동화

[웹과 패키지 배포](distribution.md)

## 22. 보안과 견고성

- 메모리에 일괄 적재하는 full POS와 component resource는 각각 128 MiB, enriched predicate와
  user lexicon text는 각각 16 MiB로 제한한다. Native CLI는 metadata의 크기를 먼저 거부하고
  metadata가 없거나 읽는 중 파일이 바뀌는 경우에도 `limit + 1` byte까지만 읽어 상한을 다시
  검사한다. 검색 대상 파일은 이 일괄 적재 상한의 적용 대상이 아니며 streaming search 계약을
  따른다.
- candidate program 수에 상한을 둔다.
- phrase matcher의 DP 상태는 검증된 atom span 수의 합에 비례하며, 전체 조합용 API의 중간
  partial 수에는 명시적 상한을 둔다.
- 사용자 사전 파싱 오류에는 파일명과 줄 번호를 표시한다.
- symlink 순환을 방지한다.
- 검색 결과, 검색 중 issue, 초기화 오류를 포함한 모든 사람이 읽는 출력에 escape 정책을 적용해 제어 문자가 터미널 동작을 바꾸지 않게 한다.
- JSON에는 원문 제어 문자를 정상 escape한다.
- 파일 경로가 유효 UTF-8이 아니어도 처리한다.
- broken pipe에서 panic하지 않는다.
- matcher와 constraint resolver는 unsafe 없이 구현하는 것을 기본 원칙으로 한다.

## 23. 제품 인수 기준

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
11. macOS arm64에서 formula test가 통과한다.
12. 사용자 사전 없이도 핵심 불규칙 fixture가 통과한다.
13. Homebrew 기본 설치에서 full POS lexicon이 로드되고, 사전 누락 시 명확한 진단을 출력한다.
14. 공개 Rust 라이브러리가 동일한 query plan과 matcher를 사용해 메모리 입력을 검색한다.
15. 공개 라이브러리와 핵심 의존 crate가 Rust 1.97의 `wasm32-unknown-unknown` target에서
    빌드된다.
16. `kfind` npm 산출물의 Node smoke test, TypeScript declaration 검사와
    `npm pack --dry-run`이 통과한다.
17. native package는 compact component resource를 `share/kfind`에 설치하고 resource 누락·손상 시
    component `smart` query를 초기화 오류로 종료한다.
18. WASM binary는 compact artifact를 포함하지 않고, 외부 또는 별도 정적 asset bytes를 받은
    생성자에서 schema·source·digest를 검증한다.
19. man page와 한국어 README가 사람용 무품사 기본 경로와 에이전트 자동화 경로를 구분하고,
    에이전트 예시는 명시적 품사, `any`, embedded와 JSON 출력을 사용한다.
20. 품사를 생략한 held-out 검색의 품질·성능 benchmark가 별도 fixture와 보고서 절로 존재한다.
21. `kfind --init`은 TTY checkbox, 반복 `--agent`, 비TTY stdin에서 같은 agent 대상 집합을
    설치한다.
22. Claude Code, Codex와 Gemini CLI의 project skill 경로에 같은 원본의 `SKILL.md`,
    `SessionStart` 지침 hook과 shell tool 실행 전 kfind hook을 설치하며, 기존 agent 설정과
    다른 hook을 보존하고 `custom`은 skill 원문 외의 내용을 stdout에 섞지 않는다.
23. 관리하지 않는 기존 skill은 보존하고 init 실패를 exit code 2와 escape된 진단으로 보고한다.
24. Homebrew formula는 agent skill 원본을 설치하고 project link가 stable `opt` 경로를 사용해
    upgrade 뒤 새 원본을 가리킨다.
25. 일반 text 검색의 POSIX TTY와 Windows console/ConPTY stdin/stdout은 검색 중 결과를
    점진적으로 표시하는 resize 가능한 내장 pager에서 긴 match 줄을 match별 행으로 펼치고
    target 앞뒤 비율에 맞춰 생략하며, `--no-pager`, non-terminal과 agent JSON 출력은 기존
    stdout stream을 유지한다.
26. 배포용 full POS와 compact component resource로 전체 morphology gold를 실행했을 때 자동 품사
    coverage를 포함한 모든 positive와 negative case가 기대값과 일치한다.
27. 설치된 agent hook은 session마다 한국어 형태 검색에 kfind를 사용하라는 지침을 주입한다.
    한글 pattern의 `rg`·`grep` 계열 shell tool call은 고정 문자열 모드를 명시했을 때 허용하고
    그 밖에는 거부한다. 한글 path·glob, pattern file과 kfind 호출은 허용한다.
28. Tagged release의 Windows x64 archive와 Chocolatey package가 같은 checksum 계약을 사용하며,
    archive의 `kfind.exe`는 동적 MSVC·UCRT 의존성이 없고 `--check-data`와 local Chocolatey
    install smoke test가 통과한다.
29. Windows Terminal과 같은 ConPTY 안의 PowerShell에서 일반 text 검색을 실행하면 내장 TUI가
    열리고, resize와 아래 화살표 이동을 반영한 뒤 `q` 입력으로 alternate screen과 raw mode를
    복구하며 정상 종료한다.

## 24. 공개 코드 인터페이스

[Rust와 JavaScript API](bindings.md)

## 25. 제품 원칙

`kfind`의 핵심은 “모든 문장을 분석하는 것”이 아니라 다음 세 가지다.

```text
표제어를 정확히 해석한다.
검색 가능한 형태 규칙을 유한한 계획으로 컴파일한다.
원문에서는 긴 고정 앵커를 찾고 필요한 위치만 검증한다.
```

이 원칙을 지키면 형태 품질은 사전과 규칙 fixture로 개선할 수 있고, 검색 성능은 기존의 검증된 파일 순회·바이트 검색 계층을 활용해 유지할 수 있다.

## 26. 참고 자료

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
