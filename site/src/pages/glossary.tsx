import { DocumentLocale, useDocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { DocumentSection, PageIntro } from '../components/document';
import { getGlossaryContent, GlossaryCategory } from '../components/glossary';

import * as styles from './glossary.css';

export const meta = createDocumentMeta(RoutePath.Glossary);

export default function GlossaryPage(): React.JSX.Element {
  const locale = useDocumentLocale();
  const content = getGlossaryContent(locale);

  return (
    <article>
      <PageIntro
        eyebrow={content.eyebrow}
        title={content.title}
        summary={content.summary}
      />

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
    </article>
  );
}
