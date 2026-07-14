import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';
import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function GettingStartedPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="GUIDE · GETTING STARTED"
        title="설치하고 첫 검색 실행하기"
        summary="기본 설정은 사람이 직접 검색할 때 정확도를 우선합니다. 품사를 몰라도 표제어를 입력해 시작할 수 있습니다."
      />

      <DocumentSection title="설치">
        <h3>Homebrew</h3>
        <pre>
          <code>brew install seokminhong/brew/kfind</code>
        </pre>
        <h3>현재 checkout에서 빌드</h3>
        <pre>
          <code>cargo install --locked --path crates/kfind-cli</code>
        </pre>
        <Callout title="런타임 네트워크">
          <p>
            CLI는 검색 중에 모델이나 resource를 내려받지 않습니다. Homebrew로
            설치하면 full POS와 component resource도 함께 설치됩니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection
        title="에이전트 skill 설치"
        lead="--init은 검색과 분리된 초기화 모드입니다. 현재 프로젝트에 kfind skill을 설치하며, 검색에 필요한 query와 path는 받지 않습니다."
      >
        <p>
          터미널에서 대상을 지정하지 않고 실행하면 Claude Code, Codex, Gemini
          CLI와 custom 출력 중 하나 이상을 선택할 수 있습니다. 자동화에서는{' '}
          <code>--agent</code>를 반복해서 지정하거나 stdin으로 대상 이름을
          전달합니다.
        </p>
        <pre>
          <code>{`# 대화형 복수 선택
kfind --init

# 재현 가능한 비대화형 설치
kfind --init --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --init

# 다른 에이전트용 SKILL.md 생성
kfind --init --agent custom > path/to/kfind/SKILL.md`}</code>
        </pre>
        <div className="compact-grid">
          <div>
            <strong>Claude Code</strong>
            <code>.claude/skills/kfind</code>
          </div>
          <div>
            <strong>Codex</strong>
            <code>.agents/skills/kfind</code>
          </div>
          <div>
            <strong>Gemini CLI</strong>
            <code>.gemini/skills/kfind</code>
          </div>
          <div>
            <strong>Custom</strong>
            <code>stdout으로 SKILL.md 출력</code>
          </div>
        </div>
        <Callout title="업데이트 방식">
          <p>
            Homebrew 설치는 안정적인 <code>opt/kfind</code> 경로를 연결하므로
            패키지를 업그레이드하면 project skill도 함께 갱신됩니다. source에서
            직접 빌드하거나 Cargo로 설치하면 skill을 project에 복사하므로 새
            버전에서 <code>--init</code>을 다시 실행해야 합니다. kfind 관리
            표식이 없는 기존 skill은 덮어쓰지 않습니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection
        title="첫 표제어 검색"
        lead="경로를 생략하면 pipe로 전달된 stdin을 검색합니다. stdin이 연결되지 않았다면 현재 디렉터리를 검색합니다."
      >
        <pre>
          <code>{`kfind 걷다 src docs
kfind 사용자 .
printf '길을 걸었다.\n' | kfind 걷다`}</code>
        </pre>
        <ol className="steps">
          <li>
            <strong>표제어를 분석합니다.</strong>
            <p>
              <code>걷다</code>를 동사와 ㄷ 불규칙 활용으로 해석합니다.
            </p>
          </li>
          <li>
            <strong>검색 계획을 만듭니다.</strong>
            <p>
              <code>걷고</code>, <code>걷는</code>, <code>걸어</code> 같은
              anchor와 verifier를 구성합니다.
            </p>
          </li>
          <li>
            <strong>검증된 span을 출력합니다.</strong>
            <p>파일명, 줄과 일치 표면형을 일반 검색 도구처럼 보여 줍니다.</p>
          </li>
        </ol>
      </DocumentSection>

      <DocumentSection title="품사를 명확히 지정하기">
        <p>
          중의어이거나 사전에 없는 표제어는 전역 <code>--pos</code> 또는 atom
          태그로 품사를 지정합니다. phrase query에서는 atom마다 다른 품사를
          지정할 수 있습니다.
        </p>
        <pre>
          <code>{`kfind --pos verb 걷다 src
kfind 'n:사용자 v:검증하다' src
kfind 'det:새 n:기능' docs
kfind '"Hello, world!"' README.md`}</code>
        </pre>
        <div className="tag-list" aria-label="지원하는 쿼리 품사 태그">
          {[
            'n: noun',
            'pro: pronoun',
            'num: numeral',
            'v: verb',
            'adj: adjective',
            'det: determiner',
            'adv: adverb',
            'j: particle',
            'intj: interjection',
            'lit: literal',
          ].map((tag) => (
            <code key={tag}>{tag}</code>
          ))}
        </div>
      </DocumentSection>

      <DocumentSection title="자동화에서 사용하기">
        <p>
          에이전트는 품사를 명시하고 <code>any</code>, embedded lexicon, JSON
          Lines를 함께 사용합니다. 이 조합은 recall을 우선하므로 false
          positive가 포함될 수 있습니다. 결과 주변의 문맥을 읽고 이를 제거하는
          단계까지 자동화에 포함해야 합니다.
        </p>
        <pre>
          <code>{`kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src`}</code>
        </pre>
        <p className="next-link">
          다음:{' '}
          <Link to={RoutePath.Options}>확장과 경계 옵션을 자세히 비교하기</Link>
        </p>
      </DocumentSection>
    </article>
  );
}
