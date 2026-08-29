import { Collapsible } from '@base-ui/react/collapsible';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router';

import {
  cacheDocumentLocale,
  DocumentLocale,
  localizedDocumentHref,
  useDocumentLocale,
  useDocumentTranslation,
} from './i18n';
import {
  navigationGroupForPath,
  navigationPageForPath,
  primaryNavigationItems,
  RoutePath,
  routePathFromPathname,
} from './navigation';
import { currentDocumentVersion, siteAssetHref } from './site-build';
import { VersionSelector } from './version-selector/version-selector';

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

function PrimaryNavigation({
  currentPath,
}: {
  readonly currentPath: RoutePath;
}): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();
  const activeGroup =
    navigationPageForPath(currentPath) === undefined
      ? undefined
      : navigationGroupForPath(currentPath);

  return (
    <nav
      aria-label={t('common.header.primary_aria')}
      className="primary-navigation"
    >
      {primaryNavigationItems.map((item) => {
        const group = navigationGroupForPath(item.path);
        const current = group.labelKey === activeGroup?.labelKey;

        return (
          <Link
            aria-current={current ? 'page' : undefined}
            key={item.path}
            to={localizedDocumentHref(item.path, locale)}
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
                    to={localizedDocumentHref(item.path, locale)}
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
                            to={localizedDocumentHref(
                              `${item.path}#${section.id}`,
                              locale,
                            )}
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

function SiteHeader({
  currentPath,
}: {
  readonly currentPath: RoutePath;
}): React.JSX.Element {
  const { i18n, t } = useDocumentTranslation();
  const locale = useDocumentLocale();
  const location = useLocation();
  const navigate = useNavigate();
  const isPlayground = currentPath === RoutePath.Playground;

  const selectLocale = (nextLocale: DocumentLocale): void => {
    const currentHref = `${location.pathname}${location.search}${location.hash}`;
    const nextHref = localizedDocumentHref(currentHref, nextLocale);

    cacheDocumentLocale(nextLocale);
    if (nextHref === currentHref) {
      void i18n.changeLanguage(nextLocale);
      return;
    }

    void navigate(nextHref, { replace: true });
  };

  return (
    <header className="docs-header">
      <div className="header-inner">
        <Link
          className="brand"
          to={localizedDocumentHref(RoutePath.Overview, locale)}
          aria-label={t('common.brand.home_aria')}
        >
          <img
            alt=""
            aria-hidden="true"
            className="brand-mark"
            height="29"
            src={siteAssetHref('/favicon.svg')}
            width="29"
          />
          <span>kfind</span>
          <span className="brand-suffix">
            {isPlayground
              ? t('common.header.playground')
              : t('common.brand.document_suffix')}
          </span>
        </Link>
        <PrimaryNavigation currentPath={currentPath} />
        <div className="header-actions">
          <VersionSelector />
          <div
            aria-label={t('common.language.aria')}
            className="language-control"
            role="group"
          >
            <button
              aria-pressed={locale === DocumentLocale.Korean}
              onClick={() => {
                selectLocale(DocumentLocale.Korean);
              }}
              type="button"
            >
              {t('common.language.korean')}
            </button>
            <button
              aria-pressed={locale === DocumentLocale.English}
              onClick={() => {
                selectLocale(DocumentLocale.English);
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
            <Link
              aria-current={isPlayground ? 'page' : undefined}
              className="header-cta"
              to={localizedDocumentHref(RoutePath.Playground, locale)}
            >
              {t('common.header.playground')}
            </Link>
            <a href="https://github.com/SeokminHong/kfind">GitHub</a>
          </nav>
        </div>
      </div>
    </header>
  );
}

function DocumentMobileNavigation({
  currentPath,
}: {
  readonly currentPath: RoutePath;
}): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();

  return (
    <Collapsible.Root className="mobile-navigation">
      <Collapsible.Trigger>
        <span>{t('common.mobile_navigation.trigger')}</span>
        <svg
          aria-hidden="true"
          className="mobile-navigation-chevron"
          viewBox="0 0 16 16"
        >
          <path d="m3.5 6 4.5 4 4.5-4" />
        </svg>
      </Collapsible.Trigger>
      <Collapsible.Panel className="mobile-navigation-panel">
        <PrimaryNavigation currentPath={currentPath} />
        <DocumentNavigation />
        <nav
          aria-label={t('common.header.external_aria')}
          className="mobile-utilities"
        >
          <Link to={localizedDocumentHref(RoutePath.Playground, locale)}>
            {t('common.header.playground')}
          </Link>
          <a href="https://github.com/SeokminHong/kfind">GitHub</a>
        </nav>
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

function PlaygroundMobileNavigation(): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();

  return (
    <Collapsible.Root className="mobile-navigation">
      <Collapsible.Trigger>
        <span>{t('common.mobile_navigation.site_trigger')}</span>
        <svg
          aria-hidden="true"
          className="mobile-navigation-chevron"
          viewBox="0 0 16 16"
        >
          <path d="m3.5 6 4.5 4 4.5-4" />
        </svg>
      </Collapsible.Trigger>
      <Collapsible.Panel className="mobile-navigation-panel">
        <PrimaryNavigation currentPath={RoutePath.Playground} />
        <nav
          aria-label={t('common.header.external_aria')}
          className="mobile-utilities"
        >
          <Link
            aria-current="page"
            to={localizedDocumentHref(RoutePath.Playground, locale)}
          >
            {t('common.header.playground')}
          </Link>
          <a href="https://github.com/SeokminHong/kfind">GitHub</a>
        </nav>
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

function SiteFooter(): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();

  return (
    <footer className="docs-footer">
      <span>kfind {currentDocumentVersion}</span>
      <a href="https://github.com/SeokminHong/kfind/blob/main/README.md">
        README
      </a>
      <Link to={localizedDocumentHref(RoutePath.Licenses, locale)}>
        {t('common.footer.license')}
      </Link>
    </footer>
  );
}

export function DocumentShell(): React.JSX.Element {
  const location = useNavigationLocation();

  return (
    <>
      <SiteHeader currentPath={location.pathname} />
      <DocumentMobileNavigation currentPath={location.pathname} />
      <div className="docs-shell">
        <aside className="docs-sidebar">
          <DocumentNavigation />
        </aside>
        <main className="docs-content" id="content">
          <Outlet />
          <SiteFooter />
        </main>
      </div>
    </>
  );
}

export function PlaygroundShell(): React.JSX.Element {
  return (
    <>
      <SiteHeader currentPath={RoutePath.Playground} />
      <PlaygroundMobileNavigation />
      <div className="playground-shell">
        <main className="playground-content" id="content">
          <Outlet />
          <SiteFooter />
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
