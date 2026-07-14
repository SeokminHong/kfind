import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';
import { DocumentSection, PageIntro } from '../components/document';
import {
  GlossaryCategory,
  glossaryCategoryLabels,
  glossaryTerms,
} from '../components/glossary';

import * as styles from './glossary.css';

export default function GlossaryPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="REFERENCE · GLOSSARY"
        title="kfind 단어장"
        summary="문서에서 반복해서 사용하는 검색·형태 분석 용어를 같은 의미로 읽을 수 있도록 정의합니다. 영문 코드 표기와 한국어 설명은 한 항목에서 함께 확인할 수 있습니다."
      />

      {Object.values(GlossaryCategory).map((category) => (
        <DocumentSection
          key={category}
          title={glossaryCategoryLabels[category]}
        >
          <dl className={styles.list}>
            {glossaryTerms
              .filter((term) => term.category === category)
              .map((term) => (
                <div className={styles.entry} id={term.id} key={term.id}>
                  <dt className={styles.heading}>
                    <dfn className={styles.term}>{term.name}</dfn>
                    {term.notation === undefined ? null : (
                      <span className={styles.notation}>{term.notation}</span>
                    )}
                  </dt>
                  <dd className={styles.definition}>{term.definition}</dd>
                </div>
              ))}
          </dl>
        </DocumentSection>
      ))}

      <p className="next-link">
        다음:{' '}
        <Link to={RoutePath.Options}>쿼리와 옵션을 조합하는 방법 살펴보기</Link>
      </p>
    </article>
  );
}
