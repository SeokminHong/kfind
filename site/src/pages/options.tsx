import { SplitDiagram } from '../components/diagram';
import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function OptionsPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="REFERENCE · QUERY COMPILATION"
        title="쿼리와 옵션"
        summary="확장 수준, 품사, 경계와 Unicode 정책은 서로 다른 축입니다. 각 축이 어떤 후보를 만들고 어떤 후보를 버리는지 구분해 선택합니다."
      >
        <div className="defaults-strip" aria-label="기본 컴파일 옵션">
          <span>
            <code>expand=inflection</code>
          </span>
          <span>
            <code>boundary=smart</code>
          </span>
          <span>
            <code>pos=auto</code>
          </span>
          <span>
            <code>normalization=nfc</code>
          </span>
          <span>
            <code>max-gap=24</code>
          </span>
        </div>
      </PageIntro>

      <DocumentSection
        title="확장 수준"
        lead="--expand는 표제어에서 어떤 종류의 search branch를 만들지 결정합니다."
      >
        <div className="option-card-grid">
          <article className="option-card">
            <header>
              <code>literal</code>
              <span>정확한 표면형</span>
            </header>
            <p>
              입력 문자열 하나만 branch로 만듭니다. 활용, 조사, 파생을 만들지
              않지만 선택한 boundary와 Unicode 정규화는 그대로 적용됩니다.
            </p>
            <pre>
              <code>kfind --expand literal 걸어 .</code>
            </pre>
            <p className="option-result">
              <strong>찾음</strong> 걸어
            </p>
            <p className="option-result">
              <strong>제외</strong> 걷다 · 걸었다
            </p>
          </article>

          <article className="option-card" data-featured="true">
            <header>
              <code>inflection</code>
              <span>기본값</span>
            </header>
            <p>
              사전의 품사·활용 분류로 조사 결합, 어미 결합, 불규칙 교체와 제한된
              continuation을 만듭니다. 생산적 파생 접미사는 추가하지 않습니다.
            </p>
            <pre>
              <code>kfind --expand inflection 걷다 .</code>
            </pre>
            <p className="option-result">
              <strong>찾음</strong> 걸어 · 걸었다 · 걷는 · 걷기에서도
            </p>
            <p className="option-result">
              <strong>제외</strong> 새 파생 표제어
            </p>
          </article>

          <article className="option-card">
            <header>
              <code>derivation</code>
              <span>inflection 포함</span>
            </header>
            <p>
              inflection의 모든 branch에 버전 관리된 생산적 파생을 더합니다.
              <code>-적</code>, <code>-하다</code>, <code>-되다</code>,
              <code>-시키다</code>, <code>-스럽다</code>, <code>-답다</code>,
              <code>-롭다</code>, <code>-화</code>가 현재 규칙 목록입니다.
            </p>
            <pre>
              <code>kfind --expand derivation 검증 .</code>
            </pre>
            <p className="option-result">
              <strong>찾음</strong> 검증 · 검증하다 · 검증했다 · 검증되다
            </p>
            <p className="option-result">
              <strong>비용</strong> branch와 오탐 가능성 증가
            </p>
          </article>
        </div>

        <SplitDiagram
          title="확장 모드는 포함 관계로 동작합니다"
          caption="derivation은 별도 검색기가 아니라 inflection plan에 파생 branch를 합친 상위 모드입니다."
          source={{
            label: 'QUERY',
            title: '검증 · noun',
            description:
              '하나의 분석에서 선택한 확장 수준에 따라 branch가 달라집니다.',
          }}
          paths={[
            {
              label: 'LITERAL',
              title: '검증',
              description: '입력 표면형만 검색합니다.',
            },
            {
              label: 'INFLECTION',
              title: '검증 + 조사',
              description:
                '검증, 검증을, 검증에서도처럼 체언 굴절을 허용합니다.',
            },
            {
              label: 'DERIVATION',
              title: '검증 + 조사 + 파생',
              description: '검증하다·검증되다와 그 활용형까지 추가합니다.',
            },
          ]}
        />

        <Callout title="--literal 단축 옵션" tone="warning">
          <p>
            <code>--literal</code>은 <code>--expand literal --pos literal</code>
            을 동시에 지정합니다. <code>--expand inflection|derivation</code>{' '}
            또는 literal이 아닌 <code>--pos</code>와 함께 쓰면 컴파일
            오류입니다. 이 경로는 full POS lexicon을 필요로 하지 않습니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection
        title="Boundary 정책"
        lead="확장이 무엇을 생성하는 축이라면 boundary는 후보 span을 어디까지 허용할지 정하는 축입니다."
      >
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">값</th>
                <th scope="col">검증</th>
                <th scope="col">선택 기준</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>smart</code>
                </td>
                <td>
                  품사 verifier가 조사·어미를 소비한 token 끝을 확인합니다.
                  검증된 합성명사 component는 복구합니다.
                </td>
                <td>사람의 기본 검색, precision 우선</td>
              </tr>
              <tr>
                <td>
                  <code>token</code>
                </td>
                <td>
                  모든 core 시작과 완성 token 끝에 엄격한 Unicode token 경계를
                  요구합니다.
                </td>
                <td>독립 token만 필요할 때</td>
              </tr>
              <tr>
                <td>
                  <code>any</code>
                </td>
                <td>
                  좌우 경계를 요구하지 않고 형태 branch가 만든 부분 문자열
                  후보를 보존합니다.
                </td>
                <td>자동화와 recall 우선 검색</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div className="example-grid">
          <article>
            <span>SMART COMPONENT</span>
            <code>n:요리 → 중국요리</code>
            <p>
              완전한 형태 경로가 <code>요리</code> component를 증명하면
              허용합니다.
            </p>
          </article>
          <article>
            <span>CROSSING SUBSTRING</span>
            <code>n:국요 → 중국요리</code>
            <p>component 경계를 가로지르므로 smart에서는 거부합니다.</p>
          </article>
          <article>
            <span>UNRESTRICTED</span>
            <code>국요 → 중국요리</code>
            <p>
              <code>any</code>는 형태 경계 근거 없이 부분 문자열을 허용합니다.
            </p>
          </article>
        </div>
      </DocumentSection>

      <DocumentSection title="품사와 자동 분석">
        <p>
          <code>--pos auto</code>는 core lexicon, 사용자 사전, 생산적 접미 패턴,
          full POS 순으로 가능한 분석을 모읍니다. 같은 표제어의 복수 분석은
          합집합으로 보존합니다. <code>--pos</code> 또는 atom 태그는 이 후보를
          한 품사로 좁힙니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">CLI 값</th>
                <th scope="col">Atom 태그</th>
                <th scope="col">주요 확장</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>noun</code>
                </td>
                <td>
                  <code>n:</code>
                </td>
                <td>복수·조사 연쇄, derivation</td>
              </tr>
              <tr>
                <td>
                  <code>pronoun</code>
                </td>
                <td>
                  <code>pro:</code>
                </td>
                <td>대명사 override와 조사</td>
              </tr>
              <tr>
                <td>
                  <code>numeral</code>
                </td>
                <td>
                  <code>num:</code>
                </td>
                <td>체언 verifier</td>
              </tr>
              <tr>
                <td>
                  <code>verb</code>
                </td>
                <td>
                  <code>v:</code>
                </td>
                <td>동작 용언 어미와 불규칙 활용</td>
              </tr>
              <tr>
                <td>
                  <code>adjective</code>
                </td>
                <td>
                  <code>adj:</code>
                </td>
                <td>상태 용언 어미와 불규칙 활용</td>
              </tr>
              <tr>
                <td>
                  <code>determiner</code>
                </td>
                <td>
                  <code>det:</code>
                </td>
                <td>literal surface와 경계</td>
              </tr>
              <tr>
                <td>
                  <code>adverb</code>
                </td>
                <td>
                  <code>adv:</code>
                </td>
                <td>literal, derivation에서 제한 보조사</td>
              </tr>
              <tr>
                <td>
                  <code>particle</code>
                </td>
                <td>
                  <code>j:</code>
                </td>
                <td>받침 조건을 반영한 조사 이형태</td>
              </tr>
              <tr>
                <td>
                  <code>interjection</code>
                </td>
                <td>
                  <code>intj:</code>
                </td>
                <td>literal surface와 token 경계</td>
              </tr>
              <tr>
                <td>
                  <code>literal</code>
                </td>
                <td>
                  <code>lit:</code>
                </td>
                <td>형태 분석 없음</td>
              </tr>
            </tbody>
          </table>
        </div>
        <Callout title="미등록 다 종결어">
          <p>
            철자가 <code>다</code>로 끝난다는 이유만으로 동사로 추정하지
            않습니다. 미등록 입력은 literal 후보로 남기며, 필요하면{' '}
            <code>v:커스텀하다</code>처럼 품사를 명시합니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection title="Unicode 정규화와 phrase 거리">
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">옵션</th>
                <th scope="col">동작</th>
                <th scope="col">비용·주의</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>--unicode-normalization nfc</code>
                </td>
                <td>NFC query branch로 검색합니다.</td>
                <td>기본값이며 corpus 전체를 복사해 정규화하지 않습니다.</td>
              </tr>
              <tr>
                <td>
                  <code>canonical</code>
                </td>
                <td>NFC와 NFD anchor를 모두 만듭니다.</td>
                <td>branch와 matcher 크기가 늘 수 있습니다.</td>
              </tr>
              <tr>
                <td>
                  <code>none</code>
                </td>
                <td>입력 byte 형태를 그대로 찾습니다.</td>
                <td>
                  정규화가 다른 문자열은 같은 글자로 보여도 일치하지 않습니다.
                </td>
              </tr>
              <tr>
                <td>
                  <code>--max-gap 24</code>
                </td>
                <td>
                  앞 atom의 token 끝과 다음 atom의 token 시작 사이 Unicode
                  scalar를 제한합니다.
                </td>
                <td>순서를 유지하며 줄을 넘지 않습니다.</td>
              </tr>
            </tbody>
          </table>
        </div>
        <pre>
          <code>kfind &apos;n:권한 v:검증하다&apos; src --max-gap 24</code>
        </pre>
      </DocumentSection>

      <DocumentSection title="파일 검색과 출력 옵션">
        <div className="compact-grid">
          <div>
            <strong>범위</strong>
            <code>--glob · --type · --hidden · --no-ignore</code>
          </div>
          <div>
            <strong>인코딩</strong>
            <code>auto · utf-8 · utf-16le · utf-16be · euc-kr</code>
          </div>
          <div>
            <strong>문맥</strong>
            <code>-A · -B · -C · -n · --column</code>
          </div>
          <div>
            <strong>요약</strong>
            <code>--count · --files-with-matches · --quiet</code>
          </div>
          <div>
            <strong>구조화</strong>
            <code>--json · --explain-query · --explain-match</code>
          </div>
          <div>
            <strong>실행</strong>
            <code>--threads · --sort path · --data-dir</code>
          </div>
          <div>
            <strong>Agent skill</strong>
            <code>--init · --agent</code>
          </div>
        </div>
        <p className="reference-link">
          출력 호환 옵션과 종료 코드의 규범 목록은{' '}
          <a href="https://github.com/SeokminHong/kfind/blob/main/README.ko.md">
            한국어 README
          </a>
          에서 확인할 수 있습니다.
        </p>
      </DocumentSection>

      <DocumentSection
        title="Agent skill 초기화 옵션"
        lead="초기화와 검색은 독립된 실행 모드입니다. 옵션 조합을 분리하면 자동화가 프로젝트 파일을 바꾸는 시점과 corpus를 읽는 시점이 명확해집니다."
      >
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">옵션</th>
                <th scope="col">입력</th>
                <th scope="col">계약</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>--init</code>
                </td>
                <td>query와 path 없음</td>
                <td>
                  현재 디렉터리에 agent skill을 초기화합니다. 검색 옵션과 함께
                  사용하면 오류입니다.
                </td>
              </tr>
              <tr>
                <td>
                  <code>--agent &lt;AGENT&gt;</code>
                </td>
                <td>
                  <code>claude-code</code> · <code>codex</code> ·{' '}
                  <code>gemini</code> · <code>custom</code>
                </td>
                <td>
                  <code>--init</code>에서만 사용하며 반복할 수 있습니다. 같은
                  대상을 여러 번 지정해도 한 번만 처리합니다.
                </td>
              </tr>
              <tr>
                <td>대상 옵션 생략</td>
                <td>TTY 선택 또는 비TTY stdin</td>
                <td>
                  TTY에서는 checkbox를 표시합니다. 비대화형 입력은 줄마다 agent
                  이름 하나를 받아 명시적 옵션과 같은 결과를 만듭니다.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <Callout title="안전한 파일 변경">
          <p>
            대화형 선택을 취소하거나 대상을 고르지 않으면 아무 파일도 바꾸지
            않습니다. 설치 중 하나라도 실패하면 전체 성공으로 보고하지 않으며,
            관리 표식이 없는 기존 파일은 보존합니다.
          </p>
        </Callout>
      </DocumentSection>
    </article>
  );
}
