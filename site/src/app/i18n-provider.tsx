import { useEffect } from 'react';
import { I18nextProvider } from 'react-i18next';

import {
  defaultDocumentLocale,
  detectCookieLocale,
  getDocumentI18n,
  useDocumentTranslation,
} from './i18n';

export function DocumentI18nProvider({
  children,
}: {
  readonly children: React.ReactNode;
}): React.JSX.Element {
  return <I18nextProvider i18n={getDocumentI18n()}>{children}</I18nextProvider>;
}

export function DocumentLocaleSync(): null {
  const { i18n } = useDocumentTranslation();
  const locale = i18n.resolvedLanguage ?? defaultDocumentLocale;

  useEffect(() => {
    const cookieLocale = detectCookieLocale();

    if (cookieLocale !== undefined && cookieLocale !== i18n.resolvedLanguage) {
      void i18n.changeLanguage(cookieLocale);
    }
  }, [i18n]);

  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dir = i18n.dir(locale);
  }, [i18n, locale]);

  return null;
}
