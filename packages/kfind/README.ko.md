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
있습니다. `kfind/assets/morphology-component-compact.kfc`를 정적 asset으로 복사하거나
별도 호스트에 올릴 수 있습니다. Component-aware smart 명사 검색을 사용할 때만 생성자에
bytes를 전달하거나 query compile 전에 `loadComponentResource`를 호출합니다. CLI와 달리
package는 asset을 자동으로 찾거나 fetch하지 않습니다. WASM binary에는 이 데이터가 포함되지
않습니다. 선택적인 full POS binary는
`Kfind.withFullPos(fullPos, componentResource?)`로 로드할 수 있습니다.

이 패키지는 브라우저 bundler용 ESM module입니다.
