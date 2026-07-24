import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const guideDocuments: TechnicalDocuments = {
  [RoutePath.Installation]: {
    [DocumentLocale.Korean]: {
      eyebrow: '시작 · 배포',
      title: '설치',
      sections: [
        section('배포 profile', [
          'Homebrew 배포물은 native CLI, full POS, enriched predicate와 compact component resource를 함께 설치합니다. Cargo 설치는 source에서 native CLI를 만들며 배포 resource 경로를 사용자가 준비합니다.',
          'npm 패키지 `@kfind/kfind`는 browser API와 Node.js CLI를 제공합니다. Enriched predicate와 compact component는 포함하지만 full POS는 포함하지 않습니다.',
        ]),
        section(
          'native 설치',
          [
            'macOS의 기본 배포 경로는 개인 tap의 Homebrew formula입니다. 빌드 도구가 이미 있는 환경에서는 Cargo로 같은 version의 CLI를 설치할 수 있습니다.',
          ],
          {
            code: `brew install seokminhong/brew/kfind
kfind --version

cargo install --locked --path crates/kfind-cli`,
          },
        ),
        section(
          'npm 설치',
          [
            'Node.js 20 이상에서는 scoped package를 설치합니다. Package의 `bin` 이름은 `kfind`이므로 local script, `npx`, `pnpm dlx`, Yarn 2 이상의 `yarn dlx`와 `node_modules/.bin`에서 같은 명령을 사용합니다.',
            'Prerelease channel을 추적하려면 `@next`를, 재현 가능한 설치에는 정확한 version을 사용합니다.',
          ],
          {
            code: `npm install @kfind/kfind@1.0.0-rc.3
npx @kfind/kfind 걷다 README.md
pnpm dlx @kfind/kfind 걷다 README.md
yarn dlx @kfind/kfind 걷다 README.md`,
          },
        ),
        section('설치 확인', [
          '`kfind --version`은 실행 파일 version을 출력합니다. `kfind --literal 한국어 README.md`가 path, line, column과 surface를 출력하면 기본 파일 입력과 matcher가 동작합니다.',
          'Full POS 자동 판정, EUC-KR, Git ignore, TUI가 필요하면 native CLI를 사용합니다. npm CLI는 UTF-8 파일과 표준 입력을 대상으로 하는 portable profile입니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'GET STARTED · DISTRIBUTION',
      title: 'Installation',
      sections: [
        section('Distribution profiles', [
          'The Homebrew distribution installs the native CLI together with full-POS, enriched-predicate, and compact-component resources. A Cargo installation builds the native CLI from source and leaves resource placement to the caller.',
          'The `@kfind/kfind` package exposes the browser API and a Node.js CLI. It includes enriched predicates and the compact component resource, but not the full-POS resource.',
        ]),
        section(
          'Native installation',
          [
            'The personal Homebrew tap is the standard macOS distribution. Environments with the build toolchain can install the same CLI through Cargo.',
          ],
          {
            code: `brew install seokminhong/brew/kfind
kfind --version

cargo install --locked --path crates/kfind-cli`,
          },
        ),
        section(
          'npm installation',
          [
            'Install the scoped package on Node.js 20 or later. Its bin name is `kfind`, so local scripts, `npx`, `pnpm dlx`, Yarn 2+ `yarn dlx`, and `node_modules/.bin` use the same command.',
            'Use the `next` channel to follow prereleases and an exact version for reproducible installation.',
          ],
          {
            code: `npm install @kfind/kfind@1.0.0-rc.3
npx @kfind/kfind 걷다 README.md
pnpm dlx @kfind/kfind 걷다 README.md
yarn dlx @kfind/kfind 걷다 README.md`,
          },
        ),
        section('Installation check', [
          '`kfind --version` prints the executable version. `kfind --literal 한국어 README.md` verifies basic file input and matching by emitting a path, line, column, and surface.',
          'Use the native CLI when full-POS auto-detection, EUC-KR, Git ignore rules, or the TUI is required. The npm CLI is the portable profile for UTF-8 files and standard input.',
        ]),
      ],
    },
  },
  [RoutePath.Workflows]: {
    [DocumentLocale.Korean]: {
      eyebrow: '시작 · 절차',
      title: '검색 절차',
      sections: [
        section('탐색 검색', [
          '첫 검색은 표제어와 기본 `inflection`, `smart` 경계를 사용합니다. 결과가 없으면 `--boundary any`로 구조 경계를 풀고, 품사 중의성이 예상되면 `--pos`를 명시합니다.',
          '탐색 단계에서는 파일 확장자보다 directory 범위를 먼저 제한합니다. 결과 surface와 주변 문맥을 확인한 뒤 query를 바꿉니다.',
        ]),
        section('정밀 검색', [
          '정밀 검색은 atom 품사 태그, 구 순서와 `--max-gap`을 고정합니다. Literal 문자열이 목적이면 형태 생성을 끄는 `--literal`을 사용합니다.',
          '구조로 구분할 수 없는 의미 중의성은 검색기가 제거하지 않습니다. 해당 후보는 주변 code나 문장 문맥에서 판정합니다.',
        ]),
        section(
          '자동화',
          [
            '에이전트와 script는 `--json` 또는 native CLI의 JSON Lines 출력을 사용합니다. Match가 없는 정상 결과와 실행 오류를 종료 코드로 구분하고 stdout만 parser에 전달합니다.',
          ],
          {
            code: `kfind --pos verb --boundary any --json 걷다 src \
  | jq -c 'select(.surface != null)'`,
          },
        ),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'GET STARTED · WORKFLOW',
      title: 'Search workflows',
      sections: [
        section('Exploration', [
          'Start with a lemma and the default `inflection` and `smart` settings. If no result appears, relax structural boundaries with `--boundary any`; specify `--pos` when the lemma is POS-ambiguous.',
          'Constrain directories before filtering extensions. Inspect each surface and its nearby context before changing the query.',
        ]),
        section('Precision search', [
          'Fix atom-level POS tags, phrase order, and `--max-gap` for a precise search. Use `--literal` when only the exact input string is relevant.',
          'The matcher does not remove semantic ambiguity that morphology cannot distinguish. Resolve those candidates from surrounding code or sentence context.',
        ]),
        section(
          'Automation',
          [
            'Agents and scripts use `--json` or the native JSON Lines output. Distinguish a normal no-match result from execution failure by exit status, and feed only stdout into the parser.',
          ],
          {
            code: `kfind --pos verb --boundary any --json 걷다 src \
  | jq -c 'select(.surface != null)'`,
          },
        ),
      ],
    },
  },
  [RoutePath.Goals]: {
    [DocumentLocale.Korean]: {
      eyebrow: '시작 · 제품 범위',
      title: '목표와 비목표',
      sections: [
        section('제품 목표', [
          '사용자가 아는 표제어와 짧은 구를 조사, 어미, 불규칙 활용과 제한된 파생을 포함하는 검색 후보로 컴파일합니다. Source에서는 anchor가 있는 국소 범위만 검증합니다.',
          '출력은 원문 span과 규칙 provenance입니다. 사람과 에이전트가 후속 문맥 판정을 수행할 수 있도록 surface 근거를 보존합니다.',
        ]),
        section('비목표', [
          '문장 전체의 형태소열, 문장 성분, 의존 관계와 의미역을 만들지 않습니다. Corpus에 나타난 모든 token의 표제어를 역분석하는 색인기도 아닙니다.',
          '사전에 없는 임의의 파생, 무제한 조사·어미 연쇄와 의미 기반 동음이의어 제거는 지원하지 않습니다.',
        ]),
        section('선택 기준', [
          '표제어에서 도달 가능한 활용형을 대규모 code와 문서에서 찾을 때 kfind를 사용합니다. 문장 전체 annotation이나 언어학적 분석 결과가 필요하면 범용 형태소 분석기를 사용합니다.',
          '두 도구를 결합할 때는 kfind로 파일과 span 후보를 줄인 뒤 선택한 문맥만 분석기에 전달합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'GET STARTED · PRODUCT SCOPE',
      title: 'Goals and non-goals',
      sections: [
        section('Product goals', [
          'Compile a known lemma or short phrase into candidates covering particles, endings, irregular inflection, and bounded derivation. Verify only local source regions containing an anchor.',
          'Return source spans and rule provenance so a person or agent can perform the next context decision without losing surface evidence.',
        ]),
        section('Non-goals', [
          'kfind does not produce a whole-sentence morpheme sequence, syntactic dependencies, or semantic roles. It is also not an index that reverse-analyzes every observed token.',
          'Arbitrary derivation, unbounded particle or ending chains, and semantic homonym removal are not supported.',
        ]),
        section('Selection guide', [
          'Use kfind to locate inflected surfaces reachable from a lemma across large code and documentation sets. Use a general morphological analyzer when whole-sentence annotation is the desired output.',
          'When combining both tools, first narrow files and spans with kfind, then analyze only the selected context.',
        ]),
      ],
    },
  },
};
