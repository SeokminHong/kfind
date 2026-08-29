# kfind 문서 사이트

React Router Framework Mode로 문서, 벤치마크와 WebAssembly 플레이그라운드를
제공합니다. build는 모든 문서 route를 `build/client`에 prerender하며,
Cloudflare Pages가 각 clean URL을 정적 HTML로 제공합니다.

각 indexable route는 한국어 clean URL과 같은 path의 영어 `?hl=en` URL을
제공합니다. 두 URL은 locale별 본문, 고유 title·description·self-canonical,
상호 `hreflang`, Open Graph와 social card metadata를 가집니다. 홈의 `WebSite`와
나머지 route의 `BreadcrumbList`는 JSON-LD로 prerender합니다. Build는 이
metadata, 단일 `h1`, 두 locale의 sitemap과 route 집합, `404.html`의 `noindex`를
검사합니다.

한국어와 영어를 별도 정적 HTML로 prerender합니다. Pages Function은 `?hl=en`
요청을 영어 산출물로 연결하며 browser와 crawler를 구분하지 않습니다. 언어
control은 query를 바꾸고 선택한 언어를 `kfind-document-locale` cookie에도
저장합니다. Query가 없는 URL에서는 cookie가 hydration 뒤 사용자 언어를 복원합니다.
공통 interface와 metadata는 i18next catalog를 사용하고, 기술 문서 본문과 단어장은
locale별 content를 사용합니다.

품질 차트는 `docs/benchmarks/site-morphology.json`의 승인 snapshot을 D3로
렌더링합니다. raw와 contract-adjusted 값은 같은 scale에서 함께 표시합니다.
Canonical, query matrix와 Robust는 각각 kfind `embedded/full POS × any/smart`
4개 profile과 외부 분석기 고정 설정을 나열하고, 같은 fixture에서 측정한 초기화,
처리량, p95와 peak RSS를 함께 표시합니다. Snapshot에 contract review가 없는
평가군도 동일한 두 품질 값을 유지하고 reviewed case 수를 0으로 기록합니다.
형태 질의와 정규식 기준선도 full-POS any와 smart를 품질·batch 시간에서 별도 행으로
유지합니다.

플레이그라운드는 현재 `kfind-wasm` crate를 browser용으로 빌드합니다. 선택적 smart
component resource는 `kfind-assets` R2 bucket에 저장하며, 사용자가 요청한 경우에만
same-origin Pages Function을 통해 streaming합니다.

Publish workflow는 선택한 GitHub Release의 문서와 component resource를
`/versions/VERSION` base path로 다시 빌드합니다. 결정적 tar archive와 byte-range index는
R2의 `site/versions/VERSION`에 불변 객체로 저장하고, gzip archive와 index는 같은 GitHub
Release에도 첨부합니다. Pages Function은 공개 manifest에 있는 버전만 archive에서
range-read하여 응답합니다. Header의 버전 선택기는 현재 route, query와 fragment를 유지한 채
현재 문서와 게시된 버전 사이를 이동합니다.

```sh
pnpm --dir site install
pnpm --dir site run dev
pnpm --dir site run build
KFIND_SITE_BASE_PATH=/versions/1.0.0 pnpm --dir site run build:versioned
pnpm --dir site run dev:pages
```

배포 대상은 production branch가 `main`인 direct-upload Cloudflare Pages project
`kfind`입니다. 배포는 component resource를 다시 만들고 R2에 올린 뒤 정적 사이트와
Pages Function을 게시합니다.

`.github/workflows/pages.yml`은 `main` push와 수동 실행에서 배포합니다. 저장소에는
Pages와 R2 쓰기 권한을 가진 `CLOUDFLARE_ACCOUNT_ID`와
`CLOUDFLARE_API_TOKEN` secret이 필요합니다.

```sh
pnpm --dir site run deploy
```
