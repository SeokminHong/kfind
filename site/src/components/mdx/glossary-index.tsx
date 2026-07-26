import { DocumentLocale, useDocumentLocale } from '../../app/i18n';
import { DocumentSection } from '../document';
import { getGlossaryContent, GlossaryCategory } from '../glossary';

import * as styles from './glossary-index.css';

export function GlossaryIndex(): React.JSX.Element {
  const locale = useDocumentLocale();
  const content = getGlossaryContent(locale);

  return (
    <>
      {Object.values(GlossaryCategory).map((category) => (
        <DocumentSection
          id={category}
          key={category}
          title={content.categoryLabels[category]}
        >
          <dl className={styles.list}>
            {content.terms
              .filter((term) => term.category === category)
              .map((term) => (
                <div className={styles.entry} id={term.id} key={term.id}>
                  <dt className={styles.heading}>
                    <dfn className={styles.term}>{term.name}</dfn>
                    {term.notation === undefined ? null : (
                      <span className={styles.notation}>{term.notation}</span>
                    )}
                  </dt>
                  <dd className={styles.definition}>
                    {term.definition}
                    {term.example === undefined ? null : (
                      <>
                        <br />
                        <strong>
                          {locale === DocumentLocale.Korean
                            ? '예시'
                            : 'Example'}
                        </strong>{' '}
                        {term.example}
                      </>
                    )}
                  </dd>
                </div>
              ))}
          </dl>
        </DocumentSection>
      ))}
    </>
  );
}
