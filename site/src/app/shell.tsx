import type { DocumentRouteHandle } from './router';

import { useEffect } from 'react';
import { NavLink, Outlet, ScrollRestoration, useMatches } from 'react-router';

import { navigationGroups, RoutePath } from './navigation';

const defaultDescription =
  '한국어 표제어와 활용형을 찾는 text matcher kfind의 기술 문서와 WebAssembly playground';

function isDocumentRouteHandle(value: unknown): value is DocumentRouteHandle {
  if (typeof value !== 'object' || value === null) {
    return false;
  }

  return 'title' in value && 'description' in value;
}

function Navigation(): React.JSX.Element {
  return (
    <nav aria-label="문서 목차">
      {navigationGroups.map((group) => (
        <div className="navigation-group" key={group.label}>
          <p>{group.label}</p>
          {group.items.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === RoutePath.Overview}
            >
              {item.label}
            </NavLink>
          ))}
        </div>
      ))}
    </nav>
  );
}

export function Shell(): React.JSX.Element {
  const matches = useMatches();
  let handle: DocumentRouteHandle | undefined;

  for (const match of matches) {
    if (isDocumentRouteHandle(match.handle)) {
      handle = match.handle;
    }
  }

  useEffect(() => {
    const title = handle?.title ?? '문서';
    document.title = `${title} · kfind`;

    const description = document.querySelector('meta[name="description"]');
    description?.setAttribute(
      'content',
      handle?.description ?? defaultDescription,
    );
  }, [handle]);

  return (
    <>
      <a className="skip-link" href="#content">
        본문으로 건너뛰기
      </a>
      <header className="docs-header">
        <div className="header-inner">
          <NavLink
            className="brand"
            to={RoutePath.Overview}
            aria-label="kfind 문서 처음으로"
          >
            <span className="brand-mark" aria-hidden="true">
              k/
            </span>
            <span>kfind</span>
            <span className="brand-suffix">문서</span>
          </NavLink>
          <nav className="header-links" aria-label="외부 문서">
            <a href="https://github.com/SeokminHong/kfind">GitHub</a>
            <a href="https://github.com/SeokminHong/kfind/blob/main/README.ko.md">
              README
            </a>
          </nav>
        </div>
      </header>

      <details className="mobile-navigation">
        <summary>문서 메뉴</summary>
        <Navigation />
      </details>

      <div className="docs-shell">
        <aside className="docs-sidebar">
          <Navigation />
        </aside>
        <main className="docs-content" id="content">
          <Outlet />
          <footer className="docs-footer">
            <span>kfind 0.3.0-rc.2</span>
            <a href="https://github.com/SeokminHong/kfind/blob/main/LICENSE">
              MIT License
            </a>
          </footer>
        </main>
      </div>
      <ScrollRestoration />
    </>
  );
}

export function DocumentLoading(): React.JSX.Element {
  return (
    <main className="route-loading" role="status">
      문서를 불러오는 중…
    </main>
  );
}
