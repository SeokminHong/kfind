# @kfind/kfind

WebAssembly로 실행되는 JavaScript·TypeScript용 한국어 표제어 검색
패키지입니다.

```sh
npm install @kfind/kfind@1.0.0-rc.1
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
```

`yarn dlx`는 Yarn 2 이상에서 사용할 수 있습니다. 세 일회 실행 명령은 모두
패키지의 `kfind` bin을 실행합니다.

경로가 없으면 TTY에서는 현재 디렉터리를, pipe에서는 표준 입력을 검색합니다.
`--expand`, `--boundary`, `--pos`, `--normalization`, `--max-gap`,
`--literal`, `--json`을 사용할 수 있습니다. npm CLI는 UTF-8 파일을 재귀
순회하며 full POS가 필요한 검색은 native CLI를 사용해야 합니다.

패키지는 다음 asset을 export합니다.

- `@kfind/kfind/assets/predicates.enriched.tsv`
- `@kfind/kfind/assets/morphology-component-compact.kfc`

패키지는 asset의 filesystem 경로나 URL을 추정하거나 자동으로 내려받지 않습니다.
두 asset은 WASM binary에 포함되지 않습니다. 기본 constructor,
`Kfind.withFullPos`와 `loadComponentResource`도 사용할 수 있습니다.

각 패키지 버전의 component resource header에는 같은 버전이 들어 있습니다. 다른
버전의 asset을 읽으면 명시적인 오류가 발생합니다. npm `prepack`은 게시 전에
WASM과 리소스를 다시 만들고 검증합니다. package는 background update를 수행하지
않습니다.

코드와 배포 asset의 라이선스는 `LICENSES.md`에 정리되어 있습니다. 패키지는
browser bundler와 Node.js ESM 환경에서 사용합니다.
