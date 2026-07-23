import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const referenceDocuments: TechnicalDocuments = {
  [RoutePath.ReferenceCli]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · CLI',
      title: 'CLI 참조',
      summary:
        'Native CLI는 전체 파일 검색 surface를, npm CLI는 Node.js에서 바로 실행하는 UTF-8 profile을 제공합니다.',
      sections: [
        section(
          'native CLI',
          [
            '구문은 `kfind [OPTIONS] QUERY [PATH]...`입니다. 주요 compile option은 `--pos`, `--expand`, `--boundary`, `--literal`, `--max-gap`, `--unicode-normalization`입니다.',
            '파일 option은 encoding, glob, type, ignore, thread와 context 출력을 제어합니다. `--json`, `--explain-query`, `--explain-match`와 `--sort path`를 지원합니다.',
          ],
          {
            code: `kfind --pos verb --boundary smart 걷다 src
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src`,
          },
        ),
        section(
          'npm CLI',
          [
            'Node.js 20 이상에서 `@kfind/kfind`의 bin 이름 `kfind`를 사용합니다. `--expand`, `--boundary`, `--pos`, `--normalization`, `--max-gap`, `--literal`, `--json`을 지원합니다.',
            '`npx`, `pnpm dlx`, Yarn 2 이상의 `yarn dlx`는 설치 없이 같은 bin을 실행합니다.',
          ],
          {
            code: `npx @kfind/kfind 걷다 README.md
pnpm dlx @kfind/kfind --pos verb 걷다 src
yarn dlx @kfind/kfind --json 권한 docs
npx @kfind/kfind 'v:걷다|n:사용자' src`,
          },
        ),
        section('기능 차이', [
          'npm CLI는 UTF-8 재귀 순회, stdin, text와 JSON Lines에 집중합니다. Package의 enriched predicate를 사용하고 필요한 query에서 compact component를 lazy load합니다.',
          'Full POS, Git ignore, EUC-KR, context, TUI, explain과 agent 통합 초기화는 native CLI를 사용합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · CLI',
      title: 'CLI reference',
      summary:
        'The native CLI exposes the complete file-search surface; the npm CLI is a directly executable UTF-8 Node.js profile.',
      sections: [
        section(
          'Native CLI',
          [
            'Syntax is `kfind [OPTIONS] QUERY [PATH]...`. Core compile options are `--pos`, `--expand`, `--boundary`, `--literal`, `--max-gap`, and `--unicode-normalization`.',
            'File options control encoding, globs, types, ignore rules, threads, and context. Output supports `--json`, `--explain-query`, `--explain-match`, and `--sort path`.',
          ],
          {
            code: `kfind --pos verb --boundary smart 걷다 src
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src`,
          },
        ),
        section(
          'npm CLI',
          [
            'On Node.js 20 or later, `@kfind/kfind` installs the bin named `kfind`. It supports `--expand`, `--boundary`, `--pos`, `--normalization`, `--max-gap`, `--literal`, and `--json`.',
            '`npx`, `pnpm dlx`, and Yarn 2+ `yarn dlx` run the same bin without a local installation.',
          ],
          {
            code: `npx @kfind/kfind 걷다 README.md
pnpm dlx @kfind/kfind --pos verb 걷다 src
yarn dlx @kfind/kfind --json 권한 docs
npx @kfind/kfind 'v:걷다|n:사용자' src`,
          },
        ),
        section('Feature differences', [
          'The npm CLI focuses on recursive UTF-8 traversal, stdin, text, and JSON Lines. It loads packaged enriched predicates and lazily loads compact components for queries that need them.',
          'Use the native CLI for full POS, Git ignore, EUC-KR, context, TUI, explain output, and agent integration initialization.',
        ]),
      ],
    },
  },
  [RoutePath.QueryLanguage]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 질의',
      title: 'query 언어',
      summary:
        'Query parser는 atom, phrase, disjunction, 품사 prefix, 따옴표와 escape를 결정적으로 해석합니다.',
      sections: [
        section(
          '문법',
          [
            'Query는 단일 atom, 공백으로 연결한 phrase 또는 `|`로 연결한 disjunction입니다. Quoted atom 안의 공백과 `|`는 문자열 일부입니다.',
            'Unicode normalization과 형태 expansion은 lexer 이후 compile option으로 적용합니다.',
          ],
          {
            code: `query       = atom / phrase / disjunction
phrase      = atom 1*(WS atom)
disjunction = atom 1*(OWS "|" OWS atom)`,
          },
        ),
        section('atom 태그', [
          '`n`, `pro`, `num`, `v`, `adj`, `det`, `adv`, `j`, `intj`, `lit` 뒤의 colon이 품사 태그입니다. 태그는 해당 atom에만 적용됩니다.',
          '전역 POS와 구체 atom 태그가 다르면 compile 오류입니다.',
        ]),
        section('대안', [
          '`|` 앞뒤 공백은 선택 사항이고 각 alternative는 정확히 하나의 atom입니다. Phrase와 disjunction은 한 query에서 섞지 않습니다.',
          'Literal `|`는 `\\|` 또는 `"|"`로 작성합니다. CLI shell에서는 query 전체를 따옴표로 묶습니다.',
        ]),
        section('구문 오류', [
          '빈 query, 닫히지 않은 quote, 마지막 backslash, 알 수 없는 tag, 빈 tag body, 피연산자 없는 `|`와 phrase 혼합을 거부합니다.',
          '오류는 byte 위치와 원인을 포함하며 부분 plan을 실행하지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · QUERY',
      title: 'Query language',
      summary:
        'The parser deterministically interprets atoms, phrases, disjunctions, POS prefixes, quotes, and escapes.',
      sections: [
        section(
          'Syntax',
          [
            'A query is one atom, a whitespace-connected phrase, or a `|`-connected disjunction. Whitespace and `|` inside a quoted atom belong to its text.',
            'Unicode normalization and morphology expansion apply as compile options after lexing.',
          ],
          {
            code: `query       = atom / phrase / disjunction
phrase      = atom 1*(WS atom)
disjunction = atom 1*(OWS "|" OWS atom)`,
          },
        ),
        section('Atom tags', [
          'A colon after `n`, `pro`, `num`, `v`, `adj`, `det`, `adv`, `j`, `intj`, or `lit` marks POS. It applies only to that atom.',
          'A concrete global POS conflicting with an atom tag is a compile error.',
        ]),
        section('Alternatives', [
          'Whitespace around `|` is optional, and every alternative is exactly one atom. A query cannot mix phrase and disjunction composition.',
          'Write a literal `|` as `\\|` or `"|"`. Quote the whole query in a CLI shell.',
        ]),
        section('Syntax errors', [
          'Empty queries, unclosed quotes, trailing backslashes, unknown tags, empty tagged bodies, operand-free `|`, and phrase mixing are rejected.',
          'Errors include byte location and cause, and no partial plan executes.',
        ]),
      ],
    },
  },
  [RoutePath.PosTags]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 문법',
      title: '품사 태그',
      summary:
        '공개 태그는 사용자 입력을 위한 coarse POS이며 세부 resource 태그로 확장됩니다.',
      sections: [
        section('coarse POS', [
          '`noun`, `pronoun`, `numeral`, `verb`, `adjective`, `determiner`, `adverb`, `particle`, `interjection`, `literal`과 `auto`를 제공합니다.',
          'CLI prefix는 각각 `n:`, `pro:`, `num:`, `v:`, `adj:`, `det:`, `adv:`, `j:`, `intj:`, `lit:`입니다.',
        ]),
        section('세부 태그', [
          '명사 NNG·NNP·NNB·NNBC, 대명사 NP, 수사 NR, 용언 VV·VA·VX·VCP·VCN, 조사 J*, 어미 E*와 파생 X*를 사용합니다.',
          '세부 태그는 공개 option 값이 아니라 lexicon과 component resource의 분석 정보입니다.',
        ]),
        section('포함 관계', [
          'Coarse POS는 여러 세부 태그를 포함하지만 무관한 기능 tag를 흡수하지 않습니다. `noun` query가 조사 JX를 core로 만들지 않습니다.',
          'Full POS fallback도 query coarse POS 안의 세부 후보만 보완합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · GRAMMAR',
      title: 'POS tags',
      summary:
        'Public tags are coarse user-input categories expanded into detailed resource tags.',
      sections: [
        section('Coarse POS', [
          'Values are `noun`, `pronoun`, `numeral`, `verb`, `adjective`, `determiner`, `adverb`, `particle`, `interjection`, `literal`, and `auto`.',
          'CLI prefixes are `n:`, `pro:`, `num:`, `v:`, `adj:`, `det:`, `adv:`, `j:`, `intj:`, and `lit:`.',
        ]),
        section('Detailed tags', [
          'Detailed tags include NNG, NNP, NNB, NNBC, NP, NR, VV, VA, VX, VCP, VCN, J*, E*, and X* derivation tags.',
          'They are analysis data in lexicons and component resources, not public option values.',
        ]),
        section('Mapping', [
          'A coarse POS contains several detailed tags but never absorbs unrelated functional tags. A `noun` query does not use JX as its core.',
          'Full-POS fallback adds only detailed candidates inside the query coarse category.',
        ]),
      ],
    },
  },
  [RoutePath.Configuration]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 환경',
      title: '설정',
      summary:
        'CLI option, 환경 변수와 기본 경로는 명시성이 높은 순서로 resource와 사용자 사전을 결정합니다.',
      sections: [
        section('설정 파일', [
          '사용자 사전 기본 경로는 `$XDG_CONFIG_HOME/kfind/lexicon.toml`, 다음으로 `$HOME/.config/kfind/lexicon.toml`입니다.',
          '검색 option 전체를 저장하는 일반 config 파일은 없습니다. 재현 가능한 명령은 shell script나 agent skill에 option을 명시합니다.',
        ]),
        section('환경 변수', [
          '`KFIND_DATA_DIR`은 full POS와 component resource directory를, `KFIND_USER_LEXICON`은 사용자 사전 경로를 지정합니다.',
          '`LC_ALL`, `LC_MESSAGES`, `LANG`은 사람이 읽는 진단 언어만 선택하며 option과 JSON field를 바꾸지 않습니다.',
        ]),
        section('우선순위', [
          '명시적 `--data-dir`, `--user-lexicon`이 환경 변수보다 우선합니다. 환경 변수 다음에는 설치 경로와 기본 config 경로를 사용합니다.',
          '명시한 path가 잘못되면 다음 후보로 fallback하지 않고 오류를 반환합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · ENVIRONMENT',
      title: 'Configuration',
      summary:
        'CLI options, environment, and defaults resolve resources and user lexicons from most to least explicit.',
      sections: [
        section('Configuration files', [
          'The default user lexicon is `$XDG_CONFIG_HOME/kfind/lexicon.toml`, then `$HOME/.config/kfind/lexicon.toml`.',
          'There is no general file storing all search options. Reproducible scripts and agent skills spell options out.',
        ]),
        section('Environment variables', [
          '`KFIND_DATA_DIR` selects the full-POS and component directory; `KFIND_USER_LEXICON` selects a user lexicon.',
          '`LC_ALL`, `LC_MESSAGES`, and `LANG` affect only human-readable diagnostics, never options or JSON fields.',
        ]),
        section('Precedence', [
          'Explicit `--data-dir` and `--user-lexicon` override environment. Installation and default config paths follow.',
          'An invalid explicit path raises an error instead of falling through to another candidate.',
        ]),
      ],
    },
  },
  [RoutePath.UserLexicon]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 데이터',
      title: '사용자 사전',
      summary:
        'TOML 사용자 사전은 프로젝트 표제어 분석을 내장·full POS 사전에 추가하거나 명시적으로 교체합니다.',
      sections: [
        section(
          '파일 형식',
          [
            '`[[predicate]]`는 lemma, coarse `pos`, alternation과 선택적 flag를, `[[nominal]]`은 surface와 명사 속성을 선언합니다.',
            '파일은 UTF-8 TOML이며 최대 16 MiB입니다. 알 수 없는 field, 잘못된 POS와 alternation을 초기화 전에 거부합니다.',
          ],
          {
            code: `[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"`,
          },
        ),
        section('entry 의미', [
          '기본은 기존 분석에 append합니다. 같은 lemma라도 alternation이나 세부 분석이 다르면 모두 보존합니다.',
          '`replace = true`는 해당 사용자 entry가 대응하는 내장·full POS 분석보다 우선합니다. 다른 lemma와 무관한 분석은 건드리지 않습니다.',
        ]),
        section('검증', [
          '표제어 NFC, 기본형, POS별 필수 field, 중복과 override 충돌을 검사합니다. 하나라도 실패하면 전체 파일을 설치하지 않습니다.',
          '`--explain-query`의 analysis source가 `user-lexicon`이면 적용된 entry를 확인할 수 있습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · DATA',
      title: 'User lexicon',
      summary:
        'A TOML user lexicon appends project analyses to embedded and full-POS lexicons or explicitly replaces them.',
      sections: [
        section(
          'File format',
          [
            '`[[predicate]]` declares lemma, coarse `pos`, alternation, and optional flags; `[[nominal]]` declares a surface and nominal properties.',
            'Files are UTF-8 TOML up to 16 MiB. Unknown fields and invalid POS or alternations fail before installation.',
          ],
          {
            code: `[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"`,
          },
        ),
        section('Entry semantics', [
          'Entries append by default. Analyses for the same lemma remain distinct when alternation or detail differs.',
          '`replace = true` gives that user entry priority over corresponding embedded and full-POS analyses without affecting unrelated lemmas.',
        ]),
        section('Validation', [
          'Validation covers NFC, dictionary form, POS-required fields, duplicates, and override conflicts. One failure prevents installing the whole file.',
          'An analysis source of `user-lexicon` in `--explain-query` confirms application.',
        ]),
      ],
    },
  },
  [RoutePath.Jsonl]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 출력',
      title: 'JSON Lines',
      summary:
        '각 줄은 source와 match provenance를 독립적으로 해석할 수 있는 UTF-8 JSON object입니다.',
      sections: [
        section('record', [
          'Native record는 `type`, `path`, `line`, `text`, `spans`를 포함합니다. npm record는 `path`, `line`, `column`, `start`, `end`, `surface`, `atoms`를 포함합니다.',
          '각 object 뒤에 LF가 오며 diagnostic은 stderr에만 기록합니다.',
        ]),
        section('span', [
          'Native UTF-8 text offset은 `utf8-bytes`, raw bytes는 `bytes` encoding을 표시합니다. npm offset과 column은 UTF-16 code unit입니다.',
          'Core는 query 표제어에 대응하는 범위, token은 조사·어미 소비까지 포함한 범위입니다.',
        ]),
        section('provenance', [
          'Origin은 lemma, POS와 ordered `rules` 또는 `analysisIndex`, `rulePath`를 보존합니다. 하나의 atom에 여러 origin이 있을 수 있습니다.',
          'Consumer는 모르는 field를 무시할 수 있지만 좌표 encoding을 확인하지 않고 slice하면 안 됩니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · OUTPUT',
      title: 'JSON Lines',
      summary:
        'Each line is a UTF-8 JSON object independently carrying source and match provenance.',
      sections: [
        section('Record', [
          'Native records contain `type`, `path`, `line`, `text`, and `spans`. npm records contain `path`, `line`, `column`, `start`, `end`, `surface`, and `atoms`.',
          'Every object ends with LF; diagnostics remain on stderr.',
        ]),
        section('Spans', [
          'Native UTF-8 text uses `utf8-bytes`; raw byte text uses `bytes`. npm offsets and columns use UTF-16 code units.',
          'Core covers the query lemma; token may extend through consumed particles and endings.',
        ]),
        section('Provenance', [
          'An origin preserves lemma, POS, and ordered `rules`, or `analysisIndex` and `rulePath`. One atom may have several origins.',
          'Consumers may ignore unknown fields but must inspect coordinate encoding before slicing source.',
        ]),
      ],
    },
  },
  [RoutePath.ExitCodes]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 실행',
      title: '종료 코드',
      summary:
        '검색 결과 유무와 실행 실패는 shell automation에서 구분 가능한 안정 상태입니다.',
      sections: [
        section('native CLI', [
          '0은 하나 이상의 match, 1은 정상 no-match, 2는 사용법·I/O·resource·compile 오류입니다.',
          'Broken pipe는 consumer가 출력을 닫은 정상 상황으로 처리합니다.',
        ]),
        section('npm CLI', [
          '0, 1, 2의 의미는 native 검색과 같습니다. Invalid UTF-8·binary 파일은 진단 뒤 skip하지만 explicit path read 실패는 2입니다.',
          '`--help`와 `--version`은 성공 0입니다.',
        ]),
        section('pipeline 사용', [
          '`set -e`에서는 no-match 1이 script를 중단할 수 있습니다. 허용하려면 상태를 명시적으로 분기하고 2를 성공으로 바꾸지 않습니다.',
          '부분 stdout 뒤 2로 끝나면 결과를 폐기하거나 incomplete로 표시합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · EXECUTION',
      title: 'Exit codes',
      summary:
        'Match presence and execution failure are stable, distinguishable states for shell automation.',
      sections: [
        section('Native CLI', [
          '0 means one or more matches, 1 is a normal no-match result, and 2 covers usage, I/O, resource, and compile errors.',
          'A broken pipe is a normal consumer-closed output condition.',
        ]),
        section('npm CLI', [
          'The npm search uses the same meanings for 0, 1, and 2. Invalid UTF-8 and binary files are reported then skipped; an explicit path read failure is 2.',
          '`--help` and `--version` succeed with 0.',
        ]),
        section('Pipeline use', [
          'Under `set -e`, no-match status 1 can stop a script. Branch explicitly when it is allowed, without converting status 2 to success.',
          'If partial stdout is followed by status 2, discard it or mark it incomplete.',
        ]),
      ],
    },
  },
  [RoutePath.Errors]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 실행',
      title: '오류 참조',
      summary: '오류 class는 실패 단계와 복구 가능한 사용자 입력을 드러냅니다.',
      sections: [
        section('compile 오류', [
          'Query syntax, POS·literal 충돌, 잘못된 option, 빈 analysis와 plan limit이 대상입니다. Query나 option을 수정해야 합니다.',
          'Resource capability required도 compile 단계에 나타날 수 있지만 boundary를 낮춰 자동 복구하지 않습니다.',
        ]),
        section('resource 오류', [
          '파일 누락, 128 MiB 상한, magic·schema·version·source mismatch, section digest와 payload graph 오류를 구분합니다.',
          '`kfind --check-data`로 설치 resource를 검색 없이 검증할 수 있습니다.',
        ]),
        section('I/O 오류', [
          'Path metadata, open, read, decode와 output write 실패를 source path와 함께 보고합니다. Permission 오류를 no-match로 바꾸지 않습니다.',
          'Locale은 설명 문장만 바꾸며 error class, path와 option token은 유지합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · EXECUTION',
      title: 'Error reference',
      summary:
        'Error classes expose the failing stage and whether user input can recover it.',
      sections: [
        section('Compile errors', [
          'Query syntax, POS-literal conflicts, invalid options, empty analyses, and plan limits require changing the query or options.',
          'A required resource capability can surface during compile, but boundary policy is not silently weakened.',
        ]),
        section('Resource errors', [
          'Missing files, the 128 MiB limit, magic, schema, version, source identity, section digest, and payload graph failures remain distinct.',
          '`kfind --check-data` validates installed resources without searching.',
        ]),
        section('I/O errors', [
          'Metadata, open, read, decode, and output-write failures identify the source path. Permission errors never become no-match results.',
          'Locale changes explanatory text only; error class, paths, and option tokens remain stable.',
        ]),
      ],
    },
  },
  [RoutePath.RustApi]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · API',
      title: 'Rust API',
      summary:
        '`kfind` facade는 안정 engine·matcher·resource 계약을 제공하고 내부 계획 실험은 expert module로 분리합니다.',
      sections: [
        section('안정 facade', [
          'Crate root는 `Engine`, `Matcher`, `ResourceBundle`, compile option, error와 match provenance type을 노출합니다. 이 surface가 1.x 호환 계약입니다.',
          'Matcher는 UTF-8 byte slice를 검색하고 파일 순회·인코딩·출력을 수행하지 않습니다.',
        ]),
        section('resource 초기화', [
          '`Engine::with_resources`는 선택적 full POS, enriched predicate와 component를 한 profile로 검증합니다. `Engine::new`는 embedded profile입니다.',
          'Component 교체는 `load_component_resource`가 전체 검증 뒤 적용하며 실패하면 기존 resource를 유지합니다.',
        ]),
        section('expert API', [
          '`kfind::expert`는 caller-configured `Lexicons`, `QueryPlan`과 matcher plan 접근을 제공합니다.',
          'Expert type은 root facade 인자나 반환값에 나타나지 않으며 1.x 안정 계약에 포함되지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · API',
      title: 'Rust API',
      summary:
        'The `kfind` facade provides stable engine, matcher, and resource contracts while plan experiments remain under expert.',
      sections: [
        section('Stable facade', [
          'The crate root exports `Engine`, `Matcher`, `ResourceBundle`, compile options, errors, and match-provenance types. This surface is the 1.x compatibility contract.',
          'A matcher searches UTF-8 bytes and performs no file traversal, encoding detection, or output.',
        ]),
        section('Resource initialization', [
          '`Engine::with_resources` validates optional full-POS, enriched-predicate, and component data as one profile. `Engine::new` creates the embedded profile.',
          '`load_component_resource` applies a replacement only after full validation and preserves the old resource on failure.',
        ]),
        section('Expert API', [
          '`kfind::expert` exposes caller-configured `Lexicons`, `QueryPlan`, and matcher-plan access.',
          'Expert types never appear in root facade parameters or returns and are outside the 1.x stable contract.',
        ]),
      ],
    },
  },
  [RoutePath.JavaScriptApi]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · API',
      title: 'JavaScript API',
      summary:
        '`@kfind/kfind`는 browser bundler와 Node.js에서 같은 Kfind·Matcher API를 제공합니다.',
      sections: [
        section('package export', [
          'Browser condition은 bundler ESM WASM을, Node condition은 CommonJS WASM target을 선택합니다. TypeScript declaration은 두 target이 공유합니다.',
          'Static asset은 `@kfind/kfind/assets/predicates.enriched.tsv`와 compact component subpath로 export합니다.',
        ]),
        section(
          'Kfind와 Matcher',
          [
            '`Kfind.withResources({ fullPos?, enrichedPredicates?, component? })`가 전체 profile 생성 API입니다. `compile(query, options)`은 재사용 가능한 `Matcher`를 반환합니다.',
            'API는 filesystem과 URL을 추정하지 않습니다. Caller가 resource string 또는 `Uint8Array`를 전달합니다.',
          ],
          {
            code: `import { Kfind } from '@kfind/kfind';

const engine = new Kfind();
const matcher = engine.compile('걷다', { pos: 'verb' });
const matches = matcher.findAll('길을 걸어 갔다.');`,
          },
        ),
        section('UTF-16 span', [
          'Match와 atom의 start·end는 UTF-16 code unit이며 JavaScript `slice`에 바로 사용할 수 있습니다. Emoji 앞의 offset도 code point 수가 아니라 code unit 수입니다.',
          '각 atom은 core, token과 모든 `analysisIndex`, `rulePath` origin을 보존합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · API',
      title: 'JavaScript API',
      summary:
        '`@kfind/kfind` exposes the same Kfind and Matcher API in browser bundlers and Node.js.',
      sections: [
        section('Package exports', [
          'The browser condition selects bundler ESM WASM; the Node condition selects the CommonJS WASM target. Both share TypeScript declarations.',
          'Static assets are exported at the enriched-predicate and compact-component subpaths under `@kfind/kfind/assets`.',
        ]),
        section(
          'Kfind and Matcher',
          [
            '`Kfind.withResources({ fullPos?, enrichedPredicates?, component? })` creates a complete profile. `compile(query, options)` returns a reusable `Matcher`.',
            'The API never guesses filesystem paths or URLs. Callers pass resource strings or `Uint8Array` values.',
          ],
          {
            code: `import { Kfind } from '@kfind/kfind';

const engine = new Kfind();
const matcher = engine.compile('걷다', { pos: 'verb' });
const matches = matcher.findAll('길을 걸어 갔다.');`,
          },
        ),
        section('UTF-16 spans', [
          'Match and atom start and end use UTF-16 code units and can be passed directly to JavaScript `slice`. An offset after emoji is not a code-point count.',
          'Every atom preserves core, token, and all `analysisIndex` and `rulePath` origins.',
        ]),
      ],
    },
  },
  [RoutePath.ReferenceResources]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 데이터',
      title: 'resource 참조',
      summary:
        '세 resource profile은 query coverage, 용언 정보와 source 구조라는 독립 기능을 제공합니다.',
      sections: [
        section('resource profile', [
          'Full POS `lexicon.bin`은 넓은 표제어·세부 품사를, enriched TSV는 검증된 용언 alternation·derivation을, compact KFC는 source component 분석을 제공합니다.',
          'npm package는 enriched와 compact를 포함하고 full POS를 포함하지 않습니다.',
        ]),
        section('schema', [
          'Manifest는 source URL·checksum, 생성 도구와 output digest를 기록합니다. Binary header는 schema, package version과 source identity를 포함합니다.',
          'TSV와 TOML은 build 단계에서 NFC, tag, rule ID와 중복 충돌을 검증합니다.',
        ]),
        section('호환성', [
          'Component package version은 engine version과 정확히 같아야 합니다. Schema와 source digest가 다르면 load를 거부합니다.',
          'Full과 compact projection은 구조 cost를 제외한 exact·common-prefix hit, POS와 span이 일치해야 합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · DATA',
      title: 'Resource reference',
      summary:
        'Three resource profiles independently provide query coverage, predicate metadata, and source structure.',
      sections: [
        section('Resource profiles', [
          'Full-POS `lexicon.bin` supplies broad lemmas and detailed POS; enriched TSV supplies verified predicate alternation and derivation; compact KFC supplies source components.',
          'The npm package includes enriched and compact resources but not full POS.',
        ]),
        section('Schemas', [
          'Manifests record source URLs and checksums, generation tools, and output digests. Binary headers contain schema, package version, and source identity.',
          'TSV and TOML builds validate NFC, tags, rule IDs, duplicates, and conflicts.',
        ]),
        section('Compatibility', [
          'The component package version must exactly match the engine. Schema or source-digest mismatch rejects loading.',
          'Full and compact projections agree on cost-free exact and common-prefix hits, POS, and spans.',
        ]),
      ],
    },
  },
  [RoutePath.RuleIds]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · provenance',
      title: '규칙 ID',
      summary:
        'Rule ID는 표면 후보가 생성·검증된 문법 단계를 namespace와 순서로 나타냅니다.',
      sections: [
        section('namespace', [
          '`lexical.*`은 어휘 교체, `ending.*`은 어미, `particle.*`은 조사, `derivation.*`은 파생, `structural.*`은 source constraint를 나타냅니다.',
          'ID는 사람이 읽을 수 있는 stable token이며 localized label로 바뀌지 않습니다.',
        ]),
        section('규칙 경로', [
          '`걷다→걸어` origin은 `lexical.d-to-l`, `ending.aoeo` 순서입니다. Path 순서는 조립 순서를 보존합니다.',
          '여러 분석이 같은 match를 만들면 atom에 여러 path가 남습니다.',
        ]),
        section('안정성', [
          '1.x에서 기존 ID의 의미를 다른 규칙으로 재사용하지 않습니다. 세분화가 필요하면 새 ID를 추가하고 fixture를 갱신합니다.',
          'Rule ID만으로 source 의미를 추론하지 않습니다. Analysis index와 span을 함께 사용합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · PROVENANCE',
      title: 'Rule IDs',
      summary:
        'Rule IDs encode grammar stages used to generate and verify a surface, preserving namespaces and order.',
      sections: [
        section('Namespaces', [
          '`lexical.*` identifies substitutions, `ending.*` endings, `particle.*` particles, `derivation.*` derivation, and `structural.*` source constraints.',
          'IDs are stable machine-readable tokens and are never localized.',
        ]),
        section('Rule paths', [
          'The origin for `걷다→걸어` contains `lexical.d-to-l` then `ending.aoeo`, preserving assembly order.',
          'Several paths remain on an atom when different analyses yield one match.',
        ]),
        section('Stability', [
          'Within 1.x, an existing ID is not reused for another meaning. A refined rule adds an ID and updates fixtures.',
          'A rule ID alone is not source semantics. Use it with analysis index and spans.',
        ]),
      ],
    },
  },
  [RoutePath.Licenses]: {
    [DocumentLocale.Korean]: {
      eyebrow: '참조 · 배포',
      title: '라이선스',
      summary:
        '소스 code와 재배포 data는 각 license와 notice를 package 및 release에 함께 포함합니다.',
      sections: [
        section('코드 라이선스', [
          'kfind source code는 저장소 `LICENSE`의 MIT License를 적용합니다. Rust crate, site source와 npm wrapper가 대상입니다.',
          'Dependency license는 각 upstream package의 조건을 따릅니다.',
        ]),
        section('데이터 라이선스', [
          'mecab-ko-dic에서 생성한 full POS와 compact component는 원본 COPYING을 artifact에 포함합니다. Enriched predicate는 source manifest와 NOTICE를 포함합니다.',
          '원본 snapshot과 checksum은 build manifest에 고정합니다.',
        ]),
        section('배포 notice', [
          'npm package의 `LICENSES.md`와 `assets/LICENSES`, GitHub release resource tarball, Homebrew 설치 resource에 필요한 notice를 함께 둡니다.',
          'Resource를 별도 host에 복사할 때도 해당 notice를 같이 배포합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'REFERENCE · DISTRIBUTION',
      title: 'Licenses',
      summary:
        'Source code and redistributed data carry their licenses and notices in packages and releases.',
      sections: [
        section('Code license', [
          'kfind source code uses the MIT License in repository `LICENSE`, covering Rust crates, site source, and the npm wrapper.',
          'Dependencies remain under their upstream licenses.',
        ]),
        section('Data licenses', [
          'Full-POS and compact-component artifacts derived from mecab-ko-dic include its COPYING. Enriched predicates include their source manifest and NOTICE.',
          'Build manifests pin original snapshots and checksums.',
        ]),
        section('Distribution notices', [
          'The npm package includes `LICENSES.md` and `assets/LICENSES`; GitHub resource tarballs and Homebrew-installed resources include required notices.',
          'A separately hosted resource must be distributed with the same notice.',
        ]),
      ],
    },
  },
};
