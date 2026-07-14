import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function BenchmarksPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="EVIDENCE · QUALITY & PERFORMANCE"
        title="워크로드를 섞지 않는 벤치마크"
        summary="형태 검색 품질, end-to-end CLI 비용, 초기화 비용과 literal scan은 각각 다른 대상을 측정합니다. 결과를 해석할 때는 입력·환경·revision을 고정한 source report를 함께 확인해야 합니다."
      >
        <Callout title="비교 결과 해석 시 주의점">
          <p>
            아래 외부 비교에는 같은 제품 task를 수행하는 데 필요한 query 준비,
            분석과 matching 비용이 모두 포함됩니다. tokenizer에 동일한 입력만
            전달해 처리량을 비교한 순위가 아닙니다.
          </p>
        </Callout>
      </PageIntro>

      <DocumentSection title="제품 persona 결과">
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
                <td>15,611.8</td>
                <td>5.4 MiB</td>
              </tr>
              <tr>
                <td>User · full POS + smart + untagged</td>
                <td>411 / 0 / 89</td>
                <td>100.00%</td>
                <td>82.20%</td>
                <td>90.23%</td>
                <td>11,869.5</td>
                <td>92.1 MiB</td>
              </tr>
            </tbody>
          </table>
        </div>
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
          대화형 입력 조건을 반영해 같은 query에서 품사를 제거합니다.
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
        <div className="metric-definition-grid">
          <article>
            <strong>Positive</strong>
            <p>gold와 lemma·POS가 같은 match의 span이 기대 span과 겹칩니다.</p>
          </article>
          <article>
            <strong>False positive</strong>
            <p>
              lemma·POS가 같은 match를 찾았지만 그 위치가 기대 span과 겹치지
              않습니다.
            </p>
          </article>
          <article>
            <strong>False negative</strong>
            <p>기대하는 lemma·POS·span과 일치하는 결과를 찾지 못했습니다.</p>
          </article>
        </div>
        <p>
          이 지표는 문장 전체의 tokenization 정확도를 측정하지 않습니다. 검색
          결과의 lemma·POS가 gold와 같고 두 span이 겹치는지를 측정합니다. 별도
          human fixture에서는 품사를 생략하고, 지원하는 어떤 품사로도 분석되지
          않는 query를 negative로 사용합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="성능 측정 계약">
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
        <div className="stat-strip">
          <div>
            <span>1 GiB literal</span>
            <strong>0.047 s</strong>
            <small>median</small>
          </div>
          <div>
            <span>Throughput</span>
            <strong>21,787</strong>
            <small>MiB/s</small>
          </div>
          <div>
            <span>Peak RSS</span>
            <strong>7.23</strong>
            <small>MiB</small>
          </div>
          <div>
            <span>Revision</span>
            <strong>a7b3c28</strong>
            <small>2026-07-12</small>
          </div>
        </div>
      </DocumentSection>

      <DocumentSection title="명시적 품사 smart recall">
        <p>
          candidate <code>8337022</code>는 baseline <code>f8e5e3e</code>의
          coarse noun fallback을 보존합니다. 그 결과 embedded test의 FN은
          91개에서 85개로, full-POS development의 FN은 60개에서 59개로
          줄었습니다. precision 하한은 유지했고 16개 hard-negative에서 새로운
          FP는 발생하지 않았습니다. 품사를 지정하지 않는 검색 결과는 바뀌지
          않았습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="원본 보고서">
        <ul className="reference-list">
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
