# 사전 확장 전략

## 목표

core 사전을 예외 metadata로 유지하면서 full POS coverage와 활용 정확도를 확장한다.
런타임 문장 분석기와 네트워크 의존성은 추가하지 않는다.

## 데이터 계층

| 계층 | 책임 | 배포 |
| --- | --- | --- |
| 생산 규칙 | 접미 파생, 규칙 활용, 어미 continuation | 바이너리 |
| core lexicon | 불규칙 활용, 중의성, 기능어, override | 바이너리 |
| full POS | 일반 표제어와 품사 | 별도 resource |
| enriched morphology | 공개 사전의 검증된 활용 metadata | 별도 resource 후보 |
| user lexicon | 프로젝트·조직 고유어 | 사용자 파일 |

일반 명사는 미등록 한글 입력도 nominal 후보가 되므로 대규모 core 목록이 필요하지 않다.
사전 coverage가 직접 필요한 영역은 auto 품사의 용언·대명사·수사·관형사·부사와 불규칙
활용이다.

## 공개 source

### mecab-ko-dic

현재 bootstrap source다. Apache-2.0이고 고정 URL과 SHA-256로 재현할 수 있다. 표제어·품사
coverage에는 적합하지만 활용 분류를 직접 제공하지 않으므로 full POS 후보로만 사용한다.

### 국립국어원 사전

- [한국어기초사전 전체 내려받기](https://krdict.korean.go.kr/kor/dicSearchDetail/searchDetailMorpheme)는 표제어, 품사, 활용을 선택해 JSON/XML 등으로 받을 수 있다.
- [한국어기초사전 Open API](https://krdict.korean.go.kr/kor/openApi/openApiInfo)는 표제어와 품사를 제공하지만 인증키와 호출 제한이 있다.
- [우리말샘 Open API](https://opendict.korean.go.kr/service/openApiInfo)는 표제어, 품사, 활용·준말 필드를 제공한다.
- [표준국어대사전 Open API](https://stdict.korean.go.kr/openapi/openApiInfo.do)도 표제어, 품사, 활용 필드를 제공한다.

세 누리집의 일반 텍스트는 CC BY-SA 2.0 KR 정책이다. 파생 artifact는 소스 코드와 분리하고
저작자 표시·동일조건변경허락 고지를 포함한다. 출전이 있는 용례와 멀티미디어는 수집하지
않는다.

## 수집 계약

1. 릴리스 입력은 전체 내려받기 snapshot만 허용한다.
2. 원본 snapshot은 저장소에 넣지 않고 URL 또는 요청 절차, 생성 일자, SHA-256를 기록한다.
3. extractor는 표제어, 품사, 활용·준말 중 필요한 필드만 읽는다.
4. NFC 정규화, 지원 품사 mapping, 중복 제거, 제외 이유를 재현한다.
5. 산출물에 source별 입력·출력·충돌 count와 라이선스를 포함한다.
6. Open API는 갱신 후보와 소수 항목 검증에만 사용한다.

## 활용 metadata 승격

공개 활용형을 현재 규칙들의 출력과 비교한다. 특정 alternation에서만 생성되는 진단형이
있고 다른 진단형과 충돌하지 않을 때만 자동 후보로 만든다. 후보는 독립 fixture로 검증한
뒤 core 또는 enriched morphology resource에 승격한다. source 충돌과 미지원 활용은 review
report에 남긴다.

## 우선순위

1. 현재 full POS artifact의 entry·표제어·품사 통계를 고정한다.
2. benchmark 원인을 embedded/full-POS별로 분리한다.
3. 생산 가능한 continuation 누락을 사전 추가보다 먼저 수정한다.
4. 한국어기초사전 snapshot importer를 추가해 MeCab 품사와 교차 검증한다.
5. 활용형 기반 enriched morphology artifact schema를 설계한다.
6. 새로운 blind source에서 품질과 hard-negative 정밀도를 확인한다.
