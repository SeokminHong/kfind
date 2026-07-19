import { Collapsible } from '@base-ui/react/collapsible';
import { NavLink, Outlet } from 'react-router';

import {
  changeDocumentLocale,
  DocumentLocale,
  useDocumentLocale,
  useDocumentTranslation,
} from './i18n';
import { DocumentLocaleSync } from './i18n-provider';
import { DocumentMetadataSync } from './metadata';
import { navigationGroups, RoutePath } from './navigation';

function Navigation(): React.JSX.Element {
  const { t } = useDocumentTranslation();

  return (
    <nav aria-label={t('common.navigation.toc_aria')}>
      {navigationGroups.map((group) => (
        <div className="navigation-group" key={group.labelKey}>
          <p>{t(group.labelKey)}</p>
          {group.items.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === RoutePath.Overview}
            >
              {t(item.labelKey)}
            </NavLink>
          ))}
        </div>
      ))}
    </nav>
  );
}

export function Shell(): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();

  return (
    <>
      <DocumentLocaleSync />
      <DocumentMetadataSync />
      <a className="skip-link" href="#content">
        {t('common.skip_to_content')}
      </a>
      <header className="docs-header">
        <div className="header-inner">
          <NavLink
            className="brand"
            to={RoutePath.Overview}
            aria-label={t('common.brand.home_aria')}
          >
            <span className="brand-mark" aria-hidden="true">
              k/
            </span>
            <span>kfind</span>
            <span className="brand-suffix">
              {t('common.brand.document_suffix')}
            </span>
          </NavLink>
          <div className="header-actions">
            <div
              aria-label={t('common.language.aria')}
              className="language-control"
              role="group"
            >
              <button
                aria-pressed={locale === DocumentLocale.Korean}
                onClick={() => {
                  void changeDocumentLocale(DocumentLocale.Korean);
                }}
                type="button"
              >
                {t('common.language.korean')}
              </button>
              <button
                aria-pressed={locale === DocumentLocale.English}
                onClick={() => {
                  void changeDocumentLocale(DocumentLocale.English);
                }}
                type="button"
              >
                {t('common.language.english')}
              </button>
            </div>
            <nav
              className="header-links"
              aria-label={t('common.header.external_aria')}
            >
              <a href="https://github.com/SeokminHong/kfind">GitHub</a>
              <a href="https://github.com/SeokminHong/kfind/blob/main/README.md">
                README
              </a>
            </nav>
          </div>
        </div>
      </header>

      <Collapsible.Root className="mobile-navigation">
        <Collapsible.Trigger>
          {t('common.mobile_navigation.trigger')}
        </Collapsible.Trigger>
        <Collapsible.Panel>
          <Navigation />
        </Collapsible.Panel>
      </Collapsible.Root>

      <div className="docs-shell">
        <aside className="docs-sidebar">
          <Navigation />
        </aside>
        <main className="docs-content" id="content">
          <Outlet />
          <footer className="docs-footer">
            <span>kfind 1.0.0-rc.1</span>
            <a href="https://github.com/SeokminHong/kfind/blob/main/LICENSE">
              {t('common.footer.license')}
            </a>
          </footer>
        </main>
      </div>
    </>
  );
}

export function DocumentLoading(): React.JSX.Element {
  const { t } = useDocumentTranslation();

  return (
    <main className="route-loading" role="status">
      {t('common.loading.document')}
    </main>
  );
}
