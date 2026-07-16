import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';
import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';

export default function GettingStartedPage(): React.JSX.Element {
  return (
    <DocumentPage>
      <PageIntro
        eyebrow="GUIDE · GETTING STARTED"
        title="설치하고 첫 검색 실행하기"
        summary="기본 CLI는 사람이 직접 검색할 때 precision과 사용성을 우선합니다. 품사를 몰라도 표제어와 검색 경로만 입력해 시작할 수 있으며, 자동화에서는 품사와 출력 형식을 명시해 같은 엔진을 더 엄격한 계약으로 호출할 수 있습니다."
      />

      <DocumentSection title="설치">
        <p>
          macOS와 Linux에서는 Homebrew formula로 CLI와 런타임 resource를 함께
          설치할 수 있습니다. 현재 source checkout을 시험하려면 Cargo로 CLI
          crate를 직접 설치합니다.
        </p>
        <pre>
          <code>{`# Homebrew
brew install seokminhong/brew/kfind

# 현재 checkout
cargo install --locked --path crates/kfind-cli`}</code>
        </pre>
        <p>
          CLI는 검색 도중에 모델이나 resource를 내려받지 않습니다. Homebrew
          formula에는 자동 품사 분석용 full POS lexicon, enriched 용언
          metadata와 필요한 후보의 구조만 국소 판정하는 component resource가
          포함됩니다. source에서 직접 설치한 경우에는 resource를 별도로 지정하지
          않아도 embedded lexicon으로 검색할 수 있지만, 추가 resource가 필요한
          query는 해당 resource가 있는 data directory를 요구합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="첫 표제어 검색">
        <p>
          명령의 첫 번째 위치 인자는 query이고, 그 뒤에는 검색할 파일이나
          디렉터리를 지정합니다. 경로를 생략하면 pipe로 받은 stdin을 검색하며,
          stdin이 연결되지 않은 대화형 실행에서는 현재 디렉터리를 검색합니다.
        </p>
        <pre>
          <code>{`kfind 걷다 src docs
kfind 사용자 .
printf '길을 걸었다.\n' | kfind 걷다`}</code>
        </pre>
        <p>
          첫 번째 명령에서 kfind는 <code>걷다</code>를 동사와 ㄷ 불규칙 활용으로
          분석합니다. 그 분석에서 <code>걷고</code>, <code>걷는</code>,{' '}
          <code>걸어</code>와 같은 anchor, suffix consumption과 판정 제약을 가진
          candidate program을 구성하고, 지정된 경로에서 판정을 통과한 span만
          출력합니다. 기본 출력은 일반 검색 도구처럼 파일명, 줄 번호와 일치한
          표면형을 보여 줍니다.
        </p>
      </DocumentSection>

      <DocumentSection title="품사와 phrase를 명시하기">
        <p>
          중의어이거나 사전에 없는 표제어는 전역 <code>--pos</code>로 품사를
          지정할 수 있습니다. 여러 atom으로 이루어진 phrase에서는 각 atom 앞에
          태그를 붙여 서로 다른 품사를 표현합니다. 따옴표로 묶은 atom은 내부
          공백과 문장부호를 포함한 literal로 처리합니다.
        </p>
        <pre>
          <code>{`kfind --pos verb 걷다 src
kfind 'n:사용자 v:검증하다' src
kfind 'det:새 n:기능' docs
kfind '"Hello, world!"' README.md`}</code>
        </pre>
        <p>
          태그는 <code>n:</code> noun, <code>pro:</code> pronoun,{' '}
          <code>num:</code> numeral, <code>v:</code> verb, <code>adj:</code>{' '}
          adjective, <code>det:</code> determiner, <code>adv:</code> adverb,{' '}
          <code>j:</code> particle, <code>intj:</code> interjection과{' '}
          <code>lit:</code> literal을 지원합니다. 전역 <code>--pos</code>와 atom
          태그를 함께 쓰면 두 값이 같아야 하며, 다르면 query compile 오류를
          반환합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="에이전트 skill 초기화">
        <p>
          <code>--init</code>은 검색과 분리된 실행 모드입니다. 현재 프로젝트에
          kfind 사용법을 담은 agent skill을 설치하며, 이 모드에는 검색 query,
          path 또는 검색 옵션을 함께 전달할 수 없습니다. 대화형 터미널에서{' '}
          <code>kfind --init</code>만 실행하면 지원하는 agent를 복수 선택할 수
          있습니다. 자동화에서는 <code>--agent</code>를 반복하거나 stdin으로
          대상 이름을 전달합니다.
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
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">대상</th>
                <th scope="col">프로젝트 경로 또는 출력</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Claude Code</td>
                <td>
                  <code>.claude/skills/kfind/SKILL.md</code>
                </td>
              </tr>
              <tr>
                <td>Codex</td>
                <td>
                  <code>.agents/skills/kfind/SKILL.md</code>
                </td>
              </tr>
              <tr>
                <td>Gemini CLI</td>
                <td>
                  <code>.gemini/skills/kfind/SKILL.md</code>
                </td>
              </tr>
              <tr>
                <td>Custom</td>
                <td>
                  <code>SKILL.md</code> 원문을 stdout으로 출력
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          Homebrew 설치는 안정적인 <code>opt/kfind</code> 경로를 연결하므로
          formula를 업그레이드하면 연결된 project skill도 새 배포본을
          참조합니다. source나 Cargo 설치에서는 skill을 프로젝트로 복사하므로
          버전을 올린 뒤 <code>--init</code>을 다시 실행해야 합니다. 대화형
          선택을 취소하거나 대상을 고르지 않으면 파일을 만들지 않으며, kfind
          관리 표식이 없는 기존 skill은 덮어쓰지 않습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="자동화에서 검색하기">
        <p>
          에이전트는 각 형태 atom에 품사를 명시하고 <code>any</code> boundary,
          embedded lexicon과 JSON Lines 출력을 함께 사용하는 것이 기본
          경로입니다. 품사를 미리 제공하면 자동 품사 추론의 중의성을 제거할 수
          있고, <code>any</code>는 경계 때문에 발생하는 누락을 줄입니다.
          embedded lexicon은 설치 환경과 무관한 시작 비용을 제공하며, JSON
          Lines는 span과 provenance를 안정적으로 파싱하게 합니다.
        </p>
        <pre>
          <code>{`kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src`}</code>
        </pre>
        <p>
          이 조합은 recall을 우선하므로 false positive가 포함될 수 있습니다.
          자동화는 검색 결과만으로 의미를 확정하지 말고, 각 span 주변의 원문
          문맥을 읽어 작업 목적에 맞는 후보인지 확인해야 합니다. 확장 수준과
          boundary가 후보 집합을 어떻게 바꾸는지는{' '}
          <Link to={RoutePath.Options}>쿼리와 옵션</Link>에서 이어서 설명합니다.
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}
