import type { TFunction } from 'i18next';
import type { MetaDescriptor, MetaFunction } from 'react-router';

import type { DocumentTranslationKey } from './translations.ko';

import { useEffect } from 'react';
import { useLocation } from 'react-router';

import {
  DocumentLocale,
  getDocumentTranslation,
  initialDocumentLocale,
  localizedDocumentHref,
  useDocumentLocale,
  useDocumentTranslation,
} from './i18n';
import {
  knownRoutePathFromPathname,
  navigationGroupForPath,
  navigationPageForPath,
  RoutePath,
} from './navigation';

const siteOrigin = 'https://kfind.pages.dev';
const socialImageUrl = new URL('/social-card.png', siteOrigin).href;

const openGraphLocale: Readonly<Record<DocumentLocale, string>> = {
  [DocumentLocale.English]: 'en_US',
  [DocumentLocale.Korean]: 'ko_KR',
};

interface DocumentMetadataKeys {
  readonly browserTitleKey?: DocumentTranslationKey;
  readonly titleKey?: DocumentTranslationKey;
  readonly descriptionKey?: DocumentTranslationKey;
}

const routeMetadataKeys: Readonly<
  Partial<Record<RoutePath, DocumentMetadataKeys>>
> = {
  [RoutePath.Overview]: {
    browserTitleKey: 'metadata.overview.browser_title',
    titleKey: 'metadata.overview.title',
    descriptionKey: 'metadata.overview.description',
  },
  [RoutePath.GettingStarted]: {
    titleKey: 'metadata.getting_started.title',
    descriptionKey: 'metadata.getting_started.description',
  },
  [RoutePath.Options]: {
    titleKey: 'metadata.options.title',
    descriptionKey: 'metadata.options.description',
  },
  [RoutePath.Glossary]: {
    titleKey: 'metadata.glossary.title',
    descriptionKey: 'metadata.glossary.description',
  },
  [RoutePath.Analysis]: {
    titleKey: 'metadata.analysis.title',
    descriptionKey: 'metadata.analysis.description',
  },
  [RoutePath.Agents]: {
    titleKey: 'metadata.agents.title',
    descriptionKey: 'metadata.agents.description',
  },
  [RoutePath.Architecture]: {
    titleKey: 'metadata.architecture.title',
    descriptionKey: 'metadata.architecture.description',
  },
  [RoutePath.Optimization]: {
    titleKey: 'metadata.optimization.title',
    descriptionKey: 'metadata.optimization.description',
  },
  [RoutePath.Benchmarks]: {
    titleKey: 'metadata.benchmarks.title',
    descriptionKey: 'metadata.benchmarks.description',
  },
  [RoutePath.Playground]: {
    titleKey: 'metadata.playground.title',
    descriptionKey: 'metadata.playground.description',
  },
  [RoutePath.BenchmarkCurrent]: {
    browserTitleKey: 'metadata.benchmark_current.browser_title',
  },
  [RoutePath.BenchmarkPerformance]: {
    browserTitleKey: 'metadata.benchmark_performance.browser_title',
  },
  [RoutePath.BenchmarkComparisons]: {
    browserTitleKey: 'metadata.benchmark_comparisons.browser_title',
  },
  [RoutePath.Irregulars]: {
    browserTitleKey: 'metadata.irregulars.browser_title',
  },
  [RoutePath.QueryLanguage]: {
    browserTitleKey: 'metadata.query_language.browser_title',
  },
  [RoutePath.Recipes]: {
    browserTitleKey: 'metadata.recipes.browser_title',
  },
};

interface DocumentMetadata {
  readonly browserTitle: string;
  readonly title: string;
  readonly description: string;
  readonly socialImageAlt: string;
  readonly breadcrumbParent?: {
    readonly path: RoutePath;
    readonly title: string;
  };
}

function breadcrumbParent(
  path: RoutePath,
  t: TFunction,
): DocumentMetadata['breadcrumbParent'] {
  if (navigationPageForPath(path) === undefined) {
    return undefined;
  }

  const group = navigationGroupForPath(path);
  const firstPage = group.categories.flatMap((category) => category.pages)[0];
  if (
    firstPage === undefined ||
    firstPage.path === RoutePath.Overview ||
    firstPage.path === path
  ) {
    return undefined;
  }

  return {
    path: firstPage.path,
    title: t(group.labelKey),
  };
}

function translateMetadata(
  path: RoutePath,
  t: TFunction,
  locale: DocumentLocale,
): DocumentMetadata {
  const parent = breadcrumbParent(path, t);
  const keys = routeMetadataKeys[path];
  const page = navigationPageForPath(path);
  const title =
    keys?.titleKey === undefined ? page?.label[locale] : t(keys.titleKey);
  const description =
    keys?.descriptionKey === undefined
      ? page?.description[locale]
      : t(keys.descriptionKey);
  if (title === undefined || description === undefined) {
    throw new Error(`metadata is unavailable for ${path}`);
  }

  return {
    browserTitle:
      keys?.browserTitleKey === undefined
        ? `${title} | kfind`
        : t(keys.browserTitleKey),
    title,
    description,
    socialImageAlt: t('metadata.social_image_alt'),
    breadcrumbParent: parent,
  };
}

function documentUrl(path: RoutePath, locale: DocumentLocale): string {
  return new URL(localizedDocumentHref(path, locale), siteOrigin).href;
}

function createStructuredData(
  path: RoutePath,
  metadata: DocumentMetadata,
  locale: DocumentLocale,
): MetaDescriptor {
  const canonicalUrl = documentUrl(path, locale);
  const homeUrl = documentUrl(RoutePath.Overview, locale);

  if (path === RoutePath.Overview) {
    return {
      'script:ld+json': {
        '@context': 'https://schema.org',
        '@type': 'WebSite',
        name: 'kfind',
        alternateName: ['kfind Korean Search', 'kfind 한국어 검색'],
        url: canonicalUrl,
      },
    };
  }

  const parent = metadata.breadcrumbParent;
  const currentPosition = parent === undefined ? 2 : 3;

  return {
    'script:ld+json': {
      '@context': 'https://schema.org',
      '@type': 'BreadcrumbList',
      itemListElement: [
        {
          '@type': 'ListItem',
          position: 1,
          name: 'kfind',
          item: homeUrl,
        },
        ...(parent === undefined
          ? []
          : [
              {
                '@type': 'ListItem',
                position: 2,
                name: parent.title,
                item: documentUrl(parent.path, locale),
              },
            ]),
        {
          '@type': 'ListItem',
          position: currentPosition,
          name: metadata.title,
          item: canonicalUrl,
        },
      ],
    },
  };
}

function createDescriptors(
  path: RoutePath,
  metadata: DocumentMetadata,
  locale: DocumentLocale,
): MetaDescriptor[] {
  const title = metadata.browserTitle;
  const canonicalUrl = documentUrl(path, locale);
  const koreanUrl = documentUrl(path, DocumentLocale.Korean);
  const englishUrl = documentUrl(path, DocumentLocale.English);
  const alternateLocale =
    locale === DocumentLocale.Korean
      ? DocumentLocale.English
      : DocumentLocale.Korean;

  return [
    { title },
    { name: 'description', content: metadata.description },
    { tagName: 'link', rel: 'canonical', href: canonicalUrl },
    {
      tagName: 'link',
      rel: 'alternate',
      hrefLang: DocumentLocale.Korean,
      href: koreanUrl,
    },
    {
      tagName: 'link',
      rel: 'alternate',
      hrefLang: DocumentLocale.English,
      href: englishUrl,
    },
    {
      tagName: 'link',
      rel: 'alternate',
      hrefLang: 'x-default',
      href: koreanUrl,
    },
    { property: 'og:title', content: title },
    { property: 'og:description', content: metadata.description },
    { property: 'og:type', content: 'website' },
    { property: 'og:url', content: canonicalUrl },
    { property: 'og:site_name', content: 'kfind' },
    {
      property: 'og:locale',
      content: openGraphLocale[locale],
    },
    {
      property: 'og:locale:alternate',
      content: openGraphLocale[alternateLocale],
    },
    { property: 'og:image', content: socialImageUrl },
    { property: 'og:image:type', content: 'image/png' },
    { property: 'og:image:width', content: '1200' },
    { property: 'og:image:height', content: '630' },
    { property: 'og:image:alt', content: metadata.socialImageAlt },
    { name: 'twitter:card', content: 'summary_large_image' },
    { name: 'twitter:title', content: title },
    { name: 'twitter:description', content: metadata.description },
    { name: 'twitter:image', content: socialImageUrl },
    { name: 'twitter:image:alt', content: metadata.socialImageAlt },
    createStructuredData(path, metadata, locale),
  ];
}

export function createDocumentMeta(path: RoutePath): MetaFunction {
  return ({ location }) => {
    const locale = initialDocumentLocale(location.search);

    return createDescriptors(
      path,
      translateMetadata(path, getDocumentTranslation(locale), locale),
      locale,
    );
  };
}

export const createLocationDocumentMeta: MetaFunction = ({ location }) => {
  const path = knownRoutePathFromPathname(location.pathname);
  if (path === undefined) {
    return [];
  }
  const locale = initialDocumentLocale(location.search);

  return createDescriptors(
    path,
    translateMetadata(path, getDocumentTranslation(locale), locale),
    locale,
  );
};

export const notFoundMeta: MetaFunction = ({ location }) => {
  const locale = initialDocumentLocale(location.search);
  const t = getDocumentTranslation(locale);

  return [
    { title: `${t('metadata.not_found.title')} · kfind` },
    {
      name: 'description',
      content: t('metadata.not_found.description'),
    },
    { name: 'robots', content: 'noindex' },
  ];
};

function setMetaContent(selector: string, content: string): void {
  document.querySelector(selector)?.setAttribute('content', content);
}

export function DocumentMetadataSync(): null {
  const { t } = useDocumentTranslation();
  const locale = useDocumentLocale();
  const location = useLocation();

  useEffect(() => {
    const path = knownRoutePathFromPathname(location.pathname);
    if (path === undefined) {
      return;
    }

    const metadata = translateMetadata(path, t, locale);
    const title = metadata.browserTitle;

    document.title = title;
    setMetaContent('meta[name="description"]', metadata.description);
    setMetaContent('meta[property="og:title"]', title);
    setMetaContent('meta[property="og:description"]', metadata.description);
    setMetaContent('meta[property="og:locale"]', openGraphLocale[locale]);
    setMetaContent('meta[property="og:image:alt"]', metadata.socialImageAlt);
    setMetaContent('meta[name="twitter:title"]', title);
    setMetaContent('meta[name="twitter:description"]', metadata.description);
    setMetaContent('meta[name="twitter:image:alt"]', metadata.socialImageAlt);
  }, [locale, location.pathname, location.search, t]);

  return null;
}
