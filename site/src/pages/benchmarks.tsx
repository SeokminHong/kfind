import { DocumentSection, PageIntro } from '../components/document';

export default function BenchmarksPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="EVIDENCE · QUALITY & PERFORMANCE"
        title="워크로드를 섞지 않는 벤치마크"
        summary="형태 검색 품질, end-to-end CLI 비용, 초기화 비용과 literal scan은 서로 다른 경로를 측정합니다. 수치는 입력·환경·revision이 고정된 source report 안에서 해석해야 하며, 단위가 다른 결과를 하나의 점수로 합칠 수 없습니다. 외부 분석기 비교도 같은 제품 task에 필요한 query 준비, 분석과 matching을 포함하므로 순수 tokenizer 처리량 순위가 아닙니다."
      />

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
                <td>480 / 11 / 20</td>
                <td>97.76%</td>
                <td>96.00%</td>
                <td>96.87%</td>
                <td>15,397.7</td>
                <td>5.1 MiB</td>
              </tr>
              <tr>
                <td>User · full POS + smart + untagged</td>
                <td>417 / 0 / 83</td>
                <td>100.00%</td>
                <td>83.40%</td>
                <td>90.95%</td>
                <td>10,757.4</td>
                <td>92.1 MiB</td>
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
          측정합니다. 1 GiB literal scan은 형태 resource를 사용하지 않는 low-hit
          파일 scan을, product CLI workload는 실제 persona 옵션으로 100 MiB
          corpus를 검색하는 end-to-end 비용을 측정합니다.
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

      <DocumentSection title="사전 기반 불규칙 활용 lexicon">
        <p>
          <code>다르다 → 달라</code>는 르 불규칙이고{' '}
          <code>푸르다 → 푸르러</code>와 도달 뜻의 <code>이르다 → 이르러</code>
          는 러 불규칙입니다. 같은 종성에서도 활용이 갈리는 ㄷ·ㅅ·ㅂ·ㅎ은
          국립국어원 사전 snapshot의 활용형을 표제어별로 판별합니다. full-POS
          제품 경로는 기존 르·러 102개와 신규 ㄷ·ㅅ·ㅂ·ㅎ 176개, 규칙형 동형어
          companion 2개를 배포합니다.
        </p>
        <p>
          Main <code>e8f99c2</code> 대비 후보 <code>b6cd0a9</code>는 test와
          사람용 무품사 <code>smart</code>에서 새 FP 없이 FN을 각각 6건
          줄였습니다. Development와 hard-negative는 변하지 않았고, 100 MiB Human
          CLI 처리량과 isolated full-POS 초기화의 측정 범위는 겹쳤습니다. 확대된
          artifact의 peak RSS는 64~132 KiB 늘었습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="원본 보고서">
        <ul className="reference-list">
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
    </article>
  );
}
