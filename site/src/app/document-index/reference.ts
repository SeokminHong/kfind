import type { DocumentGroupIndex } from './types';

import { RoutePath } from '../route-path';

import { localized, page } from './types';

export const referenceGroup: DocumentGroupIndex = {
  labelKey: 'navigation.primary.reference',
  categories: [
    {
      label: localized('CLI', 'CLI'),
      pages: [
        page(
          RoutePath.ReferenceCli,
          'CLI 참조',
          'CLI reference',
          'native CLI와 npm CLI의 명령 형식과 지원 옵션을 나란히 정리합니다.',
          'List command forms and supported options for native and npm CLIs.',
          [
            ['native', 'native CLI', 'Native CLI'],
            ['npm', 'npm CLI', 'npm CLI'],
            ['differences', '기능 차이', 'Feature differences'],
          ],
        ),
        page(
          RoutePath.QueryLanguage,
          'query 언어',
          'Query language',
          'atom, phrase, disjunction, 태그, 인용과 compile 제약의 정확한 문법을 제공합니다.',
          'Specify atoms, phrases, disjunctions, tags, quoting, and compile constraints.',
          [
            ['syntax', '문법', 'Syntax'],
            ['tags', 'atom 태그', 'Atom tags'],
            ['alternatives', '대안', 'Alternatives'],
            ['errors', '구문 오류', 'Syntax errors'],
          ],
        ),
        page(
          RoutePath.PosTags,
          '품사 태그',
          'POS tags',
          '공개 coarse POS와 내부 세부 태그의 대응표를 제공합니다.',
          'Map public coarse POS values to internal detailed tags.',
          [
            ['coarse', 'coarse POS', 'Coarse POS'],
            ['detailed', '세부 태그', 'Detailed tags'],
            ['mapping', '포함 관계', 'Mapping'],
          ],
        ),
        page(
          RoutePath.Configuration,
          '설정',
          'Configuration',
          '설정 파일, 환경 변수와 CLI 우선순위를 정의합니다.',
          'Define configuration files, environment variables, and CLI precedence.',
          [
            ['files', '설정 파일', 'Configuration files'],
            ['environment', '환경 변수', 'Environment variables'],
            ['precedence', '우선순위', 'Precedence'],
          ],
        ),
        page(
          RoutePath.UserLexicon,
          '사용자 사전',
          'User lexicon',
          '프로젝트 표제어, 품사와 surface 교체 규칙의 TSV 형식을 정의합니다.',
          'Define TSV fields for project lemmas, POS, and surface replacements.',
          [
            ['format', '파일 형식', 'File format'],
            ['entries', 'entry 의미', 'Entry semantics'],
            ['validation', '검증', 'Validation'],
          ],
        ),
        page(
          RoutePath.Jsonl,
          'JSON Lines',
          'JSON Lines',
          '자동화 출력의 match, atom, span과 provenance schema를 정의합니다.',
          'Define match, atom, span, and provenance fields in automation output.',
          [
            ['record', 'record', 'Record'],
            ['spans', 'span', 'Spans'],
            ['provenance', 'provenance', 'Provenance'],
          ],
        ),
        page(
          RoutePath.ExitCodes,
          '종료 코드',
          'Exit codes',
          'match, no-match와 실행 실패를 구분하는 종료 상태를 정리합니다.',
          'Distinguish matches, no matches, and execution failures by exit status.',
          [
            ['native', 'native CLI', 'Native CLI'],
            ['npm', 'npm CLI', 'npm CLI'],
            ['pipelines', 'pipeline 사용', 'Pipeline use'],
          ],
        ),
        page(
          RoutePath.Errors,
          '오류 참조',
          'Error reference',
          'query, resource, 입력과 출력 오류의 분류 및 복구 조건을 설명합니다.',
          'Classify query, resource, input, and output errors and their recovery conditions.',
          [
            ['compile', 'compile 오류', 'Compile errors'],
            ['resource', 'resource 오류', 'Resource errors'],
            ['io', 'I/O 오류', 'I/O errors'],
          ],
        ),
      ],
    },
    {
      label: localized('API', 'API'),
      pages: [
        page(
          RoutePath.RustApi,
          'Rust API',
          'Rust API',
          '안정 facade와 expert API의 타입·수명주기 계약을 정리합니다.',
          'Document the type and lifecycle contracts of the stable facade and expert API.',
          [
            ['facade', '안정 facade', 'Stable facade'],
            ['resources', 'resource 초기화', 'Resource initialization'],
            ['expert', 'expert API', 'Expert API'],
          ],
        ),
        page(
          RoutePath.JavaScriptApi,
          'JavaScript API',
          'JavaScript API',
          '@kfind/kfind의 Kfind, Matcher, resource와 UTF-16 span 계약을 정리합니다.',
          'Document Kfind, Matcher, resources, and UTF-16 spans in @kfind/kfind.',
          [
            ['exports', 'package export', 'Package exports'],
            ['engine', 'Kfind와 Matcher', 'Kfind and Matcher'],
            ['spans', 'UTF-16 span', 'UTF-16 spans'],
          ],
        ),
      ],
    },
    {
      label: localized('데이터', 'DATA'),
      pages: [
        page(
          RoutePath.ReferenceResources,
          'resource 참조',
          'Resource reference',
          'full POS, enriched predicate와 compact component 파일의 형식과 호환성을 정리합니다.',
          'Reference formats and compatibility for full-POS, enriched-predicate, and compact-component files.',
          [
            ['profiles', 'resource profile', 'Resource profiles'],
            ['schemas', 'schema', 'Schemas'],
            ['compatibility', '호환성', 'Compatibility'],
          ],
        ),
        page(
          RoutePath.RuleIds,
          '규칙 ID',
          'Rule IDs',
          'match provenance에 나타나는 lexical, ending, particle과 structural ID를 설명합니다.',
          'Interpret lexical, ending, particle, and structural IDs in match provenance.',
          [
            ['namespaces', 'namespace', 'Namespaces'],
            ['composition', '규칙 경로', 'Rule paths'],
            ['stability', '안정성', 'Stability'],
          ],
        ),
        page(
          RoutePath.Glossary,
          '단어장',
          'Glossary',
          '문법 요소, 검색 실행, resource와 품질 지표를 예제와 함께 정의합니다.',
          'Define grammar, execution, resource, and quality terms with examples.',
          [
            ['search', '검색 용어', 'Search terms'],
            ['grammar', '문법 용어', 'Grammar terms'],
            ['execution', '실행 용어', 'Execution terms'],
            ['resource', 'resource 용어', 'Resource terms'],
            ['quality', '품질 지표', 'Quality metrics'],
          ],
        ),
        page(
          RoutePath.Licenses,
          '라이선스',
          'Licenses',
          '코드와 배포 resource에 적용되는 라이선스 및 notice 위치를 정리합니다.',
          'Locate licenses and notices for code and distributed resources.',
          [
            ['code', '코드 라이선스', 'Code license'],
            ['data', '데이터 라이선스', 'Data licenses'],
            ['distribution', '배포 notice', 'Distribution notices'],
          ],
        ),
      ],
    },
  ],
};
