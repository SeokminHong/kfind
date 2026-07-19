# kfind 문서 사이트

React Router Framework Mode로 문서, 벤치마크와 WebAssembly 플레이그라운드를
제공합니다. build는 모든 문서 route를 `build/client`에 prerender하며,
Cloudflare Pages가 각 clean URL을 정적 HTML로 제공합니다.

한국어가 prerender 기본 언어입니다. 영어 번역은 같은 URL에서 선택하며, 선택한
언어는 hydration 뒤 `kfind-document-locale` cookie에 저장합니다. 공통 interface와
metadata는 i18next catalog를 사용하고, 기술 문서 본문과 단어장은 locale별
content를 사용합니다.

품질 차트는 `docs/benchmarks/site-morphology.json`의 승인 snapshot을 D3로
렌더링합니다. raw와 contract-adjusted 값은 같은 scale에서 함께 표시합니다.
snapshot에 contract review가 없는 평가군도 동일한 두 값을 유지하고 reviewed
case 수를 0으로 기록합니다.

플레이그라운드는 현재 `kfind-wasm` crate를 browser용으로 빌드합니다. 선택적 smart
component resource는 `kfind-assets` R2 bucket에 저장하며, 사용자가 요청한 경우에만
same-origin Pages Function을 통해 streaming합니다.

```sh
pnpm --dir site install
pnpm --dir site run dev
pnpm --dir site run build
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
