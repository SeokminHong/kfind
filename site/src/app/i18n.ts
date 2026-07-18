import type { i18n, TFunction } from 'i18next';

import { createInstance } from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next, useTranslation } from 'react-i18next';

import { koreanTranslation } from './translations.ko';

export enum DocumentLocale {
  Korean = 'ko',
}

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

const documentI18n: i18n = createInstance();

documentI18n.use(initReactI18next).use(languageDetector);
void documentI18n.init({
  resources: {
    [DocumentLocale.Korean]: { translation: koreanTranslation },
  },
  lng: defaultDocumentLocale,
  fallbackLng: defaultDocumentLocale,
  supportedLngs: supportedDocumentLocales,
  load: 'languageOnly',
  keySeparator: false,
  returnNull: false,
  initAsync: false,
  interpolation: { escapeValue: false },
});

function isDocumentLocale(value: string | undefined): value is DocumentLocale {
  return supportedDocumentLocales.some((locale) => locale === value);
}

export function detectCookieLocale(): DocumentLocale | undefined {
  const detected = languageDetector.detect(['cookie']);
  const locale = Array.isArray(detected) ? detected[0] : detected;

  return isDocumentLocale(locale) ? locale : undefined;
}

export function getDocumentI18n(): typeof documentI18n {
  return documentI18n;
}

export function useDocumentTranslation(): ReturnType<typeof useTranslation> {
  return useTranslation();
}

export function getDocumentTranslation(
  locale: DocumentLocale = defaultDocumentLocale,
): TFunction {
  return documentI18n.getFixedT(locale);
}

export async function changeDocumentLocale(
  locale: DocumentLocale,
): Promise<void> {
  await documentI18n.changeLanguage(locale);
  languageDetector.cacheUserLanguage(locale, ['cookie']);
}
