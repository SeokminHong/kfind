# scoped npm 패키지와 Node.js CLI

- 일자: 2026-07-23
- 적용 범위: npm 배포, JavaScript export, npm CLI

## 결정

npm 패키지는 공개 organization scope인 `@kfind/kfind`로 배포한다. 패키지의
`kfind` bin은 Node.js 20 이상에서 실행되는 파일·표준 입력 검색 CLI다.

게시 산출물은 browser bundler용 ESM WASM target과 Node.js용 CommonJS WASM
target을 함께 포함한다. Conditional export는 실행 환경에 맞는 target을 선택한다.
두 target은 같은 Rust source에서 생성하고 같은 JavaScript API를 제공한다.

## 책임 경계

JavaScript binding API는 filesystem이나 package asset 경로를 추정하지 않고 resource
bytes만 받는다. 별도 `@kfind/kfind/assets` export는 Node.js 서버에 설치 package와 같은
버전의 enriched predicate와 compact component resource `file:` URL을 제공한다. 이
resolver는 browser fetch나 서버 URL을 정하지 않는다. npm CLI는 package 내부 resource를
직접 읽고 query가 구조 판정을 요구할 때 compact resource를 초기화한다.

npm 패키지는 full POS resource를 포함하지 않는다. Full POS, Git ignore 규칙,
EUC-KR과 TUI가 필요한 작업은 native CLI의 책임이다.

## 검증 계약

`prepack`은 browser와 Node.js target을 모두 만들고 공개 API, resource digest,
실행 파일 mode, 파일·표준 입력 검색, JSON Lines와 종료 코드를 검사한다. `pack:check`는
실제 tarball을 임시 소비자 project에 설치하고 resolver의 component `file:` URL로 전체
asset을 HTTP streaming한다. Prerelease는 `next`, stable release는 `latest` dist-tag를
사용한다.
