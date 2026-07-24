# 국립국어원 유래 데이터 고지

원자료 저작자·제공자는 국립국어원입니다. kfind는 국립국어원이 제공하는
한국어기초사전, 표준국어대사전과 우리말샘의 고정 snapshot에서 다음 데이터를
추출·정규화·선별합니다.

- `data/enriched/predicates.tsv`와 `REPORT.tsv`
- `data/rules/nikl-modern-endings.tsv`
- `data/rules/nikl-modern-particles.tsv`
- `data/rules/nikl-attached-nominal-suffixes.tsv`

가공 과정은 필요한 표제어·품사·활용형·관련 형태와 source ID의 추출, Unicode
정규화, 동형이의 identity 보존, 중복 제거, 현대어·문법 범주 분류와 여러 사전의
근거 대조를 포함합니다. 사전 용례, 정의, 멀티미디어와 발음 자료는 재배포하지
않습니다. 고정한 snapshot 파일명, checksum, 추출 명령과 artifact checksum은
`data/SOURCES.toml`, enriched record 통계는 `MANIFEST.toml`과 `STATS.toml`에
기록합니다.

위 데이터와 이를 변환하거나 내장한 native binary, WebAssembly, npm·Homebrew
package와 site 배포물의 해당 부분은 저작자표시-동일조건변경허락 2.0 대한민국
라이선스(CC BY-SA 2.0 KR)로 배포합니다. 독립적으로 작성한 kfind source code에는
이 데이터 라이선스가 아니라 저장소의 MIT License를 적용합니다.

- [한국어기초사전 저작권 정책](https://krdict.korean.go.kr/kor/kboardPolicy/copyRightTermsInfo)
- [표준국어대사전 저작권 정책](https://stdict.korean.go.kr/join/copyrightPolicy.do)
- [우리말샘 저작권 정책](https://opendict.korean.go.kr/service/copyrightPolicy)
- [CC BY-SA 2.0 KR](https://creativecommons.org/licenses/by-sa/2.0/kr/)

국립국어원은 kfind를 보증하거나 후원하지 않습니다.
