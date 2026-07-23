import { Link, useLocation } from 'react-router';

import { useDocumentLocale, useDocumentTranslation } from '../app/i18n';
import {
  documentPageNeighborsForPath,
  knownRoutePathFromPathname,
} from '../app/navigation';

import * as styles from './page-navigation.css';

export function DocumentPageNavigation(): React.JSX.Element | null {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();
  const location = useLocation();
  const path = knownRoutePathFromPathname(location.pathname);
  const neighbors =
    path === undefined ? undefined : documentPageNeighborsForPath(path);

  if (
    neighbors === undefined ||
    (neighbors.previous === undefined && neighbors.next === undefined)
  ) {
    return null;
  }

  return (
    <nav
      aria-label={t('common.navigation.page_aria')}
      className={styles.navigation}
    >
      {neighbors.previous === undefined ? null : (
        <Link
          className={styles.previousLink}
          rel="prev"
          to={neighbors.previous.path}
        >
          <span className={styles.direction}>
            {t('common.navigation.previous_page')}
          </span>
          <span className={styles.title}>
            {neighbors.previous.label[locale]}
          </span>
        </Link>
      )}
      {neighbors.next === undefined ? null : (
        <Link className={styles.nextLink} rel="next" to={neighbors.next.path}>
          <span className={styles.direction}>
            {t('common.navigation.next_page')}
          </span>
          <span className={styles.title}>{neighbors.next.label[locale]}</span>
        </Link>
      )}
    </nav>
  );
}
