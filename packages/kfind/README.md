# @kfind/kfind

WebAssembly로 실행되는 JavaScript·TypeScript용 한국어 표제어 검색
패키지입니다.

```sh
npm install @kfind/kfind@1.0.0-rc.3
```

```js
import { Kfind } from "@kfind/kfind";

const engine = new Kfind();
const matcher = engine.compile("걷다", { pos: "verb" });
const text = "길을 걸어 갔다.";
const matches = matcher.findAll(text);

console.log(text.slice(matches[0].start, matches[0].end));
```

여러 atom 중 하나를 찾으려면 `|`를 사용합니다.

```js
const matcher = engine.compile("v:걷다|n:사용자|n:검증");
```

각 alternative는 하나의 atom이어야 하며 공백 구와 한 query에서 섞을 수
없습니다. Literal `|`는 `engine.compile("\\|")` 또는
`engine.compile('"|"')`로 작성합니다.

`compile`은 `expand`, `boundary`, `pos`, `normalization`, `maxGap`,
`literal` 옵션을 받습니다. 허용 값과 match provenance 구조는 패키지의
TypeScript 선언에 포함됩니다.

Match offset은 UTF-16 code unit 기준이므로 `String.prototype.slice`에 직접
사용할 수 있습니다. CLI의 사전 profile이 필요하면
`Kfind.withResources({ fullPos?, enrichedPredicates?, component? })`에 resource
bytes를 전달합니다.

패키지는 Node.js 20 이상에서 `kfind` 실행 파일도 제공합니다.

```sh
npx @kfind/kfind 걷다 README.md
pnpm dlx @kfind/kfind 걷다 README.md
yarn dlx @kfind/kfind 걷다 README.md
npx @kfind/kfind --pos verb --json 걷다 src
npx @kfind/kfind 'v:걷다|n:사용자' src
```

`yarn dlx`는 Yarn 2 이상에서 사용할 수 있습니다. 세 일회 실행 명령은 모두
패키지의 `kfind` bin을 실행합니다.

경로가 없으면 TTY에서는 현재 디렉터리를, pipe에서는 표준 입력을 검색합니다.
`--expand`, `--boundary`, `--pos`, `--normalization`, `--max-gap`,
`--literal`, `--json`을 사용할 수 있습니다. npm CLI는 UTF-8 파일을 재귀
순회하며 full POS가 필요한 검색은 native CLI를 사용해야 합니다.

패키지는 다음 asset을 export합니다.

- `@kfind/kfind/assets`
- `@kfind/kfind/assets/predicates.enriched.tsv`
- `@kfind/kfind/assets/morphology-component-compact.kfc`

## Asset 자체 서빙

형태 구성 요소 판정 asset은 `smart` 검색이 원문 token 내부의 같은 품사
component span과 인접 token 구조를 검증하는 35.4 MiB compact index입니다. 전체 문장을
분석하거나 query를 확장하는 full POS 사전이 아닙니다. Enriched predicate TSV는
검증된 용언 alternation과 derivation을 query 분석에 추가합니다.

### Browser SPA

Vite처럼 `new URL(relative, import.meta.url)`을 처리하는 browser bundler는
resolver export를 import하면 같은 package 버전의 asset을 content hash가 붙은 정적
파일로 출력합니다. SPA는 bundler가 만든 same-origin URL을 fetch해 resource bytes를
초기화할 수 있습니다.

```js
import { Kfind } from "@kfind/kfind";
import { componentResourceFileUrl } from "@kfind/kfind/assets";

const response = await fetch(componentResourceFileUrl);
if (!response.ok) {
  throw new Error(`component resource: ${response.status}`);
}

const engine = Kfind.withResources({
  component: new Uint8Array(await response.arrayBuffer()),
});
```

### Node.js 서버

Node.js에서는 같은 export가 설치 package 내부의 `file:` URL을 반환합니다. 서버는
이 URL로 같은 버전의 asset을 직접 정적 서빙할 수 있습니다.

```js
import { createReadStream } from "node:fs";
import { createServer } from "node:http";
import { componentResourceFileUrl } from "@kfind/kfind/assets";

createServer((request, response) => {
  if (request.url !== "/morphology-component-compact.kfc") {
    response.writeHead(404).end();
    return;
  }

  response.writeHead(200, {
    "Cache-Control": "no-cache",
    "Content-Type": "application/octet-stream",
    "X-Content-Type-Options": "nosniff",
  });
  createReadStream(componentResourceFileUrl).pipe(response);
}).listen(3000);
```

### HTTP 배포 계약

- KFC는 `application/octet-stream`, TSV는
  `text/tab-separated-values; charset=utf-8`로 응답하고
  `X-Content-Type-Options: nosniff`를 설정합니다.
- Content hash 또는 package version이 URL에 포함된 asset만
  `public, max-age=31536000, immutable`로 캐시합니다. 고정 URL은 `no-cache` 또는
  짧은 수명과 revalidation을 사용합니다.
- 다른 origin에서 서빙할 때는 실제 application origin을
  `Access-Control-Allow-Origin`에 명시합니다.
- Package upgrade에서는 JavaScript, WASM과 resource를 함께 배포합니다. Component
  header의 package version이 engine과 다르면 초기화를 거부합니다.

Resolver 자체는 browser fetch나 서버 route를 정하지 않습니다. Browser에서는
애플리케이션이 bundler가 출력한 URL에서 bytes를 읽어 binding에 전달합니다. 두 asset은
WASM binary에 포함되지 않습니다. 기본 constructor, `Kfind.withFullPos`와
`loadComponentResource`도 사용할 수 있습니다.

각 패키지 버전의 component resource header에는 같은 버전이 들어 있습니다. 다른
버전의 asset을 읽으면 명시적인 오류가 발생합니다. npm `prepack`은 게시 전에
WASM과 리소스를 다시 만들고 검증합니다. package는 background update를 수행하지
않습니다.

코드와 배포 asset의 라이선스는 `LICENSES.md`에 정리되어 있습니다. 패키지는
browser bundler와 Node.js ESM 환경에서 사용합니다.
