import type { i18n, TFunction } from 'i18next';

import { createInstance } from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next, useTranslation } from 'react-i18next';

import { englishTranslation } from './translations.en';
import { koreanTranslation } from './translations.ko';

export enum DocumentLocale {
  English = 'en',
  Korean = 'ko',
}

declare const __KFIND_PRERENDER_LOCALE__: DocumentLocale;

export const defaultDocumentLocale = DocumentLocale.Korean;
export const supportedDocumentLocales: readonly DocumentLocale[] =
  Object.values(DocumentLocale);

const localeCookieName = 'kfind-document-locale';
const languageDetector = new LanguageDetector(undefined, {
  order: ['cookie'],
  caches: [],
  lookupCookie: localeCookieName,
  cookieMinutes: 365 * 24 * 60,
  cookieOptions: {
    path: '/',
    sameSite: 'lax',
    secure: import.meta.env.PROD,
  },
});

const resources = {
  [DocumentLocale.English]: { translation: englishTranslation },
  [DocumentLocale.Korean]: { translation: koreanTranslation },
} as const;

const translationI18n = createDocumentI18n(defaultDocumentLocale);

export function createDocumentI18n(locale: DocumentLocale): i18n {
  const instance = createInstance();

  instance.use(initReactI18next);
  void instance.init({
    resources,
    lng: locale,
    fallbackLng: defaultDocumentLocale,
    supportedLngs: supportedDocumentLocales,
    load: 'languageOnly',
    keySeparator: false,
    returnNull: false,
    initAsync: false,
    interpolation: { escapeValue: false },
  });
  return instance;
}

function isDocumentLocale(value: string | undefined): value is DocumentLocale {
  return supportedDocumentLocales.some((locale) => locale === value);
}

export function detectCookieLocale(): DocumentLocale | undefined {
  const detected = languageDetector.detect(['cookie']);
  const locale = Array.isArray(detected) ? detected[0] : detected;

  return isDocumentLocale(locale) ? locale : undefined;
}

export function cacheDocumentLocale(locale: DocumentLocale): void {
  languageDetector.cacheUserLanguage(locale, ['cookie']);
}

export function documentLocaleFromSearch(
  search: string,
): DocumentLocale | undefined {
  return new URLSearchParams(search).get('hl') === DocumentLocale.English
    ? DocumentLocale.English
    : undefined;
}

export function initialDocumentLocale(search: string): DocumentLocale {
  return (
    documentLocaleFromSearch(search) ??
    (import.meta.env.SSR ? __KFIND_PRERENDER_LOCALE__ : defaultDocumentLocale)
  );
}

export function localizedDocumentHref(
  href: string,
  locale: DocumentLocale,
): string {
  const url = new URL(href, 'https://kfind.pages.dev');

  if (locale === DocumentLocale.English) {
    url.searchParams.set('hl', DocumentLocale.English);
  } else {
    url.searchParams.delete('hl');
  }
  return `${url.pathname}${url.search}${url.hash}`;
}

export function useDocumentTranslation(): ReturnType<typeof useTranslation> {
  return useTranslation();
}

export function useDocumentLocale(): DocumentLocale {
  const { i18n } = useDocumentTranslation();
  const locale = i18n.resolvedLanguage;

  return isDocumentLocale(locale) ? locale : defaultDocumentLocale;
}

export function getDocumentTranslation(
  locale: DocumentLocale = defaultDocumentLocale,
): TFunction {
  return translationI18n.getFixedT(locale);
}

export async function changeDocumentLocale(
  i18n: i18n,
  locale: DocumentLocale,
): Promise<void> {
  await i18n.changeLanguage(locale);
  cacheDocumentLocale(locale);
}
