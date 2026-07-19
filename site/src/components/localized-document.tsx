import type { ReactNode } from 'react';

import { useDocumentLocale } from '../app/i18n';

import { DocumentPage, DocumentSection, PageIntro } from './document';

export interface DocumentContent {
  readonly eyebrow: string;
  readonly sections: ReadonlyArray<{
    readonly body: ReactNode;
    readonly title: ReactNode;
  }>;
  readonly summary: ReactNode;
  readonly title: ReactNode;
}

interface LocalizedDocumentProps {
  readonly content: Readonly<Record<string, DocumentContent>>;
}

export function LocalizedDocument({
  content,
}: LocalizedDocumentProps): React.JSX.Element {
  const locale = useDocumentLocale();
  const document = content[locale];

  if (document === undefined) {
    throw new Error(`document content is unavailable for locale ${locale}`);
  }

  return (
    <DocumentPage>
      <PageIntro
        eyebrow={document.eyebrow}
        title={document.title}
        summary={document.summary}
      />
      {document.sections.map((section, index) => (
        <DocumentSection key={index} title={section.title}>
          {section.body}
        </DocumentSection>
      ))}
    </DocumentPage>
  );
}
