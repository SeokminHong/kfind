import type { DocumentLocale } from './i18n';

import { useEffect, useMemo } from 'react';
import { I18nextProvider } from 'react-i18next';
import { useLocation } from 'react-router';

import {
  cacheDocumentLocale,
  createDocumentI18n,
  defaultDocumentLocale,
  detectCookieLocale,
  documentLocaleFromSearch,
  useDocumentTranslation,
} from './i18n';

export function DocumentI18nProvider({
  children,
  initialLocale,
}: {
  readonly children: React.ReactNode;
  readonly initialLocale: DocumentLocale;
}): React.JSX.Element {
  const i18n = useMemo(
    () => createDocumentI18n(initialLocale),
    [initialLocale],
  );

  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}

export function DocumentLocaleSync(): null {
  const { i18n } = useDocumentTranslation();
  const location = useLocation();
  const locale = i18n.resolvedLanguage ?? defaultDocumentLocale;

  useEffect(() => {
    const queryLocale = documentLocaleFromSearch(location.search);
    if (queryLocale !== undefined) {
      cacheDocumentLocale(queryLocale);
      if (queryLocale !== i18n.resolvedLanguage) {
        void i18n.changeLanguage(queryLocale);
      }
      return;
    }

    const cookieLocale = detectCookieLocale();

    if (cookieLocale !== undefined && cookieLocale !== i18n.resolvedLanguage) {
      void i18n.changeLanguage(cookieLocale);
    }
  }, [i18n, location.search]);

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dir = i18n.dir(locale);
  }, [i18n, locale]);

  return null;
}
