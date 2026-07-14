import { FlowDiagram, SplitDiagram } from '../components/diagram';
import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function AnalysisPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="CONCEPT · MORPHOLOGY"
        title="Query-directed 형태 분석"
        summary="kfind의 형태 분석 결과는 문장을 토큰화하기 위한 것이 아닙니다. 입력한 표제어에서 검색 branch와 후보 검증용 verifier를 만드는 compile 단계입니다."
      >
        <Callout title="방향이 반대입니다">
          <p>
            일반 형태소 분석기는 문장 표면형에서 형태소를 복원합니다. kfind는
            반대로 표제어와 품사에서 검색할 표면형의 조건을 만듭니다. 그런 다음
            corpus에서 찾은 후보가 이 조건을 만족하는지 검증합니다.
          </p>
        </Callout>
      </PageIntro>

      <DocumentSection title="분석에서 검색 계획까지">
        <FlowDiagram
          title="한 atom의 compile 흐름"
          caption="각 단계에서 만든 branch와 provenance를 결과에 남깁니다. 정해진 한도를 넘으면 일부 branch를 버리지 않고 오류를 반환합니다."
          steps={[
            {
              label: '01 · NORMALIZE',
              title: '입력 정규화',
              description:
                '따옴표와 품사 태그를 파싱하고 선택한 Unicode 모드로 정규화합니다.',
            },
            {
              label: '02 · ANALYZE',
              title: '품사·어휘 조회',
              description:
                'core lexicon, user lexicon, productive suffix와 full POS에서 가능한 분석을 모읍니다.',
            },
            {
              label: '03 · GENERATE',
              title: '형태 branch 생성',
              description:
                '조사, 어미, 불규칙 교체와 선택적 파생을 규칙 데이터로 계산합니다.',
            },
            {
              label: '04 · COMPILE',
              title: 'Anchor·verifier 결합',
              description:
                '고정 prefix와 길이가 제한된 suffix 상태를 결합해 실행 가능한 plan을 만듭니다.',
            },
          ]}
        />
        <Callout title="명시적 coarse 품사는 세부 품사를 보존합니다">
          <p>
            사전에서 일치하는 분석을 찾지 못하면 지정한 coarse 품사가 지원하는
            세부 품사별 fallback 분석을 만듭니다. Full POS 분석이 있는{' '}
            <code>noun</code>도 기존 분석과 누락된 보통명사·고유명사·의존명사
            fallback을 합집합으로 보존합니다. 여러 분석이 같은 검색 branch로 합쳐져도
            세부 품사의 provenance는 남습니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection
        title="어휘 분류와 활용 계산을 분리"
        lead="사전은 표제어에 적용할 불규칙 교체를 결정합니다. generator는 실제 어간과 어미가 만나는 환경에서 표면형을 계산합니다."
      >
        <div className="example-grid">
          <article>
            <span>LEXICON</span>
            <code>걷다 · VV · DToL</code>
            <p>ㄷ 불규칙이라는 어휘적 분류만 보존합니다.</p>
          </article>
          <article>
            <span>RULE</span>
            <code>걷 + ㄷ→ㄹ + 어</code>
            <p>모음 시작 어미 환경에서 말음을 교체합니다.</p>
          </article>
          <article>
            <span>SURFACE</span>
            <code>걸어 · 걸었다</code>
            <p>같은 규칙을 듣다·싣다에도 재사용합니다.</p>
          </article>
        </div>
        <p>
          철자만으로 안전하게 판별할 수 없는 ㄷ·ㅂ·ㅅ·ㅎ·르·러 불규칙과 보충법은
          사전 entry에 명시합니다. 반면 받침 유무, <code>ㄹ</code> 탈락,
          <code>ㅡ</code> 탈락, 모음 축약과 자음 어미 결합은 환경 규칙으로
          계산합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="세 계층의 형태 규칙">
        <div
          className="layer-stack"
          role="list"
          aria-label="형태 규칙의 세 계층"
        >
          <div role="listitem">
            <span>1</span>
            <div>
              <strong>어휘적 교체</strong>
              <p>표제어별 불규칙 class, 복수 분석과 개별 surface override</p>
            </div>
            <code>걷다 → DToL</code>
          </div>
          <div role="listitem">
            <span>2</span>
            <div>
              <strong>어미·조사 이형태 선택</strong>
              <p>받침, ㄹ 받침, 모음 시작과 품사 feature 조건</p>
            </div>
            <code>은/는 · 으로/로</code>
          </div>
          <div role="listitem">
            <span>3</span>
            <div>
              <strong>표면 조합과 축약</strong>
              <p>음절 분해·조합으로 실제 한글 표면형 계산</p>
            </div>
            <code>보아 → 봐</code>
          </div>
        </div>
        <p>
          이 계층을 섞지 않으면 새 어미를 추가할 때 표제어별 분기가 늘지 않고,
          새 불규칙 entry를 추가할 때도 어미 목록을 복제할 필요가 없습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="활용과 파생의 차이">
        <SplitDiagram
          title="하나의 명사 분석에서 만드는 inflection·derivation branch"
          caption="파생된 표제어가 용언이면 해당 표제어에 predicate inflection을 다시 적용합니다."
          source={{
            label: 'ANALYSIS',
            title: '검증 · NNG',
            description:
              '하나의 명사 분석에서 inflection branch와 derivation branch가 갈라집니다.',
          }}
          paths={[
            {
              label: 'INFLECTION',
              title: '검증을 · 검증에서도',
              description:
                '표제어 품사를 유지한 채 조사 verifier를 연결합니다.',
            },
            {
              label: 'DERIVATION',
              title: '검증하다 · 검증되었다',
              description:
                '새 품사의 표제어를 만든 뒤 해당 품사의 활용 generator를 실행합니다.',
            },
          ]}
        />
        <Callout title="data/rules에 정의된 조합만 생성합니다" tone="warning">
          <p>
            어미, 조사 연쇄와 파생 접미사는 <code>data/rules</code>에 등록된
            목록과 전이만 사용합니다. 목록에 없는 조합은 문법적으로 가능해
            보이더라도 생성하지 않습니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection
        title="후보 token만 형태 분석"
        lead="smart boundary에서 token 경계만으로 판정할 수 없는 후보에 한해 compact component resource를 사용합니다."
      >
        <div className="decision-table">
          <div>
            <strong>입력 범위</strong>
            <span>candidate를 포함하는 Unicode token 하나만 분석</span>
          </div>
          <div>
            <strong>긍정 근거</strong>
            <span>
              query의 lemma·POS·span이 모두 같은 node가 완전 경로에 존재
            </span>
          </div>
          <div>
            <strong>판정</strong>
            <span>candidate를 포함한 경로와 제외한 경로의 최저 비용 비교</span>
          </div>
          <div>
            <strong>결과</strong>
            <span>accept · reject · ambiguous · error</span>
          </div>
          <div>
            <strong>상한</strong>
            <span>원문 256 bytes · NFC 64 scalar · node 4,096개</span>
          </div>
        </div>
        <pre>
          <code>{`중국요리
└─ 중국 / NNP + 요리 / NNG  → n:요리 accept
└─ 중국요리 / NNG           → exact node가 없으면 비용 경로 비교

국요
└─ 중국 / NNP | 요리 / NNG 경계를 가로침 → reject`}</code>
        </pre>
        <p>
          exact node가 있는 경로라도 비용이 높으면 그 사실만으로 candidate를
          수용하지 않습니다. candidate를 포함한 완전 경로와 제외한 완전 경로의
          최저 비용을 비교해 더 낮은 쪽을 따릅니다. 두 비용이 같으면 ambiguous로
          판정하고 candidate를 거부합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="분석 결과에 남기는 생성 근거">
        <p>
          여러 분석과 규칙이 같은 surface를 만들더라도 해당 span은 한 번만
          출력합니다. 대신 atom의 <code>analysisIndex</code>와 각
          <code>rulePath</code>를 모두 보존해 <code>--explain-match</code>와
          JSON에서 생성 이유를 확인할 수 있습니다.
        </p>
        <pre>
          <code>{`surface: 걸었다
analysis: 걷다 / verb / DToL
rule path: lexical.d-to-l → ending.past → ending.final-da`}</code>
        </pre>
      </DocumentSection>
    </article>
  );
}
