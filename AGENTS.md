# kfind 저장소 작업 지침

## PR 언어

- PR 제목과 본문은 한국어를 기본으로 작성한다. Conventional Commit의 type/scope,
  코드 식별자, 명령 등은 원문 표기를 허용한다.

## 브랜치와 PR

- 별도 지시가 없으면 최신 `origin/main`을 기준으로 작업한다.
- 지시받은 작업이 `main`에 머지 가능한 상태라고 판단되면 PR을 생성하고 머지까지
  완료한다.
- PR 머지를 기다리는 동안 `main`에 추가 push가 발생하면 최신 upstream을 머지하거나
  리베이스하고, 변경 영향과 추가 작업 필요 여부를 다시 검토한다.

## 문서 정합성

- 코드 변경을 끝낼 때마다 기술 사양서, README, 웹 문서, CLI 도움말·man page와
  benchmark 문서에 갱신할 내용이 있는지 다시 검토한다.
- 사용자 동작, 옵션, 공개 API, 오류, 성능 계약이 바뀌면 관련 문서를 같은 작업에서
  갱신한다. 외부 계약이 바뀌지 않아 문서를 수정하지 않는 경우에도 그 판단을 자기
  리뷰에서 확인한다.
- README는 현재 제품을 이해하고 사용하는 데 필요한 정보만 담는다. 측정일·Git revision,
  baseline/candidate 비교, PR·브랜치·머지 상태, 날짜별 작업·보고서 목록과 완료 이력은
  README에 기록하지 않는다. 재현용 측정 정보와 변경 이력은 benchmark 보고서와 PR에 둔다.

## 성능 검증

- 문서만 변경한 PR은 성능 측정을 생략할 수 있다. PR 본문에 생략 사유를 짧게
  적는다.
- 코드가 변경되면 변경 경로와 영향에 맞는 성능 측정을 수행한다. 관련 benchmark가
  없으면 변경에 비례한 재현 가능한 benchmark를 추가하거나, 측정 불가 사유와 후속
  계획을 PR에 적는다.
- [기술 사양서](specs/kfind.md)와
  [benchmark contract](docs/benchmarks/README.md)를 우선한다. 후보 브랜치와 최신
  `origin/main` 기준선을 같은 환경, 빌드 설정, 입력으로 비교하고 관련 없는 workload나
  metric family를 섞지 않는다.
- 측정 방법에는 정확한 명령, 양쪽 Git revision, 환경과 도구 버전, 입력과 checksum,
  warm-up 횟수, 측정 횟수, 대표값 산출법을 남긴다. 일회 실행이나 build/smoke
  성공만으로 성능 불변을 주장하지 않는다.
- morphology benchmark는 fresh process에서 warm-up 1회 후 5회 측정한다.
  initialization, cases/s, p95 latency, RSS의 median/min/max를 기록한다. CLI 변경은
  해당 CLI workload benchmark를 별도로 사용한다.
- PR 본문에는 baseline/candidate 결과, 증감률과 회귀 여부를 적는다. 품질 지표와
  성능 지표를 분리하고 불리한 결과도 누락하지 않는다.
- 성능 문서화가 필요한 변경은 `docs/benchmarks`의 기존 구조에 맞춰 날짜별 보고서, 생성
  차트와 snapshot을 갱신한다. 생성물의 수치가 원본 report와 일치하는지 검증한다.
- 승인된 benchmark 보고서나 생성 차트가 추가·변경돼도 측정 수치와 이력을 README로
  복사하지 않는다. 사용자에게 설명할 현재 기능·제약이 달라졌을 때만 README의 동작 설명을
  갱신한다.
