# 보강 용언 데이터 고지

`predicates.tsv`와 `REPORT.tsv`는 국립국어원이 제공하는 한국어기초사전,
표준국어대사전과 우리말샘 snapshot에서 정규화한 용언 metadata입니다. 고정한
snapshot 파일명, checksum, 추출 필드와 record 수는 `MANIFEST.toml`과
`STATS.toml`에 기록합니다.

생성 계층에는 검토한 한국어 용언 교체, 동형이의 분석을 보존하는 데 필요한 최소
동일 품사 규칙형, 생산 규칙으로 만들 수 없는 사전 표면형이 들어 있습니다. 활용
표면형은 한국어기초사전과 표준국어대사전이 함께 지지하는 경우에만 사용합니다.
용언에서 부사로 파생된 표면형은 한국어기초사전의 entry ID가 양방향으로 일치해야
합니다.

데이터 파일은 저작자표시-동일조건변경허락 2.0 대한민국 라이선스(CC BY-SA 2.0
KR)로 배포합니다. 국립국어원을 표시하고 가공 데이터에도 같은 라이선스를
적용해야 합니다.

- [한국어기초사전 저작권 정책](https://krdict.korean.go.kr/kor/kboardPolicy/copyRightTermsInfo)
- [표준국어대사전 저작권 정책](https://stdict.korean.go.kr/join/copyrightPolicy.do)
- [우리말샘 저작권 정책](https://opendict.korean.go.kr/service/copyrightPolicy)
- [CC BY-SA 2.0 KR 전문](https://creativecommons.org/licenses/by-sa/2.0/kr/)

표제어, 품사, 활용형, 관련 형태 표면형과 source ID만 사용합니다. 사전 용례,
정의, 멀티미디어와 발음 자료는 재배포하지 않습니다.
