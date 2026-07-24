import { Link } from 'react-router';

import { DocumentLocale, useDocumentLocale } from '../app/i18n';
import { RoutePath } from '../app/navigation';
import { PageIntro } from '../components/document';

export { notFoundMeta as meta } from '../app/metadata';

const copy = {
  [DocumentLocale.Korean]: {
    link: '문서 개요',
    overview: '주소가 잘못되었거나 존재하지 않는 문서 경로입니다.',
    title: '페이지 없음',
  },
  [DocumentLocale.English]: {
    link: 'Documentation overview',
    overview:
      'The address is invalid or does not identify a documentation page.',
    title: 'Page not found',
  },
} as const;

export default function NotFoundPage(): React.JSX.Element {
  const locale = useDocumentLocale();
  const content = copy[locale];

  return (
    <article>
      <PageIntro eyebrow="404" title={content.title}>
        <p>{content.overview}</p>
        <div className="document-links">
          <Link to={RoutePath.Overview}>{content.link}</Link>
        </div>
      </PageIntro>
    </article>
  );
}
