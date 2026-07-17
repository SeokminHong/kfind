# kfind

[English](README.md) | [한국어](README.ko.md)

Korean lemma and inflection matching for JavaScript, powered by WebAssembly.

The package is not published to the npm registry yet. The install command below applies after the
first registry release.

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
`String.prototype.slice`. Build the same dictionary profile as the CLI with
`Kfind.withResources({ fullPos?, enrichedPredicates?, component? })`. The
package publishes `kfind/assets/predicates.enriched.tsv` and
`kfind/assets/morphology-component-compact.kfc` as separate static assets. It
never resolves or fetches them automatically, and neither is embedded in the
WASM binary. The existing constructor, `Kfind.withFullPos`, and
`loadComponentResource` remain available as compatibility APIs.

Each package version contains a component resource with the same version in its
header. Loading an asset from another version fails explicitly. npm `prepack`
rebuilds and tests the WASM and resources before publication; the package does
not perform background updates.

See `LICENSES.md` for the package code, component resource, and enriched
predicate data licenses and notice locations.

The package is an ESM module intended for browser bundlers.
