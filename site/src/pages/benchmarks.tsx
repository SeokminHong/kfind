import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';

export const meta = createDocumentMeta(RoutePath.Benchmarks);

export default function BenchmarksPage(): React.JSX.Element {
  return (
    <DocumentPage>
      <PageIntro
        eyebrow="EVIDENCE · QUALITY & PERFORMANCE"
        title="워크로드를 섞지 않는 벤치마크"
        summary="형태 검색 품질, 오류 입력 Robust 품질, end-to-end CLI 비용과 초기화 비용은 서로 다른 경로를 측정합니다. Canonical은 사람이 표준 맞춤법을 검증한 문장만 사용하고, 비문·오타 문장은 별도 Robust set에서 평가합니다. 수치는 입력·환경·revision이 고정된 source report 안에서 해석해야 하며 서로 다른 workload를 하나의 점수로 합치지 않습니다."
      />

      <DocumentSection title="표준문과 오류문을 분리한 품질 평가">
        <p>
          Canonical 품질은 UD Korean-Kaist의 실제 샘플링 후보 문장을 사람이 모두
          읽고 표준 맞춤법과 문장성을 검증한 뒤 통과한 문장만 사용합니다. Test
          후보 813문장 중 57문장, development 후보 792문장 중 64문장의 비문,
          오타, 띄어쓰기 오류와 source artifact를 제외했습니다. 최종 canonical은
          500 positive와 500 negative이며 Robust 점수와 합산하지 않습니다.
        </p>
        <p>
          Robust 품질은 오류가 많은 UD Korean-KSL test split에서 source signal
          문장 441개와 quota 보충 문장 4개를 전부 수동 검토해 만들었습니다. 실제
          오류 문장 439개만 남기고 정상문 5개와 source artifact 1개를 제외한 뒤,
          query·품사·원문 byte span을 다시 검증해 250 positive와 250 negative를
          고정했습니다. Synthetic 교정문이 아니라 원문 오류 문장 자체를
          평가하며, 현재 비교는 모든 제품의 기본 설정과 kfind{' '}
          <code>robustness=off</code> 기준입니다.
        </p>
        <p>
          Positive 250건 중 100건은 오타가 찾으려는 형태소 span에 직접 걸린{' '}
          <code>target-span</code>, 150건은 오류가 주변 문맥에만 있는{' '}
          <code>context-only</code>입니다. 아래 표의 전체 precision·recall·F1은
          같은 500건에서 계산하고, 두 recall을 함께 공개해 목표 오타 복구와 주변
          오류 내성을 구분합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">제품 기본 경로</th>
                <th scope="col">Precision</th>
                <th scope="col">Recall</th>
                <th scope="col">F1</th>
                <th scope="col">FP</th>
                <th scope="col">Target recall</th>
                <th scope="col">Context recall</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>kfind Agent · embedded + any</td>
                <td>97.89%</td>
                <td>92.80%</td>
                <td>95.28%</td>
                <td>5</td>
                <td>90.00%</td>
                <td>94.67%</td>
              </tr>
              <tr>
                <td>Kiwi 0.23.2</td>
                <td>100.00%</td>
                <td>85.20%</td>
                <td>92.01%</td>
                <td>0</td>
                <td>85.00%</td>
                <td>85.33%</td>
              </tr>
              <tr>
                <td>Lindera 4.0.0</td>
                <td>100.00%</td>
                <td>83.20%</td>
                <td>90.83%</td>
                <td>0</td>
                <td>83.00%</td>
                <td>83.33%</td>
              </tr>
              <tr>
                <td>MeCab-ko 1.0.2</td>
                <td>100.00%</td>
                <td>82.40%</td>
                <td>90.35%</td>
                <td>0</td>
                <td>83.00%</td>
                <td>82.00%</td>
              </tr>
              <tr>
                <td>KOMORAN 3.3.9</td>
                <td>100.00%</td>
                <td>82.00%</td>
                <td>90.11%</td>
                <td>0</td>
                <td>81.00%</td>
                <td>82.67%</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          같은 explicit-POS gold에서 kfind Agent가 recall과 F1은 가장 높았지만
          false positive 5건이 있었고, 외부 네 제품은 false positive 없이 더
          낮은 recall을 기록했습니다. 품사를 생략한 kfind Human 경로는 별도
          fixture에서 precision 99.52%, recall 83.60%, F1 90.87%, target recall
          68.00%였습니다. 입력 계약이 다르므로 위 제품 순위에 합치지 않습니다.
        </p>
        <figure className="benchmark-figure">
          <img
            src="/benchmarks/robustness-quality.svg"
            alt="실제 오류 문장 500건에서 kfind Agent, Kiwi, Lindera, MeCab-ko, KOMORAN의 precision, recall, F1과 오류 위치별 recall 비교"
            loading="lazy"
          />
          <figcaption>
            250 positive / 250 negative. Target 100건과 context 150건은 positive
            분모이며, canonical 표준문 점수와 분리합니다.
          </figcaption>
        </figure>
        <figure className="benchmark-figure">
          <img
            src="/benchmarks/robustness-performance.svg"
            alt="동일한 Robust 오류 문장 500건에서 제품별 처리량, 초기화, p95 latency와 peak RSS 비교"
            loading="lazy"
          />
          <figcaption>
            Fresh process에서 warm-up 1회 후 5회 측정한 중앙값입니다. 품질과
            실행 비용은 별도 축으로 해석합니다.
          </figcaption>
        </figure>
      </DocumentSection>

      <DocumentSection title="Query matrix raw와 contract 품질">
        <p>
          Explicit-POS query matrix는 432개 표준문에서 2,592개 질의를
          평가합니다. Raw 지표는 source corpus gold를 그대로 보존합니다.
          Contract 지표는 제품 실행 전에 고정한 review registry를 적용해, 문법
          구조로 구분할 수 없는 동형이의와 source 정렬 성분을 양성으로 보고 gold
          정렬 오류를 교정합니다. 현재 비표준 띄어쓰기 3건만 분모에서
          제외합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">kfind profile</th>
                <th scope="col">Raw precision / recall</th>
                <th scope="col">Precisionᶜ / recallᶜ</th>
                <th scope="col">Raw FP / FN</th>
                <th scope="col">FPᶜ / FNᶜ</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>embedded + smart</td>
                <td>99.67% / 92.13%</td>
                <td>100.00% / 92.44%</td>
                <td>4 / 102</td>
                <td>0 / 98</td>
              </tr>
              <tr>
                <td>full POS + smart</td>
                <td>99.69% / 99.15%</td>
                <td>100.00% / 99.46%</td>
                <td>4 / 11</td>
                <td>0 / 7</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          Full-POS의 FNᶜ 7건은 피동 파생, source 정렬 용언 성분과 반환 span
          복원처럼 아직 구현하지 않은 제품 목표입니다. 비용이나 현재 profile을
          이유로 계약 분모에서 빼지 않습니다.
        </p>
        <figure className="benchmark-figure">
          <img
            src="/benchmarks/query-matrix-quality.svg"
            alt="kfind embedded와 full POS query matrix에서 raw와 contract precision, recall, confusion matrix 비교"
            loading="lazy"
          />
          <figcaption>
            FPᶜ, FNᶜ, precisionᶜ, recallᶜ는 version-controlled registry가 실제로
            적용된 값이며 raw 지표의 별칭이 아닙니다. Review 22건의 판정은
            유지되며, 확인한 구현 목표 14건 중 7건을 해소했습니다.
          </figcaption>
        </figure>
      </DocumentSection>

      <DocumentSection title="제품 persona 결과">
        <p>
          Agent workflow와 User workflow는 실제 사용자가 제공하는 정보와 오류
          비용을 반영합니다. Agent는 모든 형태 atom에 품사를 명시하고 embedded
          lexicon과 <code>any</code> boundary를 사용해 recall과 낮은 초기화
          비용을 우선합니다. User는 품사를 생략하고 full POS lexicon, enriched
          용언 metadata와 <code>smart</code> boundary를 사용해 precision을
          우선합니다. 따라서 두 행은 같은 backend의 설정 차이뿐 아니라 서로 다른
          입력 계약을 나타냅니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">Workflow</th>
                <th scope="col">TP / FP / FN</th>
                <th scope="col">Precision</th>
                <th scope="col">Recall</th>
                <th scope="col">F1</th>
                <th scope="col">cases/s</th>
                <th scope="col">RSS</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Agent · embedded + any + explicit POS</td>
                <td>484 / 7 / 16</td>
                <td>98.57%</td>
                <td>96.80%</td>
                <td>97.68%</td>
                <td>53,317.6</td>
                <td>5.4 MiB</td>
              </tr>
              <tr>
                <td>User · full POS + smart + untagged</td>
                <td>489 / 2 / 11</td>
                <td>99.59%</td>
                <td>97.80%</td>
                <td>98.69%</td>
                <td>19,994.1</td>
                <td>57.4 MiB</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          표의 quality 값은 각 persona의 고정 fixture에서 계산합니다. 두
          fixture는 negative query를 고르는 기준이 다르므로 F1의 차이를 backend
          우열로 해석할 수 없습니다. 다음 차트는 품질과 함께 initialization,
          cases/s, p95 latency와 RSS를 배치해 각 workflow가 선택한 trade-off를
          보여 줍니다.
        </p>
        <figure className="benchmark-figure">
          <img
            src="/benchmarks/product-workflows.svg"
            alt="Agent와 User workflow의 품질, 처리량, 초기화, p95 latency와 RSS 비교"
            loading="lazy"
          />
          <figcaption>
            Agent와 User fixture는 negative query를 고르는 기준이 다릅니다. 두
            행의 품질 차이를 backend 간 우열로 해석할 수 없습니다.
          </figcaption>
        </figure>
      </DocumentSection>

      <DocumentSection title="외부 분석기와 제품 task 비교">
        <p>
          모든 분석기는 같은 1,000-case explicit-POS fixture와 gold로
          평가합니다. Agent와 외부 분석기에는 품사를 명시합니다. User만 실제
          대화형 입력 조건을 반영해 같은 query에서 품사를 제거합니다. 따라서
          User 결과에는 품사 자동 계획의 중의성과 비용이 포함되고, 다른 행과
          동일한 입력 조건이라고 볼 수 없습니다.
        </p>
        <p>
          외부 분석기 결과는 고정 snapshot으로 보존하고 fixture, schema, version
          또는 adapter 설정이 바뀔 때만 다시 측정합니다. 차트의 처리량은 각
          backend가 제품 task를 수행하는 데 필요한 query 준비, 분석과 matching을
          포함합니다. 동일한 문장을 tokenizer에 넣어 얻은 순수 분석 속도와는
          측정 구간이 다릅니다.
        </p>
        <figure className="benchmark-figure">
          <img
            src="/benchmarks/product-external-comparison.svg"
            alt="kfind Agent, User와 Kiwi, Lindera, MeCab-ko, KOMORAN의 품질 및 실행 비용 비교"
            loading="lazy"
          />
          <figcaption>
            외부 분석기 결과는 고정 snapshot으로 보존합니다. fixture, schema,
            version 또는 adapter 설정이 바뀔 때만 다시 측정합니다.
          </figcaption>
        </figure>
      </DocumentSection>

      <DocumentSection title="형태 품질의 정의">
        <p>
          형태 품질 fixture는 문장마다 찾으려는 lemma, POS와 기대 span을
          정의합니다. 검색 결과의 lemma와 POS가 gold와 같고 결과 span이 기대
          span과 겹치면 true positive입니다. 같은 lemma와 POS를 찾았지만 위치가
          기대 span과 겹치지 않으면 false positive이고, 기대 lemma·POS·span을
          만족하는 결과가 없으면 false negative입니다.
        </p>
        <p>
          이 정의는 문장 전체의 tokenization 정확도를 측정하지 않습니다. 제품이
          반환해야 하는 검색 span을 찾았는지만 측정합니다. 별도 human fixture는
          품사를 생략하고, query 표제어가 지원하는 어떤 품사로도 분석되지 않는
          문장을 negative로 사용합니다. 이 fixture의 negative 정의는
          explicit-POS fixture와 다르므로 두 결과를 하나의 F1 순위로 합치지
          않습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="성능 측정 계약">
        <p>
          각 workload는 실제로 바뀐 실행 경로를 분리해 측정합니다. Morphology
          process는 query compile과 match를 포함한 case 처리 비용을, query
          compile benchmark는 analyzer를 재사용할 때 plan 생성 비용을
          측정합니다. Matcher benchmark는 다중 anchor matcher의 one-shot build와
          짧은 검색, 재사용한 matcher의 큰 corpus scan을 분리합니다. 1 GiB
          literal scan은 형태 resource를 사용하지 않는 low-hit 파일 scan을,
          product CLI workload는 실제 persona 옵션으로 100 MiB corpus를 검색하는
          end-to-end 비용을 측정합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">Workload</th>
                <th scope="col">방법</th>
                <th scope="col">대표값</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Morphology process</td>
                <td>매번 fresh process 사용, warm-up 1회 후 5회 측정</td>
                <td>initialization, cases/s, p95, RSS의 median/min/max</td>
              </tr>
              <tr>
                <td>Query compile</td>
                <td>Criterion 기본 sample, analyzer 재사용</td>
                <td>sample당 1회 p95 nearest-rank</td>
              </tr>
              <tr>
                <td>Matcher build / reused scan</td>
                <td>
                  짧은 문장 one-shot과 고정 corpus 재사용을 별도 Criterion 측정
                </td>
                <td>sample당 1회 p95 nearest-rank</td>
              </tr>
              <tr>
                <td>1 GiB literal scan</td>
                <td>warm-up 1회 후 warm-cache run 3회, run마다 10회 scan</td>
                <td>1회 평균의 median</td>
              </tr>
              <tr>
                <td>Product CLI</td>
                <td>100 MiB·1,000파일, 독립 process</td>
                <td>wall, throughput, peak RSS</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          2026-07-12의 revision <code>a7b3c28</code>에서 1 GiB literal scan은
          median 0.047초, 21,787 MiB/s와 peak RSS 7.23 MiB를 기록했습니다. 이
          수치는 literal low-hit workload의 결과이며 morphology 품질이나 full
          POS 초기화 비용을 설명하지 않습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="상태 용언 현재 평서형 후속 형태 continuation">
        <p>
          형용사와 보조 형용사의 현재 평서형 <code>-다</code> 뒤에서{' '}
          <code>고</code>, <code>는</code>, <code>던</code>, <code>면</code>,{' '}
          <code>니</code>, <code>며</code>, <code>면서</code>, <code>는데</code>
          {', '}
          <code>지</code>를 소비합니다. <code>나쁘다면</code>,{' '}
          <code>좋다는</code>, <code>어렵다면서</code>를 양성 회귀 fixture로
          고정했습니다.
        </p>
        <p>
          동작 용언의 사전형, 지정사와 부정 지정사 <code>아니다</code>에는 이
          전이를 적용하지 않습니다. <code>가다면</code>, <code>나쁘다면도</code>
          {', '}
          <code>아니다면</code>은 거부합니다. <code>아니라면</code>은 별도
          continuation이 필요한 남은 경계로 둡니다.
        </p>
        <p>
          Main <code>809aa42</code> 대비 후보 <code>d6cefde</code>는
          development, test와 Human의 FN을 각각 1건 줄였습니다. 신규 FP는 없고
          Agent 품질은 바뀌지 않았습니다. Embedded·full-POS·Agent·User
          morphology cases/s는 0.05~1.88% 낮았고, 가장 큰 p95 증가는
          1.02%였습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="접속 조사 이면/면의 명사류 결합">
        <p>
          명사류 뒤의 접속 조사 <code>이면/면</code>은 받침 유무에 맞는 이형태만
          소비하고 token을 닫습니다. <code>백이면 백</code>,{' '}
          <code>공부면 공부</code>를 찾으며 <code>백면</code>,{' '}
          <code>공부이면</code>, <code>백이면도</code>는 거부합니다.
        </p>
        <p>
          Main <code>3a673bd</code> 대비 후보 <code>8b846aa</code>는 development
          embedded와 full-POS <code>smart</code>의 FN을 각각 1건 줄였습니다.
          신규 FP는 없고 고정 test, Agent, Human과 hard-negative 품질은 바뀌지
          않았습니다. Full-POS·Agent·User morphology cases/s는 1.23~2.08% 낮았고
          가장 큰 p95 증가는 2.65%였습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="현재 서술형 후속 형태 continuation">
        <p>
          현재 서술형 <code>-ㄴ다/-는다</code> 뒤에서 <code>고</code>,{' '}
          <code>는</code>, <code>던</code>, <code>면</code>, <code>니</code>,{' '}
          <code>며</code>, <code>면서</code>, <code>는데</code>, <code>지</code>
          를 소비합니다. <code>받는다는</code>, <code>받든다는</code>,{' '}
          <code>함께한다던</code>을 포함한 조합을 회귀 fixture로 고정했습니다.
        </p>
        <p>
          Main <code>8fb22eb</code> 대비 후보 <code>ccc9525</code>는 development
          embedded와 full-POS <code>smart</code>의 FN을 각각 1건 줄였습니다.
          신규 FP는 없고 고정 test, Agent, Human과 hard-negative 품질은 바뀌지
          않았습니다. Morphology cases/s는 2.88~4.04% 낮았고 초기화와 RSS는
          유지됐습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="선어말어미 뒤 -으되 continuation">
        <p>
          한국어기초사전의 결합 조건과 Korean-Kaist의 <code>치르+었+으되</code>
          분석에 따라 <code>ending.past</code>와 <code>ending.future</code>{' '}
          뒤에서만
          <code>으되</code>를 소비합니다. bare stem이나 <code>으데</code>는
          허용하지 않습니다.
        </p>
        <p>
          Main <code>7e58474</code> 대비 후보 <code>0ceb458</code>은 development
          embedded와 full-POS <code>smart</code>의 FN을 각각 1건 줄였습니다.
          고정 test, Agent, Human과 hard-negative 품질은 바뀌지 않았고 성능 측정
          범위는 기준선과 겹쳤습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="대명사 계사·의문 어미 축약">
        <p>
          <code>누구·무어·무엇 + 이(VCP) + -ㄴ가(EC/EF)</code>의 선언된 축약
          표면만 생성합니다. <code>smart</code>는 표면 전체에 같은 source
          품사열이 있을 때만 승인하고 뒤의 조사는 기존 조사 전이로 검증합니다.
          별도 표제어를 원 대명사의 alias로 합치지 않습니다.
        </p>
        <p>
          Main <code>67b5606</code> 대비 후보 <code>a240de5</code>는 test matrix
          full-POS FN을 2건 줄이고 FP를 유지했습니다. 성능 변화는 gate 안이고
          측정 범위가 겹치며 RSS는 감소했습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="제한된 사전 표면형 계층">
        <p>
          두 국립국어원 사전이 함께 지지하는 활용형 12,888개 중 12,758개는 기존
          분석과 생산 규칙으로 생성합니다. 배포 데이터에는 생성되지 않는 활용형
          130개, 두 사전이 독립 등재한 제한된 형용사 부사형 88개와 나머지 양방향
          파생 관계 77개를 저장합니다. 결과는 295행, 28,286바이트이며 정의와
          예문은 포함하지 않습니다.
        </p>
        <p>
          Main <code>2ef39d2</code> 대비 후보 <code>596b272</code>는 test matrix
          full-POS <code>smart</code>의 FN을 5건 줄이고 FP는 유지했습니다. 신규
          대조군은 조사 구조라서 거부했고, 초기화·처리량·p95·RSS 변화는 최신
          성능 gate 안입니다.
        </p>
      </DocumentSection>

      <DocumentSection title="원본 보고서">
        <ul className="reference-list">
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-18-pronoun-copula-ending.md">
              대명사 계사·의문 어미 축약 recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-18-dictionary-adverbial-i.md">
              사전 합의 -이 부사형 recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-18-query-matrix-contract-metrics.md">
              Query matrix raw·계약 품질 교정
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-17-robustness-quality.md">
              수동 검토 자연 오류 Robust 품질·성능
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-15-descriptive-declarative-continuation.md">
              상태 용언 현재 평서형 후속 형태 continuation
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-15-connector-myeon-particle.md">
              접속 조사 이면/면의 명사류 결합
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-15-present-declarative-continuation.md">
              현재 서술형 후속 형태 continuation
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-15-eudoe-continuation.md">
              선어말어미 뒤 -으되 continuation
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-15-dictionary-surface-lexicon.md">
              제한된 사전 표면형 계층
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-consonant-irregular-enriched-lexicon.md">
              ㄷ·ㅅ·ㅂ·ㅎ 불규칙 enriched 용언 lexicon
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-reu-reo-enriched-lexicon.md">
              르·러 불규칙과 enriched 용언 lexicon
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-full-pos-coarse-noun-recall.md">
              Full POS coarse noun 분석 합집합 recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-dependent-noun-recall.md">
              의존명사 coarse-POS fallback의 recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-h-irregular-recall.md">
              ㅎ 불규칙 core lexicon의 recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-connective-ji-position-evidence.md">
              connective-ji 위치 근거
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-local-lattice-optimization.md">
              국소 lattice를 사용하는 제품 경로 최적화
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-user-smart-precision.md">
              User smart precision 품질·성능
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-13-product-workflows.md">
              제품 workflow 측정 방법과 외부 snapshot
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/README.md">
              Benchmark contract와 보고서 전체 목록
            </a>
          </li>
        </ul>
      </DocumentSection>
    </DocumentPage>
  );
}
