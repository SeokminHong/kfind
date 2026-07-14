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
| enriched morphology | 공개 사전의 검증된 활용 metadata | 별도 라이선스 데이터 |
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

세 누리집의 일반 텍스트는 CC BY-SA 2.0 KR 정책이다. [한국어기초사전 저작권
정책](https://krdict.korean.go.kr/kor/kboardPolicy/copyRightTermsInfo), [우리말샘 저작권
정책](https://opendict.korean.go.kr/service/copyrightPolicy), [표준국어대사전 저작권
정책](https://stdict.korean.go.kr/join/copyrightPolicy.do)을 snapshot별로 다시 확인한다.
파생 artifact는 소스 코드와 분리하고 저작자 표시·동일조건변경허락 고지를 포함한다. 출전이
있는 용례와 멀티미디어는 수집하지 않는다.

## 현재 제품 기준선

`mecab-ko-dic 2.1.1-20180720` 산출물은 다음 규모다.

- 품사 entry 632,667개
- 고유 표제어 614,794개
- 용언 표제어 9,407개
- 둘 이상의 품사를 가진 표제어 16,650개
- 둘 이상의 용언 품사를 가진 표제어 383개

component-aware `smart`를 사용하는 현재 1,000-case test와 dev 결과는 다음과 같다.

| fixture/profile | TP / FP / FN | recall |
| --- | ---: | ---: |
| test embedded | 408 / 1 / 92 | 81.6% |
| test full-POS | 413 / 1 / 87 | 82.6% |
| dev embedded | 432 / 2 / 68 | 86.4% |
| dev full-POS | 436 / 2 / 64 | 87.2% |

명시적 품사 test에서 full-POS가 추가로 찾는 5건은 모두 명사다. 품사를 생략한 사람용
fixture에서는 기대 품사 plan 포함률이 embedded 46.8%, full-POS 96.4%다. full-POS의 주된
제품 가치는 auto 품사 coverage이며, 남은 명시적 품사 품질은 활용 metadata와 경계 규칙의
영향이 더 크다.

## 표제어 위생

MeCab CSV의 표면형을 모두 사전 표제어로 볼 수는 없다. `VCP.csv`에는 `이다`의 어간 `이`
외에도 `보이`, `사이` 같은 문맥용 표면형이 있다. 이를 기계적으로 `-다`형으로 바꾸면
`보이다`, `사이다`라는 잘못된 지정사 분석이 생긴다. extractor는 `VCP=이`, `VCN=아니`만
받고 나머지 14개 문맥용 지정사 표면형을 제외한다. 형태 생성기도 `이다` 이외의 VCP stem을
거부한다.

VV·VA·VX 후보도 공개 사전 snapshot과 교차 검증하기 전에는 활용 metadata로 승격하지
않는다. MeCab full POS는 품사 후보 계층이며 core의 불규칙 분류를 덮어쓰지 않는다.

미등록 `-다` 입력을 모두 용언으로 간주하지 않는다. `가볍다`처럼 검증된 표제어는 core의
`ㅂ` 불규칙 분석을 사용하지만 미등록·오입력 `-다`형은 literal과 진단만 생성한다. 사전에
없는 신규 용언은 `--pos verb|adjective` 또는 user lexicon으로 명시할 수 있다.

## 수집 계약

1. 릴리스 입력은 전체 내려받기 snapshot만 허용한다.
2. 원본 snapshot은 저장소에 넣지 않고 URL 또는 요청 절차, 생성 일자, SHA-256를 기록한다.
3. extractor는 표제어, 품사, 활용·준말 중 필요한 필드만 읽는다.
4. NFC 정규화, 지원 품사 mapping, 중복 제거, 제외 이유를 재현한다.
5. 산출물에 source별 입력·출력·충돌 count와 라이선스를 포함한다.
6. Open API는 갱신 후보와 소수 항목 검증에만 사용한다.

## 한국어기초사전 importer 계획

구현 범위는 다음 작업으로 분리한다.

1. 전체 내려받기에서 표제어·품사·활용을 선택한 JSON 또는 XML snapshot을 받는다.
2. 원본은 저장소 밖 cache에 두고 생성 일자, 요청 옵션, SHA-256, 라이선스만 manifest에
   고정한다.
3. importer는 원본 schema를 내부 `lemma`, `pos`, `conjugations`, `source_id` 후보로 바꾸고
   NFC 정규화·중복 제거·제외 이유를 기록한다.
4. 작은 schema fixture로 parser를 테스트하되 실제 사전 본문은 test fixture에 복제하지
   않는다.
5. MeCab과 품사가 일치하는 표제어, 충돌하는 표제어, 한 source에만 있는 표제어를 각각
   report한다.
6. 활용형은 자동 반영하지 않고 현재 생성 규칙과 일치 여부를 비교한 `alternation candidate`
   report로 낸다.
7. 검토된 결과만 enriched morphology artifact로 만들고 core > enriched morphology > MeCab
   POS 후보 순으로 우선한다.

Open API adapter를 함께 만들더라도 기본 경로는 snapshot importer다. API adapter는
source ID로 소수 충돌 항목을 재확인하고 새 snapshot 필요 여부를 판단하는 도구로 한정한다.

## 활용 metadata 승격

공개 활용형을 현재 규칙들의 출력과 비교한다. 특정 alternation에서만 생성되는 진단형이
있고 다른 진단형과 충돌하지 않을 때만 자동 후보로 만든다. 후보는 독립 fixture로 검증한
뒤 core 또는 enriched morphology resource에 승격한다. source 충돌과 미지원 활용은 review
report에 남긴다.

원시 레코드는 source ID와 동형어 번호를 보존한다. `(lemma, fine_pos)`는 source 간 집계에만
사용한다. `이르다/VV`의 `이르러`와 `일러`처럼 서로 다른 동형어가 서로 다른 alternation을
가질 수 있으므로, 검증된 분석은 하나로 덮어쓰지 않고 합집합으로 보존한다. core와 enriched에
같은 분석이 있으면 core만 남기고, full POS 규칙형 fallback은 같은 세부 품사의 core 또는
enriched 분석이 없을 때만 추가한다.

첫 승격 범위는 `ReuDoubleL`, `Reo`, 규칙 `EU_DROP`의 구분이다. 한국어기초사전과
표준국어대사전이 같은 alternation을 지지하는 항목을 자동 승격 후보로 삼고, 우리말샘은
중복 출처가 많으므로 독립 표결 대신 충돌·누락 검토에 사용한다. `다르다→달라`,
`푸르다→푸르러`, `이르다→이르러/일러`를 양성 fixture로, `들르다→들러`와
`치르다→치러`를 regular `EU_DROP` 음성 fixture로 고정한다.

한국어기초사전 XML snapshot에 XML 1.0 비허용 바이트가 있으면 고정 manifest에 기록된 종류,
개수와 위치만 제거한다. 원본 SHA-256과 정제 내역을 함께 기록하고 예상 값이 달라지면 importer를
실패시킨다.

## 우선순위

1. 한국어기초사전 snapshot importer로 MeCab 품사와 표제어 기본형을 교차 검증한다.
2. source별 일치·충돌·단독 표제어와 활용 후보를 재현 가능한 report로 만든다.
3. 검증된 활용 metadata만 enriched morphology artifact로 승격한다.
4. 새로운 unseen source에서 auto 품사 coverage와 hard-negative 정밀도를 확인한다.

## blind 평가 게이트

새 규칙과 사전 source가 확정된 뒤 기존 dev/test와 다른 한국어 treebank source를 고정한다.
URL·라이선스·SHA-256·quota를 먼저 기록하고 결과를 한 번 확인한다. blind 결과를 확인한 뒤
case별 core entry나 예외 branch를 추가하지 않는다. 실패 분석은 다음 개발 주기의 dev
입력으로만 사용한다.
