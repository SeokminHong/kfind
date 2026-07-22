import type { DocumentGroupIndex } from './types';

import { RoutePath } from '../route-path';

import { localized, page } from './types';

export const agentsGroup: DocumentGroupIndex = {
  labelKey: 'navigation.primary.agents',
  categories: [
    {
      label: localized('통합', 'INTEGRATION'),
      pages: [
        page(
          RoutePath.Agents,
          '에이전트 개요',
          'Agent overview',
          '에이전트가 한국어 요구사항을 코드 위치로 좁히는 검색 primitive를 설명합니다.',
          'Use kfind as a search primitive that maps Korean requirements to code locations.',
          [
            ['search-primitive', '검색 primitive', 'Search primitive'],
            ['recommended-workflow', '권장 절차', 'Recommended workflow'],
            ['skill-installation', 'skill 설치', 'Skill installation'],
            ['supported-agents', '지원 에이전트', 'Supported agents'],
            ['automation-patterns', '자동화 패턴', 'Automation patterns'],
            ['integration-contract', '통합 계약', 'Integration contract'],
          ],
        ),
        page(
          RoutePath.AgentWorkflow,
          '에이전트 검색 절차',
          'Agent workflow',
          '넓은 후보 수집, 구조 경계 강화와 주변 문맥 확인을 단계로 분리합니다.',
          'Separate broad candidate collection, structural narrowing, and local context review.',
          [
            ['candidate-search', '후보 수집', 'Candidate search'],
            ['constraint-refinement', '조건 강화', 'Constraint refinement'],
            ['context-review', '문맥 확인', 'Context review'],
          ],
        ),
        page(
          RoutePath.AgentSkills,
          'skill 설치',
          'Skill installation',
          '저장소 skill의 설치 위치, 호출 계약과 갱신 범위를 정의합니다.',
          'Install the repository skill and preserve its invocation and update contract.',
          [
            ['installation', '설치 위치', 'Installation location'],
            ['invocation', '호출 방식', 'Invocation'],
            ['maintenance', '갱신', 'Maintenance'],
          ],
        ),
        page(
          RoutePath.AgentIntegrations,
          '에이전트별 통합',
          'Agent integrations',
          'Codex, Claude Code와 Gemini CLI에서 같은 검색 계약을 적용합니다.',
          'Apply the same search contract in Codex, Claude Code, and Gemini CLI.',
          [
            ['codex', 'Codex', 'Codex'],
            ['claude-code', 'Claude Code', 'Claude Code'],
            ['gemini-cli', 'Gemini CLI', 'Gemini CLI'],
          ],
        ),
      ],
    },
    {
      label: localized('자동화', 'AUTOMATION'),
      pages: [
        page(
          RoutePath.AgentAutomation,
          '자동화 패턴',
          'Automation patterns',
          'JSON Lines, 종료 코드와 bounded output을 조합한 실행 패턴을 설명합니다.',
          'Compose JSON Lines, exit status, and bounded output into reliable automation.',
          [
            ['jsonl-pipeline', 'JSON Lines pipeline', 'JSON Lines pipeline'],
            ['result-bounds', '결과 상한', 'Result bounds'],
            ['fallback', 'fallback', 'Fallback'],
          ],
        ),
        page(
          RoutePath.AgentContract,
          '통합 계약',
          'Integration contract',
          '에이전트가 의존할 수 있는 입력, 출력, 오류와 provenance의 안정 범위를 정리합니다.',
          'Define stable input, output, error, and provenance surfaces for agent integrations.',
          [
            ['input-contract', '입력 계약', 'Input contract'],
            ['output-contract', '출력 계약', 'Output contract'],
            ['failure-contract', '실패 계약', 'Failure contract'],
          ],
        ),
      ],
    },
  ],
};
