import type { LinksFunction } from 'react-router';

import './theme.css';
import './site.css';
import './playground.css';

import {
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
  useLocation,
} from 'react-router';

import { initialDocumentLocale, useDocumentTranslation } from './app/i18n';
import { DocumentI18nProvider, DocumentLocaleSync } from './app/i18n-provider';
import { DocumentMetadataSync } from './app/metadata';
import { DocumentLoading } from './app/shell';

export const links: LinksFunction = () => [
  { rel: 'icon', href: '/favicon.ico', type: 'image/x-icon' },
  {
    rel: 'icon',
    href: '/favicon.svg',
    sizes: 'any',
    type: 'image/svg+xml',
  },
  {
    rel: 'apple-touch-icon',
    href: '/apple-touch-icon.png',
    sizes: '180x180',
  },
  {
    rel: 'stylesheet',
    href: 'https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/dist/web/variable/pretendardvariable-dynamic-subset.min.css',
    crossOrigin: 'anonymous',
  },
];

export function Layout({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const locale = initialDocumentLocale(location.search);

  return (
    <html lang={locale} dir="ltr">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="theme-color" content="#ffffff" />
        <Meta />
        <Links />
      </head>
      <body>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export function HydrateFallback(): React.JSX.Element {
  const location = useLocation();
  const locale = initialDocumentLocale(location.search);

  return (
    <DocumentI18nProvider initialLocale={locale}>
      <DocumentLoading />
    </DocumentI18nProvider>
  );
}

function SiteRoutes(): React.JSX.Element {
  const { t } = useDocumentTranslation();

  return (
    <>
      <DocumentLocaleSync />
      <DocumentMetadataSync />
      <a className="skip-link" href="#content">
        {t('common.skip_to_content')}
      </a>
      <Outlet />
    </>
  );
}

export default function App(): React.JSX.Element {
  const location = useLocation();
  const locale = initialDocumentLocale(location.search);

  return (
    <DocumentI18nProvider initialLocale={locale}>
      <SiteRoutes />
    </DocumentI18nProvider>
  );
}
