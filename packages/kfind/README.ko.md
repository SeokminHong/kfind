# kfind

[English](README.md) | [한국어](README.ko.md)

WebAssembly로 실행되는 JavaScript용 한국어 표제어·활용형 matcher입니다.

이 패키지는 아직 npm registry에 게시되지 않았습니다. 아래 설치 명령은 첫 registry 릴리스
이후에 사용할 수 있습니다.

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

Match offset은 UTF-16 code unit 기준이므로 `String.prototype.slice`에 바로 사용할 수 있습니다.
`Kfind.withResources({ fullPos?, enrichedPredicates?, component? })`로 CLI와 같은 사전 profile을
구성할 수 있습니다. Package는 `kfind/assets/predicates.enriched.tsv`와
`kfind/assets/morphology-component-compact.kfc`를 별도 정적 asset으로 배포합니다. CLI와 달리
asset을 자동으로 찾거나 fetch하지 않으며 WASM binary에도 포함하지 않습니다. 기존 생성자와
`Kfind.withFullPos`, `loadComponentResource`도 호환 API로 유지합니다.

각 package version은 header version이 같은 component resource를 포함합니다. 다른 version의
asset을 로드하면 명시적으로 실패합니다. npm `prepack`은 게시 전에 WASM과 resource를 다시 만들고
검증하며 package가 백그라운드 업데이트를 수행하지는 않습니다.

Package code, component resource, enriched predicate data의 라이선스와 고지 위치는
`LICENSES.md`에 정리되어 있습니다.

이 패키지는 브라우저 bundler용 ESM module입니다.
