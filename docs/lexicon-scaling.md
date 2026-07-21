# 사전 확장 전략

## 목적

Core 사전은 실행에 필요한 예외와 기능어 metadata를 담당하고, 일반 표제어 품사와
공개 사전 기반 활용 metadata는 별도 resource로 확장합니다. Runtime 문장 분석기와
network 의존성은 추가하지 않습니다.

## 데이터 계층

| 계층 | 책임 | 배포 |
| --- | --- | --- |
| 생산 규칙 | 접사 파생, 규칙 활용, 조사·어미 continuation | binary |
| core lexicon | 불규칙 활용, 중의성, 기능어, override | binary |
| full POS | 일반 표제어와 품사 | 별도 resource |
| enriched morphology | 공개 사전에서 교차 검증한 활용 metadata와 최소 표면형 | 별도 license data |
| user lexicon | 프로젝트·조직 고유어 | 사용자 파일 |

일반 명사는 미등록 한글 입력도 nominal 후보가 될 수 있으므로 core에 전체 명사
목록을 넣지 않습니다. 사전 coverage가 필요한 영역은 자동 품사 판정의 용언,
대명사, 수사, 관형사, 부사와 불규칙 활용입니다.

## 공개 source

### mecab-ko-dic 사전

Full POS 표제어·품사 source로 사용합니다. Apache-2.0이며 URL과 SHA-256을 고정해
재현합니다. 활용 분류를 직접 제공하지 않으므로 불규칙 metadata로 사용하지
않습니다.

### 국립국어원 사전

- [한국어기초사전 전체 내려받기](https://krdict.korean.go.kr/kor/dicSearchDetail/searchDetailMorpheme)는 표제어, 품사와 활용을 선택해 받을 수 있습니다.
- [한국어기초사전 Open API](https://krdict.korean.go.kr/kor/openApi/openApiInfo)는 표제어와 품사를 제공합니다.
- [우리말샘 Open API](https://opendict.korean.go.kr/service/openApiInfo)는 표제어, 품사, 활용과 준말 field를 제공합니다.
- [표준국어대사전 Open API](https://stdict.korean.go.kr/openapi/openApiInfo.do)는 표제어, 품사와 활용 field를 제공합니다.

일반 text는 CC BY-SA 2.0 KR 정책을 따릅니다. 각 snapshot의 저작권 정책을 확인하고
파생 artifact에 저작자 표시와 동일조건변경허락 고지를 포함합니다. 출전이 있는
용례, 정의와 멀티미디어는 수집하지 않습니다.

## 표제어 위생

MeCab CSV의 모든 표면형을 사전 표제어로 사용하지 않습니다. 지정사 CSV의 문맥용
표면형을 기계적으로 `-다`형으로 바꾸면 잘못된 표제어가 생길 수 있습니다.
Extractor는 `VCP=이`, `VCN=아니`만 허용하며 다른 지정사 문맥 표면형을
제외합니다. 형태 generator도 `이다` 이외의 VCP stem을 거부합니다.

VV·VA·VX 후보는 공개 사전 snapshot과 교차 검증한 뒤에만 활용 metadata로
승격합니다. Full POS는 품사 후보 계층이며 core의 불규칙 분류를 덮어쓰지
않습니다.

사전에 없는 `-다` 입력을 모양만 보고 용언으로 간주하지 않습니다. 검증된
표제어는 core 또는 enriched 불규칙 분석을 사용하고, 미등록 입력은 literal과
진단만 생성합니다. 신규 용언은 `--pos verb|adjective` 또는 user lexicon으로
명시합니다.

## 생성과 배포

```sh
scripts/build-full-pos.sh
scripts/build-enriched-predicates.sh
scripts/build-component-resource.sh
```

생성 script는 source URL, checksum과 schema를 검증합니다. Release binary와
component resource는 같은 version header를 가져야 하며 mismatch는 startup
오류입니다. npm package와 Homebrew formula는 동일한 immutable binary-resource
pair를 배포합니다.

사전·resource 변경의 측정 조건과 결과는 `docs/benchmarks`의 날짜별 보고서에
기록합니다. 이 문서는 현재 데이터 책임과 배포 계약만 정의합니다.
