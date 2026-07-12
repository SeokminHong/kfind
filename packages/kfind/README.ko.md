# kfind

[English](README.md) | [한국어](README.ko.md)

WebAssembly로 실행되는 JavaScript용 한국어 표제어·활용형 matcher입니다.

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

`compile`은 `expand`, `boundary`, `pos`, `normalization`, `maxGap`, `literal`
옵션을 받습니다. 생성된 TypeScript 선언에서 허용 값과 match provenance 전체 구조를
확인할 수 있습니다.

Match offset은 UTF-16 code unit 기준이므로 `String.prototype.slice`에 바로 사용할 수
있습니다. 패키지에는 core lexicon이 포함됩니다. 선택적인 full POS binary는
`Kfind.withFullPos(bytes)`로 로드할 수 있습니다.

이 패키지는 브라우저 bundler용 ESM module입니다.
