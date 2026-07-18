import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';
import { PageIntro } from '../components/document';

export { notFoundMeta as meta } from '../app/metadata';

export default function NotFoundPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="404"
        title="문서 경로를 찾을 수 없습니다"
        summary="주소가 바뀌었거나 존재하지 않는 페이지입니다. 문서 개요에서 원하는 항목을 선택해 주세요."
      >
        <div className="document-links">
          <Link to={RoutePath.Overview}>문서 개요로 이동</Link>
        </div>
      </PageIntro>
    </article>
  );
}
