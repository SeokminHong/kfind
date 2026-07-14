import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function BenchmarksPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="EVIDENCE · QUALITY & PERFORMANCE"
        title="워크로드를 섞지 않는 벤치마크"
        summary="형태 검색 품질, end-to-end CLI 비용, 초기화와 literal scan은 서로 다른 질문에 답합니다. 각 지표는 입력·환경·revision이 고정된 source report와 함께 해석합니다."
      >
        <Callout title="비교 경계">
          <p>
            아래 외부 비교는 같은 제품 task의 query 준비·분석·matching 비용을
            포함합니다. 동일 입력의 tokenizer 처리량 순위가 아닙니다.
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
            Agent와 User fixture의 negative query 계약이 다르므로 두 행의 품질을
            backend 순위로 해석하지 않습니다.
          </figcaption>
        </figure>
      </DocumentSection>

      <DocumentSection title="외부 분석기와 제품 task 비교">
        <p>
          같은 1,000-case explicit-POS fixture와 gold를 사용합니다. Agent와 외부
          분석기는 품사를 명시하고, User만 실제 대화형 입력을 반영해 같은
          query에서 품사를 제거합니다.
        </p>
        <figure className="benchmark-figure">
          <img
            src="/benchmarks/product-external-comparison.svg"
            alt="kfind Agent, User와 Kiwi, Lindera, MeCab-ko, KOMORAN의 품질 및 실행 비용 비교"
            loading="lazy"
          />
          <figcaption>
            외부 행은 고정 snapshot입니다. fixture, schema, version 또는 adapter
            설정이 바뀔 때만 다시 측정합니다.
          </figcaption>
        </figure>
      </DocumentSection>

      <DocumentSection title="형태 품질의 정의">
        <div className="metric-definition-grid">
          <article>
            <strong>Positive</strong>
            <p>gold와 같은 lemma·POS의 match가 기대 span과 겹칩니다.</p>
          </article>
          <article>
            <strong>False positive</strong>
            <p>
              같은 lemma·POS match가 문장 어디엔가 있지만 기대하지 않은
              경우입니다.
            </p>
          </article>
          <article>
            <strong>False negative</strong>
            <p>기대 lemma·POS·span을 찾지 못한 경우입니다.</p>
          </article>
        </div>
        <p>
          이 지표는 문장 전체 tokenization 정확도가 아니라 검색 계약의
          lemma·POS·span overlap을 측정합니다. 별도 human fixture는 품사를
          생략하고 지원 품사에 없는 query를 negative로 사용합니다.
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
                <td>fresh process, warm-up 1회 + 측정 5회</td>
                <td>initialization, cases/s, p95, RSS의 median/min/max</td>
              </tr>
              <tr>
                <td>Query compile</td>
                <td>Criterion 기본 sample, analyzer 재사용</td>
                <td>sample당 1회 p95 nearest-rank</td>
              </tr>
              <tr>
                <td>1 GiB literal scan</td>
                <td>warm-up 1회, warm-cache 3회, run마다 scan 10회</td>
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
          main <code>f8e5e3e</code> 대비 후보 <code>8337022</code>에서 coarse
          noun fallback을 보존해 embedded test FN을 91에서 85로 줄였습니다.
          full-POS development FN은 60에서 59로 줄었고, precision 하한과 16개
          hard-negative의 신규 FP 0을 유지했습니다. 무품사 결과는 바뀌지
          않았습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Source reports">
        <ul className="reference-list">
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-dependent-noun-recall.md">
              의존명사 coarse-POS fallback recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-h-irregular-recall.md">
              ㅎ 불규칙 core lexicon recall
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-connective-ji-position-evidence.md">
              connective-ji 위치 근거
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-local-lattice-optimization.md">
              국소 lattice 제품 경로 최적화
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-14-user-smart-precision.md">
              User smart precision 품질·성능
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-13-product-workflows.md">
              제품 workflow 방법론과 외부 snapshot
            </a>
          </li>
          <li>
            <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/README.md">
              Benchmark contract와 전체 인덱스
            </a>
          </li>
        </ul>
      </DocumentSection>
    </article>
  );
}
