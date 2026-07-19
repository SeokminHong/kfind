import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.GettingStarted);

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '안내 · 시작',
    title: '설치와 첫 검색',
    summary:
      'CLI, Rust와 npm 환경에서 kfind를 설치하고 표제어 검색, 품사 지정, 자동화 출력을 실행하는 최소 절차입니다.',
    sections: [
      {
        title: 'CLI 설치',
        body: (
          <>
            <p>
              macOS와 Linux에서는 Homebrew formula가 CLI와 같은 릴리즈의 형태
              리소스를 함께 설치합니다. source checkout은 Cargo로 설치할 수
              있습니다.
            </p>
            <pre>
              <code>{`brew install seokminhong/brew/kfind

# source checkout
cargo install --locked --path crates/kfind-cli

kfind --version`}</code>
            </pre>
            <p>
              검색 중에는 네트워크를 사용하지 않습니다. Homebrew 설치본은 full
              POS lexicon, 용언 metadata와 component resource를 formula 경로에서
              찾습니다. 리소스 schema나 source digest가 실행 파일과 맞지 않으면
              검색을 시작하지 않고 오류를 반환합니다.
            </p>
          </>
        ),
      },
      {
        title: 'npm 설치',
        body: (
          <>
            <p>
              JavaScript와 TypeScript에서는 <code>kfind</code> 패키지를
              설치합니다. 패키지는 WebAssembly 엔진과 기본 lexicon을 포함하며,
              파일 시스템이나 URL을 임의로 읽지 않습니다.
            </p>
            <pre>
              <code>npm install kfind@1.0.0-rc.1</code>
            </pre>
            <pre>
              <code>{`import { Kfind } from 'kfind';

const engine = new Kfind();
const plan = engine.compile('걷다', { pos: 'verb' });
const matches = plan.findAll('나는 길을 걸었다.');`}</code>
            </pre>
            <p>
              브라우저와 Node.js는 동일한 query compile과 memory-text match
              의미를 사용합니다. 전체 품사 사전이나 component 판정이 필요한
              호출은 패키지가 공개하는 asset을 읽어 bytes로 전달해야 합니다.
            </p>
          </>
        ),
      },
      {
        title: '표제어 검색',
        body: (
          <>
            <p>
              첫 번째 위치 인자는 검색 질의이고, 뒤의 인자는 검색할 파일이나
              디렉터리입니다. 경로가 없으면 pipe 입력을 검색합니다. 대화형
              터미널에서 stdin도 없으면 현재 디렉터리를 검색합니다.
            </p>
            <pre>
              <code>{`kfind 걷다 src docs
kfind 사용자 .
printf '길을 걸었다.\n' | kfind 걷다`}</code>
            </pre>
            <p>
              <code>걷다</code>는 동사와 ㄷ 불규칙 활용으로 분석됩니다. 검색
              계획은 <code>걷고</code>, <code>걷는</code>, <code>걸어</code>와
              같은 후보의 고정 anchor와 suffix 판정 조건을 만들고, 원문에서는 그
              조건을 통과한 span만 반환합니다.
            </p>
          </>
        ),
      },
      {
        title: '품사와 구 검색',
        body: (
          <>
            <p>
              사전 분석이 중의적이거나 입력의 문법 역할을 알고 있다면
              <code>--pos</code>를 지정합니다. 구 검색은 각 atom 앞에 품사
              태그를 붙입니다. 따옴표로 묶은 atom은 공백과 문장부호를 포함한
              literal로 처리됩니다.
            </p>
            <pre>
              <code>{`kfind --pos verb 걷다 src
kfind 'n:사용자 v:검증하다' src
kfind 'det:새 n:기능' docs
kfind '"Hello, world!"' README.md`}</code>
            </pre>
            <p>
              atom 태그는 <code>n:</code>, <code>pro:</code>, <code>num:</code>,{' '}
              <code>v:</code>, <code>adj:</code>, <code>det:</code>,{' '}
              <code>adv:</code>, <code>j:</code>, <code>intj:</code>,{' '}
              <code>lit:</code>을 지원합니다. 전역 품사와 atom 품사가 다르면
              query compile 오류가 발생합니다.
            </p>
          </>
        ),
      },
      {
        title: '자동화 출력',
        body: (
          <>
            <p>
              자동화에서는 품사, 경계 정책, 리소스와 출력 형식을 명시합니다.
              <code>any</code>는 부분 문자열 후보를 보존해 recall을 우선하고,
              JSON Lines는 원문 span과 provenance를 안정적으로 제공합니다.
            </p>
            <pre>
              <code>{`kfind --embedded --boundary any --pos verb --json 걷다 src
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src`}</code>
            </pre>
            <p>
              recall 우선 결과에는 의미상 맞지 않는 후보가 포함될 수 있습니다.
              호출자는 span 주변의 원문을 읽어 작업 목적에 맞는 결과인지
              확인해야 합니다.
            </p>
          </>
        ),
      },
      {
        title: '에이전트 skill',
        body: (
          <>
            <p>
              <code>--init</code>은 지원하는 코딩 에이전트에 kfind 사용법을 담은
              skill을 설치합니다. 검색 질의나 경로와 함께 사용할 수 없습니다.
            </p>
            <pre>
              <code>{`kfind --init
kfind --init --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --init`}</code>
            </pre>
            <p>
              Codex는 <code>.agents/skills/kfind/SKILL.md</code>, Claude Code는{' '}
              <code>.claude/skills/kfind/SKILL.md</code>, Gemini CLI는{' '}
              <code>.gemini/skills/kfind/SKILL.md</code>를 사용합니다. kfind
              관리 표식이 없는 파일은 덮어쓰지 않습니다.
            </p>
          </>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'GUIDE · GETTING STARTED',
    title: 'Installation and first search',
    summary:
      'The minimum setup for lemma search, explicit POS queries, and machine-readable output from the CLI, Rust, and npm surfaces.',
    sections: [
      {
        title: 'CLI installation',
        body: (
          <>
            <p>
              On macOS and Linux, the Homebrew formula installs the CLI and the
              morphology resources paired with the same release. A source
              checkout can be installed through Cargo.
            </p>
            <pre>
              <code>{`brew install seokminhong/brew/kfind

# source checkout
cargo install --locked --path crates/kfind-cli

kfind --version`}</code>
            </pre>
            <p>
              Search performs no network access. The Homebrew build discovers
              its full-POS lexicon, predicate metadata, and component resource
              under the formula prefix. A schema or source-digest mismatch is a
              startup error.
            </p>
          </>
        ),
      },
      {
        title: 'npm installation',
        body: (
          <>
            <p>
              JavaScript and TypeScript applications install the unscoped{' '}
              <code>kfind</code> package. It includes the WebAssembly engine and
              embedded lexicon and never guesses a filesystem path or URL.
            </p>
            <pre>
              <code>npm install kfind@1.0.0-rc.1</code>
            </pre>
            <pre>
              <code>{`import { Kfind } from 'kfind';

const engine = new Kfind();
const plan = engine.compile('걷다', { pos: 'verb' });
const matches = plan.findAll('나는 길을 걸었다.');`}</code>
            </pre>
          </>
        ),
      },
      {
        title: 'Lemma search',
        body: (
          <>
            <p>
              The first positional argument is the query. Remaining arguments
              are files or directories. With no path, kfind reads piped stdin;
              with neither a path nor piped input, it searches the current
              directory.
            </p>
            <pre>
              <code>{`kfind 걷다 src docs
kfind 사용자 .
printf '길을 걸었다.\n' | kfind 걷다`}</code>
            </pre>
            <p>
              The lemma <code>걷다</code> is analyzed as a verb with
              ㄷ-irregular conjugation. Its query plan contains fixed anchors
              and suffix decisions for surfaces such as <code>걷고</code>,{' '}
              <code>걷는</code>, and <code>걸어</code>.
            </p>
          </>
        ),
      },
      {
        title: 'POS and phrase queries',
        body: (
          <>
            <p>
              Use <code>--pos</code> when the lexicon analysis is ambiguous or
              the grammatical role is known. Phrase queries attach a POS tag to
              each atom. A quoted atom is treated as a literal containing spaces
              or punctuation.
            </p>
            <pre>
              <code>{`kfind --pos verb 걷다 src
kfind 'n:사용자 v:검증하다' src
kfind 'det:새 n:기능' docs
kfind '"Hello, world!"' README.md`}</code>
            </pre>
            <p>
              Supported tags are <code>n:</code>, <code>pro:</code>,{' '}
              <code>num:</code>, <code>v:</code>, <code>adj:</code>,{' '}
              <code>det:</code>, <code>adv:</code>, <code>j:</code>,{' '}
              <code>intj:</code>, and <code>lit:</code>. Conflicting global and
              atom POS values are compile errors.
            </p>
          </>
        ),
      },
      {
        title: 'Automation output',
        body: (
          <>
            <p>
              Automation should specify POS, boundary, resource mode, and output
              format. The <code>any</code> boundary preserves substring
              candidates for recall, while JSON Lines exposes stable source
              spans and provenance.
            </p>
            <pre>
              <code>{`kfind --embedded --boundary any --pos verb --json 걷다 src
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src`}</code>
            </pre>
            <p>
              Recall-oriented output can include semantically irrelevant
              candidates. Read the surrounding source text before acting on a
              match.
            </p>
          </>
        ),
      },
      {
        title: 'Agent skill',
        body: (
          <>
            <p>
              <code>--init</code> installs kfind guidance for supported coding
              agents and cannot be combined with a search query or path.
            </p>
            <pre>
              <code>{`kfind --init
kfind --init --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --init`}</code>
            </pre>
            <p>
              Codex uses <code>.agents/skills/kfind/SKILL.md</code>, Claude Code
              uses <code>.claude/skills/kfind/SKILL.md</code>, and Gemini CLI
              uses <code>.gemini/skills/kfind/SKILL.md</code>. Files without the
              kfind management marker are not overwritten.
            </p>
          </>
        ),
      },
    ],
  },
};

export default function GettingStartedPage(): React.JSX.Element {
  return <LocalizedDocument content={content} />;
}
