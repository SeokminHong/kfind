# kfind

[English](README.md) | [한국어](README.ko.md)

Korean lemma and inflection matching for JavaScript, powered by WebAssembly.

```sh
pnpm add kfind
```

```js
import { Kfind } from "kfind";

const engine = new Kfind();
const matcher = engine.compile("걷다");
const text = "길을 걸어 갔다.";
const matches = matcher.findAll(text);

console.log(text.slice(matches[0].start, matches[0].end)); // 걸어
```

`compile` accepts `expand`, `boundary`, `pos`, `normalization`, `maxGap`, and
`literal` options. Generated TypeScript declarations define their accepted
values and the complete match provenance shape.

Match offsets use UTF-16 code units, so they can be passed directly to
`String.prototype.slice`. Copy
`kfind/assets/morphology-component-compact.kfc` to your static assets or host it
separately. Applications using component-aware smart noun searches can pass its
bytes to the constructor or call `loadComponentResource` before compiling those
queries. Unlike the CLI, the package never resolves or fetches the asset
automatically. The WASM binary does not contain this data. Load an optional full POS
binary with `Kfind.withFullPos(fullPos, componentResource?)`.

The package is an ESM module intended for browser bundlers.
