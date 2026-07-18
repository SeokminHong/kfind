import type { LinksFunction } from 'react-router';

import './theme.css';
import './site.css';
import './playground.css';

import { Links, Meta, Scripts, ScrollRestoration } from 'react-router';

import { defaultDocumentLocale } from './app/i18n';
import { DocumentI18nProvider } from './app/i18n-provider';
import { DocumentLoading, Shell } from './app/shell';

export const links: LinksFunction = () => [
  { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' },
  {
    rel: 'stylesheet',
    href: 'https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/dist/web/variable/pretendardvariable-dynamic-subset.min.css',
    crossOrigin: 'anonymous',
  },
];

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <html lang={defaultDocumentLocale} dir="ltr">
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
  return (
    <DocumentI18nProvider>
      <DocumentLoading />
    </DocumentI18nProvider>
  );
}

export default function App(): React.JSX.Element {
  return (
    <DocumentI18nProvider>
      <Shell />
    </DocumentI18nProvider>
  );
}
