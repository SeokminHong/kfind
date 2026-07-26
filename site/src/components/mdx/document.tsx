import type { ComponentType } from 'react';

import { createElement, lazy, Suspense } from 'react';
import { useLocation } from 'react-router';

import { useDocumentLocale } from '../../app/i18n';

import { mdxComponents } from './component-map';

interface MdxModule {
  readonly default: ComponentType<{
    readonly components?: typeof mdxComponents;
  }>;
}

const documentLoaders = import.meta.glob<MdxModule>('../../documents/**/*.mdx');
const documents = new Map(
  Object.entries(documentLoaders).map(([path, loader]) => [path, lazy(loader)]),
);

export function MdxDocument(): React.JSX.Element {
  const locale = useDocumentLocale();
  const { pathname } = useLocation();
  const normalizedPathname =
    pathname.length > 1 && pathname.endsWith('/')
      ? pathname.slice(0, -1)
      : pathname;
  const documentPath = `../../documents/${locale}${
    normalizedPathname === '/' ? '/index' : normalizedPathname
  }.mdx`;
  const Content = documents.get(documentPath);

  if (Content === undefined) {
    throw new Error(`MDX document is unavailable for ${pathname}`);
  }

  return (
    <Suspense fallback={<p className="route-loading">Loading…</p>}>
      {createElement(Content, { components: mdxComponents })}
    </Suspense>
  );
}
