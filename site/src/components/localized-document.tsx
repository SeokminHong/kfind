import type { ReactNode } from 'react';

import { useDocumentLocale } from '../app/i18n';

import { DocumentPage, DocumentSection, PageIntro } from './document';

export interface DocumentContent {
  readonly eyebrow: string;
  readonly overview?: string;
  readonly sections: ReadonlyArray<{
    readonly body: ReactNode;
    readonly title: ReactNode;
  }>;
  readonly title: ReactNode;
}

interface LocalizedDocumentProps {
  readonly content: Readonly<Record<string, DocumentContent>>;
  readonly sectionIds: readonly string[];
}

export function LocalizedDocument({
  content,
  sectionIds,
}: LocalizedDocumentProps): React.JSX.Element {
  const locale = useDocumentLocale();
  const document = content[locale];

  if (document === undefined) {
    throw new Error(`document content is unavailable for locale ${locale}`);
  }
  if (document.sections.length !== sectionIds.length) {
    throw new Error(
      `document section identifiers do not match locale ${locale}`,
    );
  }

  return (
    <DocumentPage>
      <PageIntro eyebrow={document.eyebrow} title={document.title}>
        {document.overview === undefined ? null : <p>{document.overview}</p>}
      </PageIntro>
      {document.sections.map((section, index) => {
        const sectionId = sectionIds[index];
        if (sectionId === undefined) {
          throw new Error(`document section ${index} has no identifier`);
        }

        return (
          <DocumentSection id={sectionId} key={sectionId} title={section.title}>
            {section.body}
          </DocumentSection>
        );
      })}
    </DocumentPage>
  );
}
