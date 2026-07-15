# 명사·대명사·수사·관형사 smart-boundary 계약

## 제품 동작

`smart`는 문자열 token 경계뿐 아니라 검증된 형태 분석의 완전한 명사·대명사·수사·관형사
component span도 검색 결과로 인정한다. query branch와 같은 fine POS의 component를 포함한 완전
경로가 최저 제외 경로보다 형태 분석 비용 1,500 이하로 높고, 양쪽 경계를 형태 분석으로 증명해야
한다.

positive 예:

- `중국요리`의 `요리`
- `문학작품`의 `문학`
- `사용자권한`과 `권한관리`의 `권한`
- `자기견해`의 `자기`
- `둘다`의 `둘`
- `두사람`의 `두`

negative 예:

- source component 근거가 없는 `대학교`의 `학교`
- component 경계를 가로지르는 `역사과목`의 `사과`
- include 경로 비용이 최저 제외 경로보다 1,500을 초과하는 `산길을`의 `길`
- 더 큰 다른 품사 component에 포함된 `전자기견해`의 `자기`
- 더 큰 명사 component에 포함된 `아들둘레`의 `둘`
- 부사 component에 포함된 `모두사람`의 `두`

단순 substring은 `--boundary any`의 범위다. 특정 corpus 단어 denylist는 사용하지 않는다.

## resource 계약

- `ExactComponent` branch가 있는 `smart` plan만 compact component resource를 사용한다.
- CLI는 설치 resource를 자동으로 찾고 Rust/WASM은 caller가 bytes를 명시한다.
- 누락·손상·schema 또는 source 불일치는 오류이며 기존 token 경계로 fallback하지 않는다.
- component evaluator에 include 경로만 있거나 include 비용이 exclude 비용보다 1,500 이하로
  높으면 match로 복구한다. exclude-only, 비용 마진 초과, 평가 오류와 상한 초과는 거부한다.
- lexical context registry의 whole-token surface 안에서는 문맥 판정이 없을 때 원시 `accept`만
  복구한다.
- literal, `token`, `any`와 component branch가 없는 plan은 resource를 읽지 않는다.

## 검증 계약

- 네 품사의 component positive와 다른 품사·더 큰 component·경계-crossing negative를 같은
  fixture에서 평가한다.
- compact/full morphology resource의 lookup, scoring checksum과 candidate 판정이 일치해야 한다.
- 기존 조사 이형태 verifier를 우회하지 않는다.
- 상세 품질·성능 기준선은
  [Exact component 비용 마진](2026-07-15-exact-component-cost-margin.md)을
  따른다.
