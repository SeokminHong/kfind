import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.Agents);

const sectionIds = [
  'search-primitive',
  'recommended-workflow',
  'skill-installation',
  'supported-agents',
  'automation-patterns',
  'integration-contract',
] as const;

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '안내 · 에이전트',
    title: '코딩 에이전트 통합',
    summary:
      '형태 검색을 반복 추론에서 분리하고, 명시적 품사와 JSON Lines를 사용해 Codex, Claude Code와 Gemini CLI에 결정적인 검색 결과를 제공합니다.',
    sections: [
      {
        title: '검색 primitive',
        body: (
          <>
            <p>
              일반 문자열 검색은 에이전트가 활용형을 직접 열거하게 합니다.{' '}
              <code>걷다</code>를 찾을 때 <code>걷고</code>, <code>걸어</code>,{' '}
              <code>걸었다</code>를 매번 추론하면 질의마다 회수 범위가
              달라집니다. kfind는 표제어와 품사를 하나의 검색 계획으로 컴파일해
              같은 입력에 같은 후보와 생성 근거를 반환합니다.
            </p>
            <p>
              kfind는 문장 의미를 선택하지 않습니다. 에이전트는 넓게 회수한 span
              주변의 코드를 읽고 작업 목적에 맞는 후보를 선택합니다. 형태 생성과
              문맥 판단의 책임을 분리하면 검색 결과를 재현하면서도 에이전트의
              의미 판단을 유지할 수 있습니다.
            </p>
          </>
        ),
      },
      {
        title: '권장 사용 절차',
        body: (
          <>
            <ol className="steps">
              <li>
                <strong>검색 대상과 품사</strong>
                <p>
                  파일 경로를 먼저 제한하고 각 atom에 <code>n:</code>,{' '}
                  <code>v:</code> 같은 품사 태그를 지정합니다.
                </p>
              </li>
              <li>
                <strong>회수 우선 경계</strong>
                <p>
                  <code>--embedded --boundary any</code>로 리소스 초기화와 구조
                  거부를 피하고 형태 후보를 수집합니다.
                </p>
              </li>
              <li>
                <strong>구조화된 결과</strong>
                <p>
                  <code>--json</code>의 UTF-8 span, 품사와 rule provenance를
                  읽어 후속 도구에 전달합니다.
                </p>
              </li>
              <li>
                <strong>문맥 검토</strong>
                <p>
                  결과 줄과 인접 코드를 읽고 의미가 다른 동형이의어를
                  제외합니다.
                </p>
              </li>
            </ol>
            <pre>
              <code>{`kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
kfind --embedded --boundary any --pos verb --json 걷다 crates`}</code>
            </pre>
          </>
        ),
      },
      {
        title: '통합 설치',
        body: (
          <>
            <p>
              <code>kfind --init</code>은 현재 프로젝트에 에이전트별{' '}
              <code>SKILL.md</code>와 shell hook을 설치합니다. TTY에서는 대상을
              선택하고, 자동화에서는 <code>--agent</code>를 반복하거나 stdin으로
              대상 이름을 전달합니다. kfind 관리 표식이 없는 skill과 기존 agent
              설정은 보존합니다.
            </p>
            <pre>
              <code>{`kfind --init
kfind --init --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --init`}</code>
            </pre>
            <p>
              Homebrew 설치본의 skill은 versioned Cellar가 아니라 안정적인{' '}
              <code>opt</code> 경로를 가리킵니다. <code>brew upgrade</code>{' '}
              뒤에는 프로젝트 link를 다시 만들지 않아도 새 릴리즈의 지침을
              사용합니다.
            </p>
            <p>
              Project hook은 각 에이전트의 신뢰 절차를 통과한 뒤 동작합니다.
              Codex에서는 <code>/hooks</code>로 검토하고 신뢰합니다. Hook은 한글
              검색 pattern을 받은 <code>rg</code>·<code>grep</code> 계열과{' '}
              <code>git grep</code>을 차단하고 kfind로 다시 검색하도록
              안내합니다.
            </p>
          </>
        ),
      },
      {
        title: '지원 에이전트',
        body: (
          <>
            <p>
              세 통합은 같은 검색 계약을 사용합니다. 에이전트는 저장소 안의
              skill을 읽고, project hook은 literal shell 검색을 실행 전에
              검사합니다.
            </p>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">대상</th>
                    <th scope="col">설치 값</th>
                    <th scope="col">Skill 경로</th>
                    <th scope="col">Hook 설정</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Codex</th>
                    <td>
                      <code>codex</code>
                    </td>
                    <td>
                      <code>.agents/skills/kfind/SKILL.md</code>
                    </td>
                    <td>
                      <code>.codex/hooks.json</code>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Claude Code</th>
                    <td>
                      <code>claude-code</code>
                    </td>
                    <td>
                      <code>.claude/skills/kfind/SKILL.md</code>
                    </td>
                    <td>
                      <code>.claude/settings.json</code>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Gemini CLI</th>
                    <td>
                      <code>gemini</code>
                    </td>
                    <td>
                      <code>.gemini/skills/kfind/SKILL.md</code>
                    </td>
                    <td>
                      <code>.gemini/settings.json</code>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </>
        ),
      },
      {
        title: '자동화 패턴',
        body: (
          <>
            <p>
              금지 표현 검사는 <code>--quiet</code>의 종료 코드로 결과 존재
              여부를 판정할 수 있습니다. 리팩터링 후보 수집은 JSON Lines를
              유지하고, 경로와 span을 기준으로 중복을 제거합니다. 여러 표제어의
              순서가 중요하면 별도 명령을 합치는 대신 phrase query와{' '}
              <code>--max-gap</code>을 사용합니다.
            </p>
            <pre>
              <code>{`kfind --pos verb --quiet 사용하다 docs && exit 1
kfind --embedded --boundary any --json 'n:권한 v:검증하다' src \
  | jq -c 'select(.type == "match")'`}</code>
            </pre>
            <p>
              결과가 너무 많으면 먼저 검색 경로와 glob을 줄입니다. 의미 판별을
              기대해 경계 정책을 임의로 바꾸지 않으며, 구조 정밀도가 필요한
              사람용 검색에서는 full POS와 <code>smart</code>를 별도 실행합니다.
            </p>
          </>
        ),
      },
      {
        title: '통합 계약',
        body: (
          <>
            <p>
              검색 결과는 stdout, 진단과 오류는 stderr에 기록합니다. 일치가
              있으면 종료 코드 0, 일치가 없으면 1, 사용법·입력·리소스 오류는 2를
              반환합니다. JSON Lines의 각 match record는 경로, 줄, 원문, UTF-8
              byte span과 생성 근거를 포함합니다.
            </p>
            <p>
              대규모 출력은 파일과 glob으로 먼저 제한합니다. TTY pager가 필요한
              사람용 출력과 달리 <code>--json</code>, <code>--count</code>,{' '}
              <code>--quiet</code>는 비대화형 stream을 유지합니다. 에이전트는
              사람이 읽는 출력 문구를 파싱하지 않고 JSON field와 종료 코드만
              사용합니다.
            </p>
          </>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'GUIDE · AGENTS',
    title: 'Coding-agent integration',
    summary:
      'Move morphology search out of repeated model reasoning and give Codex, Claude Code, and Gemini CLI deterministic results through explicit POS queries and JSON Lines.',
    sections: [
      {
        title: 'Search primitive',
        body: (
          <>
            <p>
              Literal search makes an agent enumerate inflections for every
              task. A query for <code>걷다</code> may need <code>걷고</code>,{' '}
              <code>걸어</code>, and <code>걸었다</code>. kfind compiles the
              lemma and POS into one plan, so identical input produces the same
              candidates and rule provenance.
            </p>
            <p>
              kfind does not select a word sense. The agent reads the code
              around each broadly recalled span and keeps the candidates
              relevant to the task. This boundary makes morphology reproducible
              without removing contextual judgment from the agent.
            </p>
          </>
        ),
      },
      {
        title: 'Recommended workflow',
        body: (
          <>
            <ol className="steps">
              <li>
                <strong>Scope and POS</strong>
                <p>
                  Narrow the file paths and tag each atom with <code>n:</code>,{' '}
                  <code>v:</code>, or another explicit POS.
                </p>
              </li>
              <li>
                <strong>Recall-first boundary</strong>
                <p>
                  Use <code>--embedded --boundary any</code> to avoid resource
                  startup and structural rejection.
                </p>
              </li>
              <li>
                <strong>Structured output</strong>
                <p>
                  Read UTF-8 spans, POS, and rule provenance from{' '}
                  <code>--json</code> output.
                </p>
              </li>
              <li>
                <strong>Context review</strong>
                <p>Inspect adjacent code and discard irrelevant homographs.</p>
              </li>
            </ol>
            <pre>
              <code>{`kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
kfind --embedded --boundary any --pos verb --json 걷다 crates`}</code>
            </pre>
          </>
        ),
      },
      {
        title: 'Integration installation',
        body: (
          <>
            <p>
              <code>kfind --init</code> installs an agent-specific{' '}
              <code>SKILL.md</code> and shell hook in the current project.
              Interactive terminals offer a target picker. Automation repeats{' '}
              <code>--agent</code> or supplies target names on stdin. Unmanaged
              skills and existing agent settings are preserved.
            </p>
            <pre>
              <code>{`kfind --init
kfind --init --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --init`}</code>
            </pre>
            <p>
              A Homebrew-managed skill points to the stable <code>opt</code>{' '}
              path instead of a versioned Cellar. Existing project links
              therefore use the new guidance after <code>brew upgrade</code>.
            </p>
            <p>
              Project hooks run after each agent&apos;s trust review. In Codex,
              inspect and trust them with <code>/hooks</code>. The hook blocks{' '}
              <code>rg</code>, <code>grep</code> variants, and{' '}
              <code>git grep</code> when their search pattern contains Korean
              text, then directs the agent to kfind.
            </p>
          </>
        ),
      },
      {
        title: 'Supported agents',
        body: (
          <>
            <p>
              All three integrations use the same search contract. Each agent
              reads the repository-local skill, while the project hook checks
              literal shell searches before execution.
            </p>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">Target</th>
                    <th scope="col">Install value</th>
                    <th scope="col">Skill path</th>
                    <th scope="col">Hook configuration</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th scope="row">Codex</th>
                    <td>
                      <code>codex</code>
                    </td>
                    <td>
                      <code>.agents/skills/kfind/SKILL.md</code>
                    </td>
                    <td>
                      <code>.codex/hooks.json</code>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Claude Code</th>
                    <td>
                      <code>claude-code</code>
                    </td>
                    <td>
                      <code>.claude/skills/kfind/SKILL.md</code>
                    </td>
                    <td>
                      <code>.claude/settings.json</code>
                    </td>
                  </tr>
                  <tr>
                    <th scope="row">Gemini CLI</th>
                    <td>
                      <code>gemini</code>
                    </td>
                    <td>
                      <code>.gemini/skills/kfind/SKILL.md</code>
                    </td>
                    <td>
                      <code>.gemini/settings.json</code>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </>
        ),
      },
      {
        title: 'Automation patterns',
        body: (
          <>
            <p>
              A forbidden-expression check can use the <code>--quiet</code> exit
              status. Refactoring discovery retains JSON Lines and deduplicates
              by path and span. When lemma order matters, use a phrase query and{' '}
              <code>--max-gap</code> instead of merging separate commands.
            </p>
            <pre>
              <code>{`kfind --pos verb --quiet 사용하다 docs && exit 1
kfind --embedded --boundary any --json 'n:권한 v:검증하다' src \
  | jq -c 'select(.type == "match")'`}</code>
            </pre>
            <p>
              Reduce the path and glob before changing the search policy when
              output is large. A separate full-POS <code>smart</code> search is
              available when a human workflow needs structural precision.
            </p>
          </>
        ),
      },
      {
        title: 'Integration contract',
        body: (
          <>
            <p>
              Results go to stdout; diagnostics and errors go to stderr. Exit
              status 0 means at least one match, 1 means no match, and 2 reports
              usage, input, or resource errors. Every JSON Lines match record
              contains the path, line, source text, UTF-8 byte spans, and
              provenance.
            </p>
            <p>
              Bound large output by path and glob first. Unlike the interactive
              TTY pager, <code>--json</code>, <code>--count</code>, and{' '}
              <code>--quiet</code> remain non-interactive streams. Agents parse
              JSON fields and exit statuses, never localized human-readable
              output.
            </p>
          </>
        ),
      },
    ],
  },
};

export default function AgentsPage(): React.JSX.Element {
  return <LocalizedDocument content={content} sectionIds={sectionIds} />;
}
