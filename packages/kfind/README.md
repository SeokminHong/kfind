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
`String.prototype.slice`. The package contains the embedded core lexicon. Load
an optional full POS binary with `Kfind.withFullPos(bytes)`.

The package is an ESM module intended for browser bundlers.
