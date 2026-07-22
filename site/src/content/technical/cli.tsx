import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const cliDocuments: TechnicalDocuments = {
  [RoutePath.QuerySyntax]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 질의',
      title: '질의 문법',
      summary:
        '질의는 공백으로 나뉜 atom과 선택적 품사 태그로 구성되며 compile 전에 완전한 문법 검증을 거칩니다.',
      sections: [
        section(
          '형식 문법',
          [
            '공백은 atom 경계입니다. Atom은 태그가 붙은 bare text 또는 따옴표로 감싼 literal입니다. Backslash는 다음 Unicode scalar를 escape합니다.',
            '빈 atom, 닫히지 않은 따옴표, 값이 없는 태그와 알 수 없는 태그는 compile 오류입니다.',
          ],
          {
            code: `query   = atom *(SP atom)
atom    = [pos-tag] (bare / quoted)
pos-tag = "n:" / "v:" / "adj:" / "lit:" / ...`,
          },
        ),
        section('atom과 태그', [
          '`v:걷다`는 동사 분석만, `n:걷기`는 명사 분석만 허용합니다. 태그가 없으면 전역 `--pos` 또는 `auto`가 적용됩니다.',
          '구 query의 각 atom은 독립 분석과 program 집합을 가집니다. 같은 tag를 전체 구에 암묵적으로 전파하지 않습니다.',
        ]),
        section(
          '인용과 escape',
          [
            '`"로그 인"`은 공백을 포함한 하나의 literal atom입니다. `"`와 `\\`는 따옴표와 backslash를 문자로 보존합니다.',
            '형태 확장이 필요한 표제어를 따옴표로 감싸는 것은 literal mode와 같지 않습니다. 인용은 atom 경계만 바꾸고 expansion은 option이 결정합니다.',
          ],
          {
            code: String.raw`kfind 'v:걷다 "로그 인"' src
kfind --literal 'key\:value' config`,
          },
        ),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · QUERY',
      title: 'Query syntax',
      summary:
        'A query consists of whitespace-delimited atoms with optional POS tags and is fully validated before compilation.',
      sections: [
        section(
          'Grammar',
          [
            'Whitespace separates atoms. An atom is bare or quoted text with an optional tag. A backslash escapes the following Unicode scalar.',
            'Empty atoms, unclosed quotes, missing tag values, and unknown tags are compile errors.',
          ],
          {
            code: `query   = atom *(SP atom)
atom    = [pos-tag] (bare / quoted)
pos-tag = "n:" / "v:" / "adj:" / "lit:" / ...`,
          },
        ),
        section('Atoms and tags', [
          '`v:걷다` permits only verb analyses, while `n:걷기` permits only noun analyses. An untagged atom receives global `--pos` or `auto`.',
          'Each phrase atom owns an independent analysis and program set. A tag is never propagated implicitly across the phrase.',
        ]),
        section(
          'Quoting and escaping',
          [
            '`"로그 인"` is one literal atom containing a space. `"` and `\\` preserve a quote and backslash as characters.',
            'Quoting an inflected lemma does not enable literal mode. Quoting changes atom boundaries; the expansion option determines morphology.',
          ],
          {
            code: String.raw`kfind 'v:걷다 "로그 인"' src
kfind --literal 'key\:value' config`,
          },
        ),
      ],
    },
  },
  [RoutePath.PartsOfSpeech]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 질의',
      title: '품사 지정',
      summary:
        '공개 품사는 검색 규칙 선택을 위한 coarse category이며 source의 세부 품사 판정과 구분됩니다.',
      sections: [
        section('coarse POS', [
          '공개 값은 `noun`, `pronoun`, `numeral`, `verb`, `adjective`, `determiner`, `adverb`, `particle`, `interjection`, `literal`입니다. 내부 사전의 NNG·NNP는 noun에, VV는 verb에 포함됩니다.',
          '`auto`는 사전에 등록된 모든 호환 분석을 보존합니다. 하나를 우선 선택하지 않으므로 동형 표제어는 여러 program을 만들 수 있습니다.',
        ]),
        section('선택 규칙', [
          'Atom tag와 전역 `--pos`가 모두 구체 값이면 같아야 합니다. 다르면 사용자의 두 제약을 동시에 만족할 수 없으므로 compile 오류입니다.',
          '`--literal`은 `expand=literal`, `pos=literal`의 단축입니다. 다른 `--expand`나 non-literal POS와 함께 쓰면 충돌합니다.',
        ]),
        section('품사 중의성', [
          'Query 품사 중의성은 후보 program 집합으로 보존합니다. Source 품사는 `smart` 구조 판정에서 compact resource가 제공하는 세부 분석과 비교합니다.',
          '형태 구조로도 구분되지 않는 명사·용언 중의성은 결과에 남습니다. 의미 빈도나 language model score로 임의 순위를 매기지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · QUERY',
      title: 'Part-of-speech selection',
      summary:
        'Public POS values are coarse rule-selection categories and remain distinct from detailed source POS.',
      sections: [
        section('Coarse POS', [
          'Public values are `noun`, `pronoun`, `numeral`, `verb`, `adjective`, `determiner`, `adverb`, `particle`, `interjection`, and `literal`. Internal NNG and NNP map to noun, while VV maps to verb.',
          '`auto` preserves every compatible lexicon analysis. It does not choose one preferred interpretation, so a homographic lemma can produce several programs.',
        ]),
        section('Selection rules', [
          'When an atom tag and global `--pos` are both concrete, they must agree. A mismatch is a compile error because both user constraints cannot hold.',
          '`--literal` abbreviates `expand=literal` and `pos=literal`. It conflicts with another expansion or a non-literal POS.',
        ]),
        section('POS ambiguity', [
          'Query-side POS ambiguity remains a set of candidate programs. Under `smart`, source-side detailed analyses from the compact resource are compared with structural constraints.',
          'Ambiguity that morphology cannot resolve remains in the results. kfind does not invent a frequency or language-model ranking.',
        ]),
      ],
    },
  },
  [RoutePath.Expansion]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 질의',
      title: '형태 확장',
      summary:
        'Expansion mode는 표제어에서 생성할 형태의 상한을 정하고 source 경계 정책과 독립적으로 동작합니다.',
      sections: [
        section('literal', [
          '`literal`은 입력 Unicode 문자열만 검색합니다. 조사·어미 소비, 불규칙 교체와 파생 program을 만들지 않습니다.',
          '정규화 option은 계속 적용됩니다. 따라서 `canonical`이면 NFC와 NFD 표현을 같은 문자열의 canonical variant로 찾습니다.',
        ]),
        section('inflection', [
          '기본값인 `inflection`은 품사를 유지하는 조사 결합과 용언 활용을 생성합니다. 받침 이형태, 어미 연쇄, 불규칙 활용과 등록된 축약을 포함합니다.',
          '체언에서 용언으로 바뀌는 `하다` 파생처럼 품사가 달라지는 경로는 포함하지 않습니다.',
        ]),
        section('derivation', [
          '`derivation`은 inflection에 더해 versioned rule data의 생산적 파생을 허용합니다. 명사+`하다`, 지정된 접두·접미사와 파생 뒤 활용이 대상입니다.',
          '가능해 보이는 모든 접사를 조합하지 않습니다. 사전 품사, source component와 rule별 구조 제약이 완성된 경로만 program이 됩니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · QUERY',
      title: 'Expansion modes',
      summary:
        'Expansion sets the upper bound of surfaces generated from a lemma independently of source boundary policy.',
      sections: [
        section('Literal', [
          '`literal` searches only the input Unicode string. It creates no particle, ending, irregular, or derivational programs.',
          'Normalization still applies. With `canonical`, NFC and NFD forms are searched as canonical variants of the same string.',
        ]),
        section('Inflection', [
          'The default `inflection` mode generates POS-preserving particle attachment and predicate inflection. It includes coda-conditioned allomorphs, ending chains, irregulars, and registered contractions.',
          'POS-changing paths such as noun-to-predicate `하다` derivation are excluded.',
        ]),
        section('Derivation', [
          '`derivation` adds productive paths from versioned rule data, including noun plus `하다`, selected affixes, and subsequent inflection.',
          'It does not combine every plausible affix. A program requires complete lexicon POS, source components, and rule-specific structural constraints.',
        ]),
      ],
    },
  },
  [RoutePath.Boundaries]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 검색 조건',
      title: '경계 정책',
      summary:
        '경계는 생성된 core가 source에서 어느 구조까지 소비해야 match인지 결정합니다.',
      sections: [
        section('smart 경계', [
          '`smart`는 core 뒤의 조사·어미 전이를 소비하고 필요한 경우 source component 품사열을 검증합니다. Query program이 구조 capability를 선언하면 compact resource가 필수입니다.',
          '완성된 token과 허용된 내부 성분은 보존하지만 다른 품사의 우연한 substring은 거부합니다. 의미 중의성은 판정하지 않습니다.',
        ]),
        section('token 경계', [
          '`token`은 core 시작과 소비가 끝난 token 끝에서 Unicode word 경계를 요구합니다. Component 내부 검색을 허용하지 않으므로 독립 token 확인에 적합합니다.',
          '조사와 어미는 program이 소비한 뒤 경계를 확인합니다. 표제어 문자열 바로 뒤에서 단순히 잘라 판정하지 않습니다.',
        ]),
        section('any 경계', [
          '`any`는 좌우 token 경계를 요구하지 않고 생성 program이 확인한 span을 반환합니다. 후보 수집과 recall 중심 자동화에 사용합니다.',
          '경계 제약을 풀어도 형태 생성 규칙 자체가 풀리지는 않습니다. Literal substring 검색이 필요하면 `--literal --boundary any`를 함께 사용합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · MATCHING',
      title: 'Boundary policies',
      summary:
        'Boundary policy determines how much source structure a generated core must consume to become a match.',
      sections: [
        section('Smart boundary', [
          '`smart` consumes particle and ending transitions after the core and verifies source component POS when required. The compact resource is mandatory when a program declares structural capability.',
          'It preserves completed tokens and permitted internal components while rejecting accidental substrings of another POS. It does not resolve semantics.',
        ]),
        section('Token boundary', [
          '`token` requires Unicode word boundaries at the core start and the end of all consumed material. It excludes component-internal matches and suits independent-token checks.',
          'The matcher checks the boundary after consuming permitted particles or endings, not immediately after the lemma string.',
        ]),
        section('Any boundary', [
          '`any` imposes no left or right token boundary and returns spans verified by generated programs. Use it for broad collection and recall-oriented automation.',
          'Relaxing boundaries does not relax morphology. Combine `--literal --boundary any` for literal substring search.',
        ]),
      ],
    },
  },
  [RoutePath.Phrases]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 검색 조건',
      title: '구 검색',
      summary:
        '구 query는 atom별 형태 match를 같은 line에서 순서대로 결합합니다.',
      sections: [
        section('atom 순서', [
          '각 atom은 독립 matcher로 source 후보를 만듭니다. Phrase matcher는 이전 atom의 token 끝보다 뒤에 시작하는 다음 후보만 연결합니다.',
          '기본 계약은 같은 logical line입니다. Newline을 넘어선 결합은 하지 않으므로 source line 위치를 보존합니다.',
        ]),
        section('간격', [
          '`--max-gap`은 앞 token 끝과 다음 token 시작 사이의 Unicode scalar 수 상한입니다. 기본값 24에는 공백과 punctuation이 포함됩니다.',
          'UTF-8 byte 수나 UTF-16 code unit 수가 아닙니다. Emoji와 결합 문자가 있어도 scalar 기준으로 동일하게 계산합니다.',
        ]),
        section('구 span', [
          '결과 span은 첫 atom core 시작부터 마지막 atom의 소비된 token 끝까지입니다. 각 atom은 별도 core·token span과 provenance를 유지합니다.',
          '겹치는 후보 조합은 결정적 정렬 후 중복 제거합니다. 같은 surface라도 atom provenance가 다르면 origin 집합으로 합칩니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · MATCHING',
      title: 'Phrase search',
      summary:
        'A phrase query combines per-atom morphology matches in source order on the same line.',
      sections: [
        section('Atom order', [
          'Each atom produces source candidates through an independent matcher. The phrase matcher connects only a following candidate that begins after the previous token end.',
          'The default contract stays within one logical line. It does not connect across a newline, preserving source-line location.',
        ]),
        section('Gap', [
          '`--max-gap` limits Unicode scalars between the previous token end and next token start. The default 24 includes whitespace and punctuation.',
          'It is neither a UTF-8 byte count nor a UTF-16 code-unit count. Emoji and combining characters follow the same scalar rule.',
        ]),
        section('Phrase span', [
          'The result spans from the first atom core start through the last consumed token end. Every atom retains separate core and token spans plus provenance.',
          'Overlapping combinations are deterministically sorted and deduplicated. Analyses sharing a surface merge their origin sets rather than discarding provenance.',
        ]),
      ],
    },
  },
  [RoutePath.InputOutput]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 실행',
      title: '입력과 출력',
      summary:
        'Native CLI와 npm CLI는 matcher를 공유하지만 파일 순회, 인코딩과 표시 기능의 범위가 다릅니다.',
      sections: [
        section('입력 source', [
          'Native CLI는 path, directory와 stdin을 받고 ignore 규칙 및 병렬 file scan을 적용합니다. `-`는 stdin을 명시합니다.',
          'npm CLI는 UTF-8 path를 결정적 순서로 재귀 순회하며 symlink를 따라가지 않습니다. Path가 없으면 TTY에서는 현재 directory, pipe에서는 stdin을 선택합니다.',
        ]),
        section('인코딩', [
          'Native CLI의 `auto`는 UTF-8과 BOM 기반 UTF-16을 판별하고 명시적 `euc-kr`을 지원합니다. Decode한 text의 match를 원래 source 위치로 다시 매핑합니다.',
          'npm CLI는 UTF-8 전용입니다. NUL이 있거나 strict UTF-8 decode가 실패한 파일은 stderr 진단 뒤 건너뜁니다.',
        ]),
        section('출력 형식', [
          '사람용 text는 path, line, column과 surface를 표시합니다. Native TTY에서는 조건에 따라 pager를 사용하고 `--no-pager`로 bounded stdout stream을 강제할 수 있습니다.',
          '자동화는 JSON Lines를 사용합니다. Match, atom span과 모든 `rulePath`를 record마다 보존합니다.',
        ]),
        section('npm CLI', [
          'npm CLI의 text 좌표는 1부터 시작하는 UTF-16 line·column입니다. `--json` record는 path, line, column, start, end, surface와 atoms를 포함합니다.',
          'Match가 있으면 0, 없으면 1, 사용법·초기화·I/O 오류면 2로 종료합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · EXECUTION',
      title: 'Input and output',
      summary:
        'Native and npm CLIs share the matcher but expose different file traversal, encoding, and presentation surfaces.',
      sections: [
        section('Input sources', [
          'The native CLI accepts paths, directories, and stdin with ignore rules and parallel file scanning. `-` explicitly selects stdin.',
          'The npm CLI recursively visits UTF-8 paths in deterministic order and does not follow symlinks. With no path it selects the current directory on a TTY and stdin on a pipe.',
        ]),
        section('Encoding', [
          'Native `auto` detects UTF-8 and BOM-marked UTF-16; explicit `euc-kr` is also supported. Matches in decoded text are mapped back to source positions.',
          'The npm CLI accepts UTF-8 only. It reports and skips files containing NUL or failing strict UTF-8 decoding.',
        ]),
        section('Output formats', [
          'Human-readable text contains path, line, column, and surface. The native CLI may use a pager on a TTY; `--no-pager` forces the bounded stdout stream.',
          'Automation uses JSON Lines. Each record preserves the match, atom spans, and every `rulePath`.',
        ]),
        section('npm CLI', [
          'npm text coordinates are one-based UTF-16 line and column values. A `--json` record contains path, line, column, start, end, surface, and atoms.',
          'Exit status is 0 with a match, 1 without a match, and 2 for usage, initialization, or I/O failure.',
        ]),
      ],
    },
  },
  [RoutePath.Diagnostics]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 실행',
      title: '진단과 오류',
      summary:
        '오류는 query compile, resource 초기화와 source I/O 단계별로 구분됩니다.',
      sections: [
        section('진단 형식', [
          '사람이 읽는 진단은 stderr에만 기록하고 match output은 stdout에 유지합니다. 오류 message에는 실패한 option, path 또는 resource 조건을 포함합니다.',
          'JSON Lines mode에서도 진단을 JSON record로 섞지 않습니다. 호출자는 두 stream과 종료 상태를 함께 보존합니다.',
        ]),
        section('오류 분류', [
          'Compile 오류에는 빈 query, tag·전역 POS 충돌, 잘못된 option 값과 plan 상한 초과가 포함됩니다. Resource 오류에는 누락, version·schema·source mismatch와 digest 실패가 포함됩니다.',
          '입력 오류는 path 조회, decode와 read 실패입니다. 출력 오류는 broken pipe를 제외한 stdout write 실패입니다.',
        ]),
        section('종료 상태', [
          'Native CLI는 명령별 종료 계약을 따르며 npm CLI는 match 0, no-match 1, 오류 2를 사용합니다. Shell에서 `set -e`를 사용할 때 no-match를 허용할지 명시합니다.',
          '부분 stdout이 있어도 최종 상태가 오류면 완결된 결과가 아닙니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · EXECUTION',
      title: 'Diagnostics and errors',
      summary:
        'Failures are classified by query compilation, resource initialization, and source I/O stage.',
      sections: [
        section('Diagnostic format', [
          'Human-readable diagnostics go only to stderr, leaving match output on stdout. Messages identify the failing option, path, or resource condition.',
          'JSON Lines mode does not mix diagnostics into JSON records. A caller preserves both streams and the final status.',
        ]),
        section('Error classes', [
          'Compile errors include empty queries, tag/global POS conflicts, invalid options, and plan-limit failures. Resource errors include missing data, version, schema, source-identity, and digest failures.',
          'Input errors cover path lookup, decode, and reads. Output errors cover stdout writes other than a normal broken pipe.',
        ]),
        section('Exit status', [
          'The native CLI follows command-specific status rules; the npm CLI uses 0 for matches, 1 for no match, and 2 for errors. A `set -e` shell must explicitly decide whether no match is allowed.',
          'Partial stdout is not a complete result when the final status reports an error.',
        ]),
      ],
    },
  },
  [RoutePath.CliResources]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 실행',
      title: '사전과 resource',
      summary:
        '사전 계층은 query 분석과 source 구조 판정의 책임 및 비용을 분리합니다.',
      sections: [
        section('사전 profile', [
          'Embedded 사전은 핵심 품사, 기능어와 불규칙을 제공합니다. Enriched predicate는 고정 사전 snapshot이 지지하는 용언 정보를 더하고 full POS는 세부 품사 후보를 확장합니다.',
          'Compact component는 source surface의 품사열과 component span을 제공하며 query lexicon을 확장하지 않습니다.',
        ]),
        section('resource 해석', [
          'Native CLI는 설치 prefix, 환경 변수와 명시적 option의 우선순위로 resource를 찾습니다. 필요한 capability가 없으면 초기화 또는 compile 오류입니다.',
          'JavaScript API는 경로를 추정하지 않습니다. npm CLI만 package 내부 asset을 알고 component가 필요한 query에서 lazy load합니다.',
        ]),
        section('호환성 검증', [
          'Binary resource header의 package version, schema, source identity, section length와 SHA-256을 모두 검증합니다. 검증이 끝나기 전 bytes를 engine state에 설치하지 않습니다.',
          '새 resource load가 실패하면 이미 검증된 component state는 유지됩니다. 다른 package version의 asset으로 fallback하지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · EXECUTION',
      title: 'Lexicons and resources',
      summary:
        'Lexicon layers separate the responsibility and cost of query analysis from source structural verification.',
      sections: [
        section('Lexicon profiles', [
          'The embedded lexicon provides core POS, function words, and irregulars. Enriched predicates add entries supported by pinned dictionary snapshots; full POS expands detailed POS candidates.',
          'The compact component resource supplies POS sequences and component spans for source surfaces. It does not expand the query lexicon.',
        ]),
        section('Resource resolution', [
          'The native CLI resolves resources by explicit options, environment, and installation prefix. Missing required capability is an initialization or compile error.',
          'The JavaScript API never guesses paths. Only the npm CLI knows package assets and lazily loads the component resource for a query that needs it.',
        ]),
        section('Compatibility validation', [
          'Binary headers, package version, schema, source identity, section lengths, and SHA-256 are validated before bytes enter engine state.',
          'A failed replacement preserves the previously validated component. No asset from another package version is used as fallback.',
        ]),
      ],
    },
  },
  [RoutePath.Recipes]: {
    [DocumentLocale.Korean]: {
      eyebrow: 'CLI · 예시',
      title: '검색 예시',
      summary: '예시는 query 의도, 경계와 출력 소비자를 함께 고정합니다.',
      sections: [
        section(
          '코드 검색',
          [
            '함수명 주변의 한국어 주석과 문서를 탐색할 때는 넓은 `any`로 후보를 모은 뒤 `smart`로 재검증합니다. 정확한 identifier는 literal mode를 사용합니다.',
          ],
          {
            code: `kfind --pos verb --boundary any 변경하다 src
kfind --pos verb --boundary smart 변경하다 src
kfind --literal parse_query crates`,
          },
        ),
        section(
          '문서 감사',
          [
            '표제어와 활용형이 혼재한 문서에서는 기본 inflection을 사용합니다. 특정 용어 표기만 검사할 때는 literal query를 각각 실행합니다.',
          ],
          {
            code: `kfind --pos noun 계약 docs README.md
kfind --literal contract-adjust docs site/src`,
          },
        ),
        section(
          '에이전트 입력',
          [
            '에이전트에는 JSON Lines와 제한된 path를 전달합니다. `jq`에서 source surface를 제거하거나 provenance를 축약하지 않습니다.',
          ],
          {
            code: `kfind --pos verb --boundary any --json 검증하다 crates/kfind-query \
  | jq -c '{path, line, column, surface, atoms}'`,
          },
        ),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'CLI · RECIPES',
      title: 'Search recipes',
      summary:
        'Each recipe fixes query intent, boundary policy, and output consumer together.',
      sections: [
        section(
          'Code search',
          [
            'For Korean comments and documentation near symbols, collect with broad `any` and verify again with `smart`. Use literal mode for exact identifiers.',
          ],
          {
            code: `kfind --pos verb --boundary any 변경하다 src
kfind --pos verb --boundary smart 변경하다 src
kfind --literal parse_query crates`,
          },
        ),
        section(
          'Documentation audit',
          [
            'Use the default inflection mode when documents contain lemmas and inflected surfaces. Run separate literal queries for exact terminology checks.',
          ],
          {
            code: `kfind --pos noun 계약 docs README.md
kfind --literal contract-adjust docs site/src`,
          },
        ),
        section(
          'Agent input',
          [
            'Feed agents JSON Lines from bounded paths. Do not remove source surfaces or collapse provenance in `jq`.',
          ],
          {
            code: `kfind --pos verb --boundary any --json 검증하다 crates/kfind-query \
  | jq -c '{path, line, column, surface, atoms}'`,
          },
        ),
      ],
    },
  },
};
