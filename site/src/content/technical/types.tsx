import type { ReactNode } from 'react';

import type { RoutePath } from '../../app/navigation';

import { useLocation } from 'react-router';

import { DocumentLocale, useDocumentLocale } from '../../app/i18n';
import {
  navigationPageForPath,
  routePathFromPathname,
} from '../../app/navigation';
import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../../components/document';

export interface TechnicalSectionContent {
  readonly code?: string;
  readonly items?: readonly string[];
  readonly paragraphs: readonly string[];
  readonly title: string;
}

export interface TechnicalDocumentContent {
  readonly eyebrow: string;
  readonly sections: readonly TechnicalSectionContent[];
  readonly summary: string;
  readonly title: string;
}

export type TechnicalDocuments = Readonly<
  Partial<
    Record<
      RoutePath,
      Readonly<Record<DocumentLocale, TechnicalDocumentContent>>
    >
  >
>;

function renderInline(text: string): ReactNode {
  return text
    .split(/(?<inlineCode>`[^`]+`)/u)
    .map((segment, index) =>
      segment.startsWith('`') && segment.endsWith('`') ? (
        <code key={`${segment}-${index}`}>{segment.slice(1, -1)}</code>
      ) : (
        segment
      ),
    );
}

export function TechnicalDocument({
  documents,
}: {
  readonly documents: TechnicalDocuments;
}): React.JSX.Element {
  const locale = useDocumentLocale();
  const location = useLocation();
  const path = routePathFromPathname(location.pathname);
  const index = navigationPageForPath(path);
  const content = documents[path]?.[locale];

  if (index === undefined || content === undefined) {
    throw new Error(`technical document is unavailable for ${path}`);
  }
  if (content.sections.length !== index.sections.length) {
    throw new Error(`technical document sections do not match ${path}`);
  }

  return (
    <DocumentPage>
      <PageIntro
        eyebrow={content.eyebrow}
        title={content.title}
        summary={content.summary}
      />
      {content.sections.map((entry, position) => {
        const indexedSection = index.sections[position];
        if (indexedSection === undefined) {
          throw new Error(`technical document section ${position} has no id`);
        }
        return (
          <DocumentSection
            id={indexedSection.id}
            key={indexedSection.id}
            title={entry.title}
          >
            {entry.paragraphs.map((paragraph) => (
              <p key={paragraph}>{renderInline(paragraph)}</p>
            ))}
            {entry.items === undefined ? null : (
              <ul>
                {entry.items.map((item) => (
                  <li key={item}>{renderInline(item)}</li>
                ))}
              </ul>
            )}
            {entry.code === undefined ? null : (
              <pre>
                <code>{entry.code}</code>
              </pre>
            )}
          </DocumentSection>
        );
      })}
    </DocumentPage>
  );
}
