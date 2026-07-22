import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const agentDocuments: TechnicalDocuments = {
  [RoutePath.AgentWorkflow]: {
    [DocumentLocale.Korean]: {
      eyebrow: '에이전트 · 절차',
      title: '에이전트 검색 절차',
      summary:
        '후보 수집, 조건 강화와 문맥 확인을 분리해 누락과 과잉 검색을 추적합니다.',
      sections: [
        section('후보 수집', [
          '요구사항에서 핵심 표제어를 추출하고 `--boundary any --json`으로 넓은 후보를 모읍니다. 파일 범위는 작업 대상 directory 안으로 제한합니다.',
          '초기 결과가 0이면 literal spelling, 품사와 query tokenization을 확인합니다. 바로 동의어를 추가하면 형태 coverage 문제와 어휘 선택 문제를 구분할 수 없습니다.',
        ]),
        section('조건 강화', [
          '후보가 많으면 `smart`, atom 품사 태그와 구 순서를 차례로 적용합니다. 각 단계의 남은 path와 surface를 기록하면 어떤 제약이 후보를 제거했는지 확인할 수 있습니다.',
          '구조 경계가 필요한 query는 compact component resource가 없을 때 명시적으로 실패합니다. 이 실패를 no-match로 처리하지 않습니다.',
        ]),
        section('문맥 확인', [
          '최종 후보의 주변 symbol, call site와 test를 읽어 의미를 판정합니다. kfind provenance는 형태 규칙의 근거이며 프로그램 의미의 근거는 아닙니다.',
          '수정 뒤에는 같은 query와 반대 의미의 hard negative를 함께 실행해 검색 조건이 여전히 유효한지 확인합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'AGENTS · WORKFLOW',
      title: 'Agent search workflow',
      summary:
        'Separate candidate collection, constraint refinement, and context review so omissions and excess results remain observable.',
      sections: [
        section('Candidate search', [
          'Extract the central lemma from the requirement and collect broad candidates with `--boundary any --json`. Limit paths to the directories in scope.',
          'If the initial result is empty, inspect literal spelling, POS, and query tokenization. Adding synonyms immediately would mix morphology coverage with vocabulary choice.',
        ]),
        section('Constraint refinement', [
          'Apply `smart`, atom POS tags, and phrase order one at a time. Retaining paths and surfaces at each step reveals which constraint removed a candidate.',
          'A query requiring structural boundaries fails explicitly without the compact component resource. Do not treat that failure as a no-match result.',
        ]),
        section('Context review', [
          'Read surrounding symbols, call sites, and tests for the final candidates. kfind provenance explains morphological rules, not program semantics.',
          'After an edit, rerun the same query together with an opposite hard negative to verify that the search condition remains valid.',
        ]),
      ],
    },
  },
  [RoutePath.AgentSkills]: {
    [DocumentLocale.Korean]: {
      eyebrow: '에이전트 · skill',
      title: 'skill 설치',
      summary:
        '저장소의 skill 문서는 검색 명령, 결과 해석과 fallback을 하나의 반복 가능한 계약으로 묶습니다.',
      sections: [
        section('설치 위치', [
          '`skills/kfind-search/SKILL.md`와 함께 참조하는 script를 에이전트가 읽을 수 있는 skill directory에 설치합니다. 원본의 상대 경로 관계를 보존합니다.',
          '프로젝트 전용 설치에서는 저장소 revision을 고정합니다. 전역 설치에서는 여러 프로젝트가 같은 binary version을 요구하는지 먼저 확인합니다.',
        ]),
        section('호출 방식', [
          'Skill은 표제어, 예상 품사, 검색 path와 결과 형식을 입력으로 받습니다. 자동화에서는 JSON Lines를 기본으로 사용하고 사람이 확인할 때만 text나 TUI로 전환합니다.',
          '검색 실패와 no-match를 분리하고, component resource 오류가 발생하면 `any`로 조용히 완화하지 않습니다.',
        ]),
        section('갱신', [
          'CLI option, JSON schema 또는 rule provenance 계약이 바뀌면 같은 release의 skill을 함께 갱신합니다. Skill 본문은 현재 명령만 설명하고 revision별 변경 이력은 release와 결정 기록에 둡니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'AGENTS · SKILL',
      title: 'Skill installation',
      summary:
        'The repository skill combines commands, result interpretation, and fallback behavior into a repeatable contract.',
      sections: [
        section('Installation location', [
          'Install `skills/kfind-search/SKILL.md` and its referenced scripts in a skill directory visible to the agent. Preserve relative paths from the source.',
          'Pin the repository revision for a project-local installation. Before a global installation, verify that its projects accept the same binary version.',
        ]),
        section('Invocation', [
          'The skill accepts a lemma, expected POS, search paths, and output form. It defaults to JSON Lines for automation and switches to text or TUI only for human review.',
          'It separates execution failure from no match and never silently relaxes a component-resource failure to `any`.',
        ]),
        section('Maintenance', [
          'Update the skill with the same release whenever CLI options, JSON schema, or rule-provenance contracts change. Keep only current commands in the skill; retain revision history in releases and decision records.',
        ]),
      ],
    },
  },
  [RoutePath.AgentIntegrations]: {
    [DocumentLocale.Korean]: {
      eyebrow: '에이전트 · 환경',
      title: '에이전트별 통합',
      summary:
        '에이전트 제품이 달라도 shell 입력, JSON Lines와 오류 계약은 동일합니다.',
      sections: [
        section('Codex', [
          'Codex 작업에서는 repository `AGENTS.md`와 관련 spec을 읽은 뒤 kfind query를 실행합니다. 출력 path는 현재 workspace 안으로 제한하고, match를 수정 권한으로 해석하지 않습니다.',
        ]),
        section('Claude Code', [
          'Claude Code에서는 project command 또는 skill로 exact command를 보존합니다. Hook에서 실행할 때 no-match 종료 상태를 hook 실패와 구분합니다.',
        ]),
        section('Gemini CLI', [
          'Gemini CLI에서는 JSON Lines를 구조화된 입력으로 전달하고 stdout과 stderr를 분리합니다. 긴 결과는 shell에서 임의 요약하지 않고 path 범위나 query를 좁혀 다시 실행합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'AGENTS · ENVIRONMENTS',
      title: 'Agent integrations',
      summary:
        'Shell input, JSON Lines, and failure contracts stay the same across agent products.',
      sections: [
        section('Codex', [
          'In Codex, read repository `AGENTS.md` and the relevant specification before running a kfind query. Keep output paths inside the current workspace and do not interpret a match as authorization to edit.',
        ]),
        section('Claude Code', [
          'In Claude Code, preserve the exact command in a project command or skill. A hook must distinguish the no-match exit status from hook execution failure.',
        ]),
        section('Gemini CLI', [
          'In Gemini CLI, pass JSON Lines as structured input and keep stdout separate from stderr. Narrow paths or the query instead of arbitrarily summarizing a long result in the shell.',
        ]),
      ],
    },
  },
  [RoutePath.AgentAutomation]: {
    [DocumentLocale.Korean]: {
      eyebrow: '에이전트 · 자동화',
      title: '자동화 패턴',
      summary:
        '기계 소비 출력은 bounded search, JSON Lines와 명시적 실패 처리를 함께 사용합니다.',
      sections: [
        section('JSON Lines pipeline', [
          '각 record는 source path, span, surface와 atom provenance를 독립적으로 포함합니다. 줄 단위 parser는 전체 출력 buffering 없이 record를 처리할 수 있습니다.',
          'Parser는 모르는 field를 무시할 수 있지만 필요한 field의 type 오류를 허용하면 안 됩니다.',
        ]),
        section('결과 상한', [
          '자동화 전에 directory, file type 또는 query atom을 제한합니다. 현재 CLI의 결과 상한을 의미적 sampling으로 대체하지 않습니다.',
          'Native CLI의 bounded stdout과 npm CLI의 결정적 path 순서는 같은 입력의 반복 실행을 비교할 수 있게 합니다.',
        ]),
        section('fallback', [
          'Resource나 compile 오류는 실행 실패입니다. `smart`가 실패했을 때 `any`로 재실행하려면 호출자가 precision 손실을 승인하고 별도 결과로 표시해야 합니다.',
          'No-match는 정상적인 검색 결과입니다. 후속 query를 실행할 수 있지만 원래 query가 성공했다는 사실을 보존합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'AGENTS · AUTOMATION',
      title: 'Automation patterns',
      summary:
        'Machine-readable output combines bounded search, JSON Lines, and explicit failure handling.',
      sections: [
        section('JSON Lines pipeline', [
          'Each record independently contains source path, span, surface, and atom provenance. A line-oriented parser can process it without buffering the full output.',
          'A parser may ignore unknown fields but must reject invalid types for fields it requires.',
        ]),
        section('Result bounds', [
          'Constrain directories, file types, or query atoms before automation. Do not replace the current result set with semantic sampling.',
          'Bounded native output and deterministic npm path ordering make repeated runs on identical input comparable.',
        ]),
        section('Fallback', [
          'Resource and compile errors are execution failures. Retrying `smart` as `any` requires explicit approval of the precision loss and separate labeling of the result.',
          'No match is a normal search result. A follow-up query may run, but the original successful no-match outcome remains part of the record.',
        ]),
      ],
    },
  },
  [RoutePath.AgentContract]: {
    [DocumentLocale.Korean]: {
      eyebrow: '에이전트 · 계약',
      title: '통합 계약',
      summary:
        '에이전트는 query 입력, match schema와 실패 분류에만 의존하고 사람용 표현에는 의존하지 않습니다.',
      sections: [
        section('입력 계약', [
          'Query는 하나 이상의 atom과 선택적 품사 태그로 구성합니다. Path, encoding, boundary와 expansion은 query text 밖의 실행 옵션입니다.',
          '에이전트는 shell quoting이 끝난 실제 argument를 기록합니다. 자연어 설명만 남기면 같은 query를 재현할 수 없습니다.',
        ]),
        section('출력 계약', [
          'Match span은 원문 좌표이며 atom은 core, token과 모든 origin을 보존합니다. `rulePath`는 후보가 생성된 규칙 경로지만 의미 판정 label은 아닙니다.',
          'JSON Lines record의 순서보다 path와 span을 identity로 사용합니다. 병렬 실행 설정에 따라 서로 다른 파일의 완료 순서는 달라질 수 있습니다.',
        ]),
        section('실패 계약', [
          'Compile 오류, 필수 resource 누락, 입력 decode와 I/O 실패는 stderr와 실패 종료 상태로 나타납니다. 정상 no-match와 섞지 않습니다.',
          '부분 결과 뒤에 I/O 실패가 발생할 수 있으므로 호출자는 종료 상태를 확인한 뒤에만 stdout을 완결된 결과로 채택합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'AGENTS · CONTRACT',
      title: 'Integration contract',
      summary:
        'Agents depend on query input, match schema, and failure classes rather than human-facing presentation.',
      sections: [
        section('Input contract', [
          'A query contains one or more atoms with optional POS tags. Paths, encoding, boundary, and expansion are execution options outside the query text.',
          'Record the actual arguments after shell quoting. A natural-language description alone cannot reproduce the query.',
        ]),
        section('Output contract', [
          'A match span uses source coordinates, and each atom preserves core, token, and every origin. `rulePath` identifies candidate-generation rules, not semantic labels.',
          'Use path and span as identity instead of JSON Lines order. Completion order across files can vary with parallel execution settings.',
        ]),
        section('Failure contract', [
          'Compile errors, missing required resources, decode failures, and I/O failures appear on stderr with a failure status. Keep them distinct from a normal no-match result.',
          'An I/O failure can follow partial output, so accept stdout as complete only after checking the final exit status.',
        ]),
      ],
    },
  },
};
