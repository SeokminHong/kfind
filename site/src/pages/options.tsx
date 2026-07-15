import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';

export default function OptionsPage(): React.JSX.Element {
  return (
    <DocumentPage>
      <PageIntro
        eyebrow="REFERENCE · QUERY COMPILATION"
        title="쿼리와 옵션"
        summary="확장 수준은 어떤 형태를 생성할지 결정하고, 품사는 적용할 형태 규칙을 결정하며, boundary는 생성된 후보를 원문에서 허용할 조건을 결정합니다. Unicode 정규화와 phrase 거리는 문자열 표현과 여러 atom의 결합 범위를 정합니다. 각 축은 독립적으로 선택되지만 하나의 query plan 안에서 함께 적용됩니다."
      />

      <DocumentSection title="기본 query plan">
        <p>
          별도 옵션이 없으면 kfind는 활용형을 확장하고, 사전에서 품사를 자동으로
          분석하며, 품사별 <code>smart</code> boundary를 적용합니다. query는
          NFC로 정규화하고, phrase atom 사이에는 최대 24개의 Unicode scalar를
          허용합니다.
        </p>
        <pre>
          <code>{`expand=inflection
pos=auto
boundary=smart
unicode-normalization=nfc
max-gap=24`}</code>
        </pre>
        <p>
          이 기본값은 사람이 품사를 모르는 상태에서 직접 검색하는 경우를
          대상으로 합니다. 자동화가 recall과 재현 가능한 초기화 비용을 우선하면
          품사를 명시하고 <code>--boundary any --embedded --json</code>을 함께
          사용합니다. 다음 절들은 각 선택이 query plan을 어떻게 바꾸는지
          정의합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="확장 수준">
        <p>
          <code>--expand</code>는 하나의 표제어 analysis에서 생성할 branch의
          범위를 정합니다. 세 값은 독립된 검색기를 선택하는 것이 아니라, 입력
          표면형에서 활용과 파생을 차례로 추가하는 포함 관계를 이룹니다.
        </p>

        <h3>Literal</h3>
        <p>
          <code>literal</code>은 입력한 문자열을 포함하는 branch 하나만
          만듭니다. 활용형, 조사가 붙은 형태나 파생 표제어는 생성하지 않습니다.
          다만 선택한 boundary와 Unicode 정규화는 그대로 적용하므로, literal은
          byte 검색과 완전히 같은 의미가 아니라 형태 확장만 끈 상태입니다.
        </p>
        <pre>
          <code>{`kfind --expand literal 걸어 .

찾음: 걸어
제외: 걷다 · 걸었다`}</code>
        </pre>

        <h3>Inflection</h3>
        <p>
          기본값인 <code>inflection</code>은 사전의 품사와 활용 분류를 바탕으로
          조사 결합, 어미 결합, 불규칙 교체와 제한된 continuation을 생성합니다.
          명사 <code>검증</code>에서는 <code>검증을</code>과{' '}
          <code>검증에서도</code>를, 동사 <code>걷다</code>에서는{' '}
          <code>걸어</code>, <code>걸었다</code>, <code>걷는</code>을 찾을 수
          있습니다. 두 사전이 함께 지지하는 활용형도 포함하지만, 새로운 표제어를
          만드는 파생형은 이 범위에 포함하지 않습니다.
        </p>
        <pre>
          <code>{`kfind --expand inflection 걷다 .

찾음: 걸어 · 걸었다 · 걷는 · 걷기에서도
제외: 새 파생 표제어`}</code>
        </pre>

        <h3>Derivation</h3>
        <p>
          <code>derivation</code>은 inflection의 모든 branch를 보존하고{' '}
          <code>data/rules</code>에 정의된 생산적 파생 규칙을 추가합니다. 현재
          규칙은 <code>-적</code>, <code>-하다</code>, <code>-되다</code>,{' '}
          <code>-시키다</code>, <code>-스럽다</code>, <code>-답다</code>,{' '}
          <code>-롭다</code>와 <code>-화</code>를 포함합니다. 파생 결과가
          용언이면 그 표제어에 용언 활용을 다시 적용하므로 <code>검증하다</code>
          뿐 아니라 <code>검증했다</code>도 검색합니다. 이 범위는 branch 수와
          false positive 가능성을 함께 늘리므로 파생어가 필요한 query에만
          사용합니다. 한국어기초사전에서 용언과 부사의 entry ID가 양방향으로
          일치하는 파생형도 이 단계에서만 추가됩니다.
        </p>
        <pre>
          <code>{`검증 / noun
  ├─ literal    → 검증
  ├─ inflection → 검증 · 검증을 · 검증에서도
  └─ derivation → 위 결과 + 검증하다 · 검증했다 · 검증되다`}</code>
        </pre>
        <p>
          <code>--literal</code>은 <code>--expand literal --pos literal</code>을
          동시에 지정하는 단축 옵션입니다. 따라서{' '}
          <code>--expand inflection|derivation</code> 또는 literal이 아닌{' '}
          <code>--pos</code>와 함께 사용할 수 없으며, 충돌하면 컴파일 오류를
          반환합니다. Literal query는 품사 사전이 필요하지 않으므로 full POS
          lexicon을 읽지 않습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Boundary 정책">
        <p>
          확장 수준이 생성할 형태를 결정한다면 boundary는 원문에서 발견한 span이
          어떤 문자 환경에 놓여야 하는지를 결정합니다. 같은 branch라도
          boundary에 따라 허용되는 위치가 달라지므로, 형태 coverage와 부분
          문자열 허용 범위를 별개의 문제로 다뤄야 합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">값</th>
                <th scope="col">검증 조건</th>
                <th scope="col">적합한 용도</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>smart</code>
                </td>
                <td>
                  품사 verifier가 조사·어미를 소비한 뒤 token 끝을 확인합니다.
                  명사·대명사·수사·관형사 component는 같은 fine POS의 형태
                  근거가 검증된 경우에만 복구합니다.
                </td>
                <td>사람의 기본 검색, precision 우선</td>
              </tr>
              <tr>
                <td>
                  <code>token</code>
                </td>
                <td>
                  core 시작과 완성된 token 끝이 모두 Unicode token 경계에 맞아야
                  합니다.
                </td>
                <td>독립 token만 필요한 검색</td>
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
        <p>
          예를 들어 <code>n:요리</code>를 <code>중국요리</code>에서 찾을 때{' '}
          <code>smart</code>는 완전한 형태 경로에서 <code>요리</code>가 명사
          component로 확인되면 후보를 허용합니다. 반면 <code>국요</code>는{' '}
          <code>중국</code>과 <code>요리</code>의 component 경계를 가로지르므로
          거부합니다. <code>any</code>는 이런 형태 경계를 요구하지 않으므로 같은
          문자열을 부분 span으로 보존합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="품사와 자동 분석">
        <p>
          <code>--pos auto</code>는 core lexicon, enriched 용언 metadata, user
          lexicon, 생산적 접미 패턴과 full POS lexicon에서 가능한 analysis를
          정해진 우선순위로 모읍니다. 같은 표제어에 여러 analysis가 있으면
          하나를 임의로 선택하지 않고 합집합을 보존합니다. 전역{' '}
          <code>--pos</code> 또는 atom 태그를 지정하면 이 집합을 해당 coarse
          POS로 제한합니다.
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
        <p>
          입력이 <code>다</code>로 끝난다는 사실만으로는 동사인지, 형용사인지,
          체언인지 결정할 수 없습니다. kfind는 사전에 없는 <code>다</code> 종결
          입력을 용언으로 추정하지 않고 literal 후보로 남깁니다. 새 용언을
          활용형까지 검색하려면 <code>v:커스텀하다</code>처럼 품사를 명시하거나
          user lexicon에 분석을 추가해야 합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Unicode 정규화와 phrase 거리">
        <p>
          정규화 옵션은 같은 글자가 서로 다른 Unicode byte열로 표현될 때 만들
          query branch를 정합니다. 원문 전체를 복사해 정규화하지 않으므로,
          선택한 모드에 따라 anchor 수와 검증 비용이 달라집니다. Phrase의{' '}
          <code>max-gap</code>은 정규화와 별개로, 앞 atom의 token 끝과 다음
          atom의 token 시작 사이에 허용할 거리를 제한합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">옵션</th>
                <th scope="col">동작</th>
                <th scope="col">비용과 제약</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>--unicode-normalization nfc</code>
                </td>
                <td>NFC로 정규화한 query branch를 사용합니다.</td>
                <td>기본값이며 corpus 전체를 정규화하지 않습니다.</td>
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
                  인접 atom의 token span 사이 Unicode scalar 수를 제한합니다.
                </td>
                <td>atom 순서를 유지하고 줄을 넘지 않습니다.</td>
              </tr>
            </tbody>
          </table>
        </div>
        <pre>
          <code>kfind &apos;n:권한 v:검증하다&apos; src --max-gap 24</code>
        </pre>
      </DocumentSection>

      <DocumentSection title="파일 검색과 출력 옵션">
        <p>
          Query compile 옵션과 별도로 검색 범위, 입력 인코딩, 출력 문맥과 결과
          형식을 제어할 수 있습니다. 다음 표는 역할별 옵션 묶음이며, 값과 충돌
          규칙을 포함한 전체 CLI 계약은{' '}
          <a href="https://github.com/SeokminHong/kfind/blob/main/README.ko.md">
            한국어 README
          </a>
          에서 확인할 수 있습니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">역할</th>
                <th scope="col">옵션 또는 값</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>검색 범위</td>
                <td>
                  <code>--glob · --type · --hidden · --no-ignore</code>
                </td>
              </tr>
              <tr>
                <td>인코딩</td>
                <td>
                  <code>auto · utf-8 · utf-16le · utf-16be · euc-kr</code>
                </td>
              </tr>
              <tr>
                <td>문맥과 위치</td>
                <td>
                  <code>-A · -B · -C · -n · --column</code>
                </td>
              </tr>
              <tr>
                <td>결과 요약</td>
                <td>
                  <code>--count · --files-with-matches · --quiet</code>
                </td>
              </tr>
              <tr>
                <td>구조화와 설명</td>
                <td>
                  <code>--json · --explain-query · --explain-match</code>
                </td>
              </tr>
              <tr>
                <td>Terminal 출력</td>
                <td>
                  <code>--color · --no-pager</code>
                </td>
              </tr>
              <tr>
                <td>실행 제어</td>
                <td>
                  <code>--threads · --sort path · --data-dir</code>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          일반 text 결과를 TTY stdin/stdout에서 쓰면 검색 시작과 함께 내장 TUI를
          열고 완성된 행을 점진적으로 반영합니다. 긴 match 줄은 검증된 match마다
          별도 행으로 펼치고 target 앞뒤를 원문 비율에 맞춰 생략합니다. 검색
          결과의 마지막 행이 content 영역 아래에 닿으면 더 스크롤하지 않습니다.
          키 반복 이동은 입력된 이동량을 유지하면서 content viewport 크기에 맞는
          frame으로 합칩니다. 검색 중에도 terminal resize와 이동을 처리하며,
          <code>↑/↓</code> 또는 <code>k/j</code>로 이동하고 <code>q</code>나
          <code>Esc</code>로 종료합니다. Redirect, pipe, JSON Lines와
          count·파일명 요약·quiet 출력은 기존 stdout stream을 유지합니다. 대화형
          text 출력에서도 TUI가 필요하지 않으면 <code>--no-pager</code>로 끌 수
          있습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Agent skill 초기화 옵션">
        <p>
          Skill 초기화는 프로젝트 파일을 변경하는 동작이고, 검색은 corpus를 읽는
          동작입니다. 두 책임을 섞지 않기 위해 <code>--init</code> mode에는
          query, path와 검색 옵션을 전달할 수 없습니다.
        </p>
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
                <td>현재 디렉터리에 agent skill을 초기화합니다.</td>
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
                  대상은 한 번만 처리합니다.
                </td>
              </tr>
              <tr>
                <td>대상 옵션 생략</td>
                <td>TTY 선택 또는 비TTY stdin</td>
                <td>
                  TTY에서는 checkbox를 표시하고, 비대화형 입력에서는 공백이나
                  줄바꿈으로 구분한 agent 이름을 받습니다.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          대화형 선택을 취소하거나 대상을 고르지 않으면 파일을 변경하지
          않습니다. 설치 대상 중 하나라도 실패하면 전체 작업을 성공으로 보고하지
          않으며, kfind 관리 표식이 없는 기존 파일은 보존합니다. 이 계약은 반복
          실행으로 관리 중인 skill을 갱신하면서도 사용자가 만든 파일을 덮어쓰지
          않게 합니다.
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}
