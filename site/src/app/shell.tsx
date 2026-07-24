import { Collapsible } from '@base-ui/react/collapsible';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { Link, Outlet, useLocation } from 'react-router';

import {
  changeDocumentLocale,
  DocumentLocale,
  useDocumentLocale,
  useDocumentTranslation,
} from './i18n';
import { DocumentLocaleSync } from './i18n-provider';
import { DocumentMetadataSync } from './metadata';
import {
  navigationGroupForPath,
  primaryNavigationItems,
  RoutePath,
  routePathFromPathname,
} from './navigation';

interface NavigationLocation {
  readonly hash: string;
  readonly pathname: RoutePath;
}

const serverNavigationLocation: NavigationLocation = {
  hash: '',
  pathname: RoutePath.Overview,
};
const activeSectionTop = 160;

function unsubscribeFromHydration(): void {
  // Hydration readiness has no external event source.
}

function subscribeToHydration(): () => void {
  return unsubscribeFromHydration;
}

function clientHydrated(): boolean {
  return true;
}

function serverHydrated(): boolean {
  return false;
}

function useNavigationLocation(): NavigationLocation {
  const location = useLocation();
  const hydrated = useSyncExternalStore(
    subscribeToHydration,
    clientHydrated,
    serverHydrated,
  );

  return hydrated
    ? {
        hash: location.hash,
        pathname: routePathFromPathname(location.pathname),
      }
    : serverNavigationLocation;
}

function useActiveSection(
  pathname: RoutePath,
  hash: string,
): string | undefined {
  const group = navigationGroupForPath(pathname);
  const sections = group.categories
    .flatMap((category) => category.pages)
    .find((item) => item.path === pathname)?.sections;
  const firstSection = sections?.[0]?.id;
  const [observedSection, setObservedSection] = useState<string>();
  const requestedTarget = hash.slice(1);
  const requestedSection = sections?.find(
    (section) => section.id === requestedTarget,
  )?.id;

  useEffect(() => {
    if (sections === undefined || sections.length === 0) {
      return;
    }

    const updateFromScroll = (): void => {
      let current = sections[0]?.id;
      for (const section of sections) {
        const element = document.querySelector<HTMLElement>(`#${section.id}`);
        if (
          element === null ||
          element.getBoundingClientRect().top > activeSectionTop
        ) {
          break;
        }
        current = section.id;
      }
      setObservedSection(current);
    };

    const alignRequestedSection = (): void => {
      if (requestedTarget.length > 0) {
        document
          .querySelector<HTMLElement>(`#${CSS.escape(requestedTarget)}`)
          ?.scrollIntoView();
      }
      updateFromScroll();
    };

    const scrollFrame = globalThis.requestAnimationFrame(alignRequestedSection);
    globalThis.addEventListener('scroll', updateFromScroll, { passive: true });
    globalThis.addEventListener('resize', alignRequestedSection);
    return () => {
      globalThis.cancelAnimationFrame(scrollFrame);
      globalThis.removeEventListener('scroll', updateFromScroll);
      globalThis.removeEventListener('resize', alignRequestedSection);
    };
  }, [pathname, requestedSection, requestedTarget, sections]);

  const observedIsCurrent =
    sections?.some((section) => section.id === observedSection) ?? false;
  return observedIsCurrent
    ? observedSection
    : (requestedSection ?? firstSection);
}

function PrimaryNavigation(): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const location = useNavigationLocation();
  const activeGroup = navigationGroupForPath(location.pathname);

  return (
    <nav
      aria-label={t('common.header.primary_aria')}
      className="primary-navigation"
    >
      {primaryNavigationItems.map((item) => {
        const group = navigationGroupForPath(item.path);
        const current = group.labelKey === activeGroup.labelKey;

        return (
          <Link
            aria-current={current ? 'page' : undefined}
            key={item.path}
            to={item.path}
          >
            {t(item.labelKey)}
          </Link>
        );
      })}
    </nav>
  );
}

function DocumentNavigation(): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();
  const location = useNavigationLocation();
  const activeSection = useActiveSection(location.pathname, location.hash);
  const group = navigationGroupForPath(location.pathname);

  return (
    <nav
      aria-label={t('common.navigation.toc_aria')}
      className="document-navigation"
    >
      <p className="document-navigation-title">{t(group.labelKey)}</p>
      {group.categories.map((category) => {
        const categoryKey = category.pages[0]?.path ?? category.label?.ko;

        return (
          <div className="document-navigation-category" key={categoryKey}>
            {category.label === undefined ? null : (
              <p className="document-navigation-category-title">
                {category.label[locale]}
              </p>
            )}
            {category.pages.map((item) => {
              const currentPage = item.path === location.pathname;

              return (
                <div className="document-navigation-page" key={item.path}>
                  <Link
                    aria-current={currentPage ? 'page' : undefined}
                    className="document-navigation-page-link"
                    to={item.path}
                  >
                    {item.label[locale]}
                  </Link>
                  {currentPage ? (
                    <ul className="document-section-links">
                      {item.sections.map((section) => (
                        <li key={section.id}>
                          <Link
                            aria-current={
                              activeSection === section.id
                                ? 'location'
                                : undefined
                            }
                            to={`${item.path}#${section.id}`}
                          >
                            {section.label[locale]}
                          </Link>
                        </li>
                      ))}
                    </ul>
                  ) : null}
                </div>
              );
            })}
          </div>
        );
      })}
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
          <Link
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
          </Link>
          <PrimaryNavigation />
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
              <Link className="header-cta" to={RoutePath.Playground}>
                {t('common.header.playground')}
              </Link>
              <a href="https://github.com/SeokminHong/kfind">GitHub</a>
            </nav>
          </div>
        </div>
      </header>

      <Collapsible.Root className="mobile-navigation">
        <Collapsible.Trigger>
          {t('common.mobile_navigation.trigger')}
        </Collapsible.Trigger>
        <Collapsible.Panel className="mobile-navigation-panel">
          <PrimaryNavigation />
          <DocumentNavigation />
          <nav
            aria-label={t('common.header.external_aria')}
            className="mobile-utilities"
          >
            <Link to={RoutePath.Playground}>
              {t('common.header.playground')}
            </Link>
            <a href="https://github.com/SeokminHong/kfind">GitHub</a>
          </nav>
        </Collapsible.Panel>
      </Collapsible.Root>

      <div className="docs-shell">
        <aside className="docs-sidebar">
          <DocumentNavigation />
        </aside>
        <main className="docs-content" id="content">
          <Outlet />
          <footer className="docs-footer">
            <span>kfind 1.0.0-rc.1</span>
            <a href="https://github.com/SeokminHong/kfind/blob/main/README.md">
              README
            </a>
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
