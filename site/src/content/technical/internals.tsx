import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const internalDocuments: TechnicalDocuments = {
  [RoutePath.Pipeline]: {
    [DocumentLocale.Korean]: {
      eyebrow: '내부 구조 · 실행',
      title: '실행 pipeline',
      summary:
        'Query 경로는 분석과 program 생성을 담당하고 corpus 경로는 anchor scan과 국소 검증만 수행합니다.',
      sections: [
        section('compile 단계', [
          'Lexer가 query atom과 태그를 만들고 normalization이 canonical variant를 준비합니다. Lexicon은 atom별 품사 분석과 교체 class를 반환합니다.',
          'Generator는 활용·파생 surface를 전부 긴 문자열로 보관하지 않고 anchor, core, suffix 전이와 구조 조건을 가진 `CandidateProgram`으로 낮춥니다.',
        ]),
        section('scan 단계', [
          'Program의 anchor를 Aho-Corasick automaton에 넣고 각 source chunk를 한 번 scan합니다. Anchor가 없는 byte 위치에는 형태 분석이나 구조 graph를 만들지 않습니다.',
          'Chunk 경계의 overlap은 가장 긴 anchor와 phrase 조건을 수용하고 원문 offset으로 환산됩니다.',
        ]),
        section('verify 단계', [
          'Anchor hit는 연결된 program만 실행합니다. Verifier가 core, 조사·어미 상태, boundary와 선택적 structural constraint를 순서대로 확인합니다.',
          'Compact graph는 해당 token과 인접 context만 준비합니다. Node·context limit을 넘으면 constraint unavailable 오류이며 단순 경계로 fallback하지 않습니다.',
        ]),
        section('출력 단계', [
          '승인된 atom span을 phrase matcher가 연결하고 중복·겹침 정책을 적용합니다. 최종 match는 원문 좌표와 모든 analysis origin을 유지합니다.',
          'CLI 계층은 match를 source line·column과 출력 schema로 변환합니다. Library matcher는 파일과 인코딩을 알지 못합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'INTERNALS · EXECUTION',
      title: 'Execution pipeline',
      summary:
        'The query lane analyzes and builds programs; the corpus lane only scans anchors and verifies local candidates.',
      sections: [
        section('Compilation', [
          'The lexer creates query atoms and tags; normalization prepares canonical variants. Lexicons return POS analyses and substitution classes per atom.',
          'The generator does not retain every surface as a long string. It lowers analyses into `CandidateProgram` values containing anchors, cores, suffix transitions, and structural requirements.',
        ]),
        section('Scanning', [
          'Program anchors enter an Aho-Corasick automaton that scans each source chunk once. Source offsets without an anchor receive no morphology analysis or structural graph.',
          'Chunk overlap covers the longest anchor and phrase requirement before offsets are mapped back to the original source.',
        ]),
        section('Verification', [
          'An anchor hit executes only its linked programs. The verifier checks core, particle or ending states, boundaries, and optional structural constraints in order.',
          'The compact graph is prepared only for the token and nearby context. Exceeding node or context limits is a constraint-unavailable error, never a fallback to simpler boundaries.',
        ]),
        section('Emission', [
          'The phrase matcher connects approved atom spans and applies overlap and deduplication policy. Final matches preserve source coordinates and every analysis origin.',
          'The CLI layer converts matches to source lines, columns, and output schemas. The library matcher knows nothing about files or encodings.',
        ]),
      ],
    },
  },
  [RoutePath.QueryCompiler]: {
    [DocumentLocale.Korean]: {
      eyebrow: '내부 구조 · compile',
      title: 'query compiler',
      summary:
        'Compiler는 열린 형태 분석을 실행 비용이 제한된 검색 IR로 변환합니다.',
      sections: [
        section('분석 후보', [
          'Lexicon lookup은 표제어, coarse POS와 user entry를 조합해 가능한 분석을 모두 반환합니다. Explicit POS는 포함되는 세부 품사를 제한하지만 같은 범주 안의 중의성은 유지합니다.',
          '분석은 stem alternation, particle·ending start state, derivation path와 source capability requirement를 가집니다.',
        ]),
        section('program IR', [
          '`CandidateProgram`은 anchor bytes, core 조립, suffix automaton, boundary와 `StructuralConstraint`를 담습니다. Provenance는 lexical·derivation·ending rule ID의 순서 있는 경로입니다.',
          'Resource가 필요한 constraint는 compile 결과에 capability로 남습니다. Matcher 생성 시 resource가 없으면 fail-fast 오류입니다.',
        ]),
        section('정렬과 중복 제거', [
          'Program은 anchor, 실행 조건과 span projection의 결정적 key로 정렬합니다. 실행 조건이 같은 분석은 program을 합치고 origin 집합만 union합니다.',
          'Plan 상한은 program·anchor와 structural state 폭을 제한합니다. 상한 초과는 일부 후보를 버리는 대신 compile 오류를 반환합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'INTERNALS · COMPILATION',
      title: 'Query compiler',
      summary:
        'The compiler turns open-ended morphology analyses into a bounded search IR.',
      sections: [
        section('Analysis candidates', [
          'Lexicon lookup combines the lemma, coarse POS, and user entries while preserving every compatible analysis. Explicit POS narrows detailed tags but does not remove ambiguity inside the category.',
          'An analysis carries stem alternation, particle or ending start state, derivation path, and source-capability requirements.',
        ]),
        section('Program IR', [
          'A `CandidateProgram` contains anchor bytes, core assembly, a suffix automaton, boundary policy, and `StructuralConstraint`. Provenance is an ordered path of lexical, derivation, and ending rule IDs.',
          'A resource-dependent constraint remains an explicit capability in the compiled plan. Matcher construction fails fast when that resource is absent.',
        ]),
        section('Ordering and deduplication', [
          'Programs are sorted by deterministic keys over anchors, execution conditions, and span projection. Analyses with identical execution merge programs while unioning origins.',
          'Plan limits bound program, anchor, and structural-state width. Exceeding a limit raises a compile error instead of dropping candidates.',
        ]),
      ],
    },
  },
  [RoutePath.Matcher]: {
    [DocumentLocale.Korean]: {
      eyebrow: '내부 구조 · scan',
      title: 'matcher',
      summary:
        'Matcher는 byte anchor를 넓게 찾고 program verifier로 형태 조건을 좁힙니다.',
      sections: [
        section('anchor index', [
          '여러 program이 같은 anchor를 공유하면 automaton state와 hit list를 공유합니다. Anchor는 candidate coverage를 보존하는 범위에서 길고 희소한 surface를 우선합니다.',
          'NFC와 NFD variant는 별도 byte pattern이지만 같은 logical program과 source span projection으로 연결됩니다.',
        ]),
        section('후보 window', [
          'Hit offset에서 core가 시작·끝날 수 있는 제한된 window만 검사합니다. 조사와 어미 automaton은 suffix Unicode scalar를 전이하며 불완전한 소비를 거부합니다.',
          'Structural verifier는 token 전체가 필요한 경우에만 compact resource index를 조회합니다.',
        ]),
        section('match 선택', [
          '같은 start에서 여러 program이 승인되면 더 긴 완성 token과 결정적 program key를 사용해 non-overlap 결과를 만듭니다. Origin은 손실 없이 합칩니다.',
          'Phrase mode에서는 atom 후보를 먼저 보존한 뒤 순서와 gap을 적용하므로 단일 atom의 최장 match가 다음 atom을 숨기지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'INTERNALS · SCANNING',
      title: 'Matcher',
      summary:
        'The matcher finds byte anchors broadly and narrows them through program verifiers.',
      sections: [
        section('Anchor index', [
          'Programs sharing an anchor also share automaton state and hit lists. Anchor selection favors long, rare surfaces without reducing candidate coverage.',
          'NFC and NFD variants are separate byte patterns linked to the same logical program and source-span projection.',
        ]),
        section('Candidate window', [
          'Only a bounded window in which the core can begin and end is inspected around a hit. Particle and ending automata transition over suffix scalars and reject incomplete consumption.',
          'The structural verifier consults the compact index only when a whole-token analysis is required.',
        ]),
        section('Match selection', [
          'When several programs succeed at one start, completed-token length and deterministic program keys produce non-overlapping results while preserving all origins.',
          'Phrase mode retains atom candidates before applying order and gap, preventing a single-atom longest match from hiding the next atom.',
        ]),
      ],
    },
  },
  [RoutePath.StructuralVerification]: {
    [DocumentLocale.Korean]: {
      eyebrow: '내부 구조 · source',
      title: '구조 판정',
      summary:
        '구조 판정은 의미 추론 없이 source 분석 graph가 query constraint를 증명하는지만 확인합니다.',
      sections: [
        section('구조 제약', [
          'Constraint는 query core에 대응하는 component span, 필요한 POS, token 전체 경로와 선행·후행 component 조건을 선언합니다. 직접 surface 예외 목록은 사용하지 않습니다.',
          'Exact path와 common-prefix path는 source 분석이 선언한 component 순서와 span이 모두 맞아야 승인됩니다.',
        ]),
        section('graph 준비', [
          'Decoder는 surface에 연결된 분석과 component edge를 읽고 final context preparation에 필요한 node만 materialize합니다. 공통 fact와 immutable slice를 공유해 query별 복사를 줄입니다.',
          'Node 수, context 깊이와 payload 크기 상한은 malformed resource에서도 메모리 사용을 제한합니다.',
        ]),
        section('판정 실패', [
          '필요한 path가 없으면 후보는 false입니다. Resource가 없거나 graph limit을 넘으면 constraint unavailable로 구분해 제품이 precision이 낮은 경로로 바뀌지 않게 합니다.',
          '여러 source 분석 중 하나가 constraint를 만족하면 후보를 유지하고 해당 analysis index를 provenance에 기록합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'INTERNALS · SOURCE',
      title: 'Structural verification',
      summary:
        'Structural verification asks only whether source analysis graphs prove query constraints, without semantic inference.',
      sections: [
        section('Structural constraints', [
          'A constraint declares the component span corresponding to the query core, required POS, whole-token path, and neighboring components. It uses no registry of exceptional source strings.',
          'Exact and common-prefix paths require both component order and spans declared by the source analysis.',
        ]),
        section('Graph preparation', [
          'The decoder reads analyses and component edges for a surface and materializes only nodes required by final context preparation. Shared facts and immutable slices reduce per-query copying.',
          'Limits on nodes, context depth, and payload size bound memory even for malformed resources.',
        ]),
        section('Verification failure', [
          'A missing path rejects the candidate. A missing resource or graph limit is constraint unavailable, preventing silent degradation to a lower-precision path.',
          'If any source analysis satisfies the constraint, the candidate remains and its analysis index enters provenance.',
        ]),
      ],
    },
  },
  [RoutePath.InternalResources]: {
    [DocumentLocale.Korean]: {
      eyebrow: '내부 구조 · 데이터',
      title: 'resource 구조',
      summary:
        '각 resource는 query lexicon 또는 source 구조라는 한 책임과 독립 호환성 계약을 가집니다.',
      sections: [
        section('resource 계층', [
          'Core·enriched·full POS는 query 분석용 lexicon 계층입니다. Compact component는 source surface에서 분석과 component를 조회하는 구조 index입니다.',
          '생성 source와 checksum은 manifest에 고정합니다. Runtime network fetch나 background update는 없습니다.',
        ]),
        section('container 형식', [
          'Binary container는 magic, schema, package version, source identity, section directory와 SHA-256을 포함합니다. Section은 큰 index와 payload를 독립 검증할 수 있습니다.',
          'Encoded component resource는 128 MiB 상한을 먼저 검사합니다. 길이와 offset을 확인한 뒤에만 digest와 payload graph를 읽습니다.',
        ]),
        section('초기화 수명주기', [
          'Native engine은 검증된 resource를 소유하고 matcher 사이에서 재사용합니다. JavaScript는 `withResources` 또는 `loadComponentResource`로 bytes를 명시적으로 전달합니다.',
          '교체는 새 bytes의 모든 검증이 끝난 뒤 원자적으로 적용됩니다. 실패하면 기존 state를 보존합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'INTERNALS · DATA',
      title: 'Resource architecture',
      summary:
        'Each resource has one query-lexicon or source-structure responsibility and an independent compatibility contract.',
      sections: [
        section('Resource layers', [
          'Core, enriched, and full POS form query-analysis lexicon layers. The compact component artifact indexes analyses and components for source surfaces.',
          'Generation sources and checksums are pinned in manifests. Runtime network fetching and background updates do not exist.',
        ]),
        section('Container format', [
          'A binary container includes magic, schema, package version, source identity, section directory, and SHA-256. Large index and payload sections can be verified independently.',
          'The 128 MiB encoded-resource limit is checked first. Lengths and offsets are validated before digest or payload graphs are read.',
        ]),
        section('Initialization lifecycle', [
          'A native engine owns validated resources and reuses them across matchers. JavaScript callers pass bytes explicitly through `withResources` or `loadComponentResource`.',
          'Replacement applies atomically only after all validation completes. Failure preserves existing state.',
        ]),
      ],
    },
  },
  [RoutePath.UnicodeSpans]: {
    [DocumentLocale.Korean]: {
      eyebrow: '내부 구조 · text',
      title: 'Unicode와 span',
      summary:
        '검색 variant를 정규화해도 결과 좌표와 surface는 원문 표현을 가리킵니다.',
      sections: [
        section('정규화', [
          '`nfc`는 query와 source 비교를 NFC 기준으로 준비합니다. `canonical`은 NFC와 NFD를 모두 검색하며 `none`은 입력 code point sequence를 그대로 사용합니다.',
          '정규화된 buffer는 matching 전용입니다. 출력 surface를 정규화된 문자열로 바꾸지 않습니다.',
        ]),
        section('좌표계', [
          'Rust matcher의 기본 span은 UTF-8 byte offset입니다. JavaScript binding은 변환 map을 사용해 `String.prototype.slice`와 호환되는 UTF-16 code unit offset을 반환합니다.',
          'CLI line·column은 frontend contract에 맞는 좌표계를 따르며 npm CLI는 UTF-16, native source report는 byte·decoded scalar mapping을 보존합니다.',
        ]),
        section('원문 span', [
          'Normalization map은 각 normalized scalar가 온 원문 byte 또는 UTF-16 range를 추적합니다. 결합 문자와 emoji 안쪽으로 span을 자르지 않습니다.',
          'Phrase span도 첫 atom과 마지막 atom의 원문 range를 합칩니다. Provenance의 core·token span은 같은 좌표계를 사용합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'INTERNALS · TEXT',
      title: 'Unicode and spans',
      summary:
        'Even when search variants are normalized, result coordinates and surfaces point to the original representation.',
      sections: [
        section('Normalization', [
          '`nfc` prepares query and source comparisons in NFC. `canonical` searches NFC and NFD; `none` preserves the input code-point sequence.',
          'Normalized buffers exist only for matching. Output surfaces are never replaced with normalized strings.',
        ]),
        section('Coordinate systems', [
          'The Rust matcher uses UTF-8 byte offsets. The JavaScript binding maps them to UTF-16 code-unit offsets compatible with `String.prototype.slice`.',
          'CLI line and column values follow each frontend contract. The npm CLI uses UTF-16, while native source reporting preserves byte-to-decoded-scalar mappings.',
        ]),
        section('Source spans', [
          'Normalization maps track the original byte or UTF-16 range of every normalized scalar. Spans never split a combining sequence or emoji encoding.',
          'Phrase spans combine the original ranges of the first and last atoms. Provenance core and token spans use the same coordinate system.',
        ]),
      ],
    },
  },
};
