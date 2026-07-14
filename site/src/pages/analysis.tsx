import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';

export default function AnalysisPage(): React.JSX.Element {
  return (
    <DocumentPage>
      <PageIntro
        eyebrow="CONCEPT · MORPHOLOGY"
        title="검색 쿼리에서 시작하는 형태 분석"
        summary="kfind는 corpus의 문장을 형태소열로 변환하지 않습니다. 사용자가 입력한 표제어와 품사를 유한한 query plan으로 컴파일한 뒤, 원문에서 찾은 후보가 그 계획을 만족하는지만 검증합니다. 따라서 형태 분석은 독립된 결과물이 아니라 검색 조건을 구성하는 수단입니다."
      />

      <DocumentSection title="분석의 방향과 범위">
        <p>
          일반적인 형태소 분석은 관찰된 문장을 입력으로 받아 각 표면형의
          표제어와 품사를 추정합니다. kfind가 해결하는 문제는 다릅니다. kfind는
          찾으려는 표제어와 품사를 먼저 알고 있으며, 여기에서 검색 가능한
          표면형과 그 표면형이 성립할 조건을 도출합니다. 이후 corpus를 훑으면서
          이 조건을 만족하는 위치만 결과로 선택합니다. 문장에서 표제어를
          복원하는 과정과 표제어에서 검색 조건을 만드는 과정은 입력과 출력이
          다르므로 구분해야 합니다.
        </p>
        <p>
          이 설계는 형태 처리 비용을 corpus의 크기가 아니라 query의 크기에
          결부합니다. 보통 query는 짧고 corpus는 크기 때문에, query를 한 번
          분석해 검색 계획을 만들고 원문에서는 고정 문자열을 먼저 찾는 편이 전체
          문장을 반복해서 분석하는 것보다 비용을 통제하기 쉽습니다. 다만 이
          방식은 문장의 모든 형태소나 문맥상 의미를 알려 주지 않습니다. 반환값은
          주어진 표제어에서 생성될 수 있는 후보 span과 그 생성 근거이며,
          최종적인 의미 판별은 호출자의 몫입니다.
        </p>
        <p>
          이하에서 <code>atom</code>은 하나의 표제어와 선택적 품사로 이루어진
          query 단위를 뜻합니다. 하나의 atom은 사전에서 얻은 하나 이상의{' '}
          <code>analysis</code>로 해석되고, 각 analysis는 검색 가능한 형태를
          나타내는 <code>branch</code>를 만듭니다. branch는 원문에서 먼저 찾을
          고정 문자열인 <code>anchor</code>와, anchor 주변의 조사·어미·경계를
          확인하는 <code>verifier</code>를 결합합니다. 이 용어들은 분석 결과와
          검색 실행 사이의 책임을 구분하기 위해 사용합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="입력에서 검색 계획까지">
        <p>
          컴파일은 정규화, 어휘 분석, 형태 생성, 실행 계획 구성의 순서로
          진행됩니다. 각 단계는 앞 단계의 결과를 좁히거나 확장하지만, 이전
          단계에서 선택한 품사와 생성 근거를 버리지 않습니다.
        </p>
        <pre>
          <code>{`query atom
  → parse and normalize
  → lexical analyses
  → morphological branches
  → anchor and verifier
  → executable query plan`}</code>
        </pre>
        <h3>입력 정규화와 어휘 분석</h3>
        <p>
          먼저 따옴표와 품사 태그를 해석하고, 선택한 Unicode 모드에 따라 입력을
          정규화합니다. 그다음 core lexicon, enriched 용언 metadata, user
          lexicon, 생산적인 접미 패턴과 full POS lexicon을 정해진 우선순위로
          조회합니다. 이 단계의 목적은 한 품사를 성급히 고르는 것이 아니라, 현재
          입력을 설명할 수 있는 분석 집합을 구성하는 것입니다. 하나의 표제어에
          규칙형과 불규칙형 분석이 함께 있으면 두 분석을 모두 보존합니다.
        </p>
        <p>
          사용자가 coarse POS를 명시했는데 사전 분석만으로 그 품사의 세부 범위를
          모두 채우지 못하면, 지원되는 세부 품사에 대해 fallback analysis를
          추가합니다. 예를 들어 <code>noun</code>은 보통명사·고유명사·의존명사를
          포함합니다. 여러 analysis가 결과적으로 같은 anchor와 verifier를 만들면
          실행 branch는 합칠 수 있지만, 각 세부 품사에서 유래했다는 provenance는
          합친 branch에 그대로 남깁니다.
        </p>

        <h3>형태 branch 생성</h3>
        <p>
          각 analysis에는 품사에 맞는 조사, 어미, 불규칙 교체와 선택적 파생
          규칙을 적용합니다. 이때 생성 가능한 모든 한국어 표현을 추측하지
          않습니다. 어미와 조사 연쇄, 파생 접미사의 허용 범위는{' '}
          <code>data/rules</code>에 기록된 목록과 전이로 제한됩니다. 문법적으로
          가능해 보이는 조합이라도 규칙 데이터에 없으면 branch를 만들지
          않습니다. 따라서 지원 범위는 구현의 우연한 분기가 아니라 버전으로
          관리되는 데이터에서 결정됩니다.
        </p>

        <h3>Anchor와 verifier 구성</h3>
        <p>
          생성된 표면형을 모두 완성 문자열로 열거하면 branch 수와 matcher
          메모리가 빠르게 증가합니다. kfind는 각 branch에서 충분히 긴 고정
          prefix를 anchor로 선택하고, 뒤따르는 조사나 어미는 공유된 verifier
          상태로 표현합니다. 예를 들어 <code>걸었</code>을 anchor로 찾은 뒤 같은
          verifier가 <code>습니다</code>, <code>지만</code>, <code>는데</code>와
          같은 continuation을 검사할 수 있습니다. 이 분리는 빠른 byte scan과
          형태 규칙 검증이 각각 맡을 일을 명확하게 합니다.
        </p>
        <p>
          컴파일 과정에는 query 길이, atom 수, analysis 수, 전체 branch 수와
          예상 matcher 메모리에 대한 상한이 있습니다. 상한을 넘었을 때 일부
          branch를 임의로 제거하면 검색 결과가 입력이나 실행 환경에 따라
          불완전해질 수 있습니다. kfind는 이런 축소를 하지 않고 컴파일 오류를
          반환하므로, 호출자는 계획이 완전하게 만들어지지 않았다는 사실을 알 수
          있습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="어휘 정보와 형태 규칙의 합성">
        <p>
          활용을 계산하려면 표제어에 고유한 정보와 여러 표제어가 공유하는 규칙을
          분리해야 합니다. 사전 entry는 어떤 불규칙 교체를 적용할지 결정하고,
          generator는 사전에서 선택한 교체와 실제 어간·어미 환경을 결합해
          표면형을 계산합니다. 두 사전이 함께 지지하지만 규칙으로 만들 수 없는
          소수 표면형만 별도 surface 계층에 저장합니다. 예를 들어
          <code>걷다</code>의 entry는 <code>DToL</code>이라는 어휘적 분류를
          제공하고, generator는 모음으로 시작하는 어미 앞에서{' '}
          <code>걷 + ㄷ→ㄹ + 어</code>를 적용해 <code>걸어</code>를 만듭니다.
        </p>
        <pre>
          <code>{`lexicon:  걷다 / VV / DToL
rule:     걷 + ㄷ→ㄹ + 어
surface:  걸어, 걸었다`}</code>
        </pre>
        <p>
          전체 규칙계는 세 층으로 합성됩니다. 첫째, 어휘 층은 표제어별 불규칙
          class, 복수 analysis와 개별 surface override를 제공합니다. 둘째,
          이형태 선택 층은 받침 유무, <code>ㄹ</code> 받침, 모음 시작 여부와
          품사 feature에 따라 <code>은/는</code>이나 <code>으로/로</code>처럼
          알맞은 조사·어미를 고릅니다. 셋째, 표면 조합 층은 한글 음절을 분해하고
          다시 조합해 <code>보아 → 봐</code>와 같은 축약을 계산합니다. 세 층을
          분리하면 새 어미를 추가할 때 표제어별 분기를 복제하지 않아도 되고, 새
          불규칙 표제어를 추가할 때도 기존 어미 체계를 그대로 재사용할 수
          있습니다.
        </p>
        <p>
          철자만으로 규칙형과 구별할 수 없는 ㄷ·ㅂ·ㅅ·ㅎ·르·러 불규칙과 보충법은
          사전에 명시합니다. 반면 받침 유무, <code>ㄹ</code> 탈락,{' '}
          <code>ㅡ</code> 탈락, 모음 축약과 자음 어미 결합은 환경 규칙으로
          계산합니다. 이 경계는 사전이 담당할 예외와 generator가 담당할 일반화를
          분명하게 유지합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="활용과 파생의 구분">
        <p>
          활용과 파생은 모두 하나의 analysis에서 시작할 수 있지만 결과의 품사
          관계가 다릅니다. 활용은 표제어의 품사를 유지한 채 조사나 어미를
          결합합니다. <code>검증</code>을 명사로 분석했다면 <code>검증을</code>
          과 <code>검증에서도</code>가 이 경로에 속합니다. 반면 파생은 접미사를
          결합해 새로운 품사의 표제어를 만듭니다. 같은 명사에서{' '}
          <code>검증하다</code>나 <code>검증되다</code>를 만들었다면, 그 결과는
          용언의 새 analysis가 되며 이후에는 용언 활용 규칙을 적용해{' '}
          <code>검증했다</code>나 <code>검증되었다</code>를 생성합니다.
        </p>
        <pre>
          <code>{`검증 / NNG
  ├─ inflection → 검증을, 검증에서도
  └─ derivation → 검증하다, 검증되다
                         └─ predicate inflection → 검증했다, 검증되었다`}</code>
        </pre>
        <p>
          이 구분은 파생 접미사를 단순한 문자열 suffix로 취급하지 않게 합니다.
          파생으로 품사가 바뀌면 이후에 허용되는 어미와 경계 조건도 함께
          바뀌어야 하기 때문입니다. 파생과 후속 활용 역시{' '}
          <code>data/rules</code>에 정의된 경로만 사용하므로, 지원 범위 밖의
          연쇄를 임의로 확장하지 않습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="필요한 후보만 국소적으로 분석">
        <p>
          대부분의 branch는 anchor 주변의 조사·어미와 token 경계만 검사하면
          판정할 수 있습니다. 그러나 <code>smart</code> boundary에서 복합 명사
          내부의 성분 경계를 확인하거나, token 전체의 어휘 분석과 부분 span의
          용언 해석이 충돌하는 경우에는 경계 검사만으로 충분하지 않습니다.
          kfind는 이런 context requirement를 가진 branch에 한해서 compact
          component resource를 사용합니다. 분석 범위도 corpus 전체가 아니라
          candidate를 포함하는 Unicode token 하나로 제한합니다.
        </p>
        <p>
          국소 분석은 candidate의 lemma, POS와 span이 모두 일치하는 node가
          완전한 분석 경로에 포함되는지를 확인합니다. 그런 경로가 존재한다는
          사실만으로 candidate를 채택하지는 않습니다. candidate를 포함하는 완전
          경로의 최저 비용과 포함하지 않는 완전 경로의 최저 비용을 비교하고,
          포함 경로가 더 강할 때만 수용합니다. 두 비용이 같으면 분석이 하나로
          결정되지 않은 것으로 보고 <code>ambiguous</code>로 거부합니다.
        </p>
        <pre>
          <code>{`중국요리
├─ 중국 / NNP + 요리 / NNG  → n:요리의 성분 경계가 성립
└─ 중국요리 / NNG           → 두 완전 경로의 비용을 비교

국요
└─ 중국 / NNP | 요리 / NNG 경계를 가로지름 → reject`}</code>
        </pre>
        <p>
          이 과정은 원문 256 bytes, NFC 64 Unicode scalar, node 4,096개라는 상한
          안에서만 실행됩니다. 입력이나 lattice가 상한을 넘거나 resource 검증에
          실패하면 결과를 추측하지 않고 오류를 반환합니다. 그러므로 국소 분석은
          일반 목적의 문장 분석기로 확장되지 않으며, 검색 후보 하나를 판정하는
          데 필요한 계산으로만 남습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="결과에 보존되는 생성 근거">
        <p>
          서로 다른 analysis와 rule path가 같은 surface를 만들 수 있습니다.
          kfind는 같은 위치의 span을 중복 출력하지 않지만, 그 span을 설명하는
          근거까지 하나로 축소하지는 않습니다. 결과에는 atom의{' '}
          <code>analysisIndex</code>와 각 <code>rulePath</code>를 모두
          보존합니다. 따라서 <code>--explain-match</code>와 JSON 출력에서 어떤
          표제어 분석과 규칙 연쇄가 해당 일치를 만들었는지 확인할 수 있습니다.
        </p>
        <pre>
          <code>{`surface: 걸었다
analysis: 걷다 / verb / DToL
rule path: lexical.d-to-l → ending.past → ending.final-da`}</code>
        </pre>
        <p>
          이 provenance는 검색 결과의 의미를 자동으로 확정하지 않습니다. 같은
          표면형이 여러 표제어에서 생성될 수 있으면 가능한 근거를 함께 보여 줄
          뿐입니다. 호출자는 원문 문맥과 자신의 작업 목적에 따라 후보를 최종
          선택할 수 있습니다.
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}
