import type { TFunction } from 'i18next';
import type { MetaDescriptor, MetaFunction } from 'react-router';

import type { DocumentTranslationKey } from './translations.ko';

import { useEffect } from 'react';
import { useLocation } from 'react-router';

import { getDocumentTranslation, useDocumentTranslation } from './i18n';
import { RoutePath } from './navigation';

const siteOrigin = 'https://kfind.pages.dev';

interface DocumentMetadataKeys {
  readonly titleKey: DocumentTranslationKey;
  readonly descriptionKey: DocumentTranslationKey;
}

const routeMetadataKeys: Readonly<Record<RoutePath, DocumentMetadataKeys>> = {
  [RoutePath.Overview]: {
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
};

interface DocumentMetadata {
  readonly title: string;
  readonly description: string;
}

function translateMetadata(
  keys: DocumentMetadataKeys,
  t: TFunction,
): DocumentMetadata {
  return {
    title: t(keys.titleKey),
    description: t(keys.descriptionKey),
  };
}

function createDescriptors(
  path: RoutePath,
  metadata: DocumentMetadata,
): MetaDescriptor[] {
  const title = `${metadata.title} · kfind`;
  const canonicalUrl = new URL(path, siteOrigin).href;

  return [
    { title },
    { name: 'description', content: metadata.description },
    { tagName: 'link', rel: 'canonical', href: canonicalUrl },
    { property: 'og:title', content: title },
    { property: 'og:description', content: metadata.description },
    { property: 'og:type', content: 'website' },
    { property: 'og:url', content: canonicalUrl },
  ];
}

export function createDocumentMeta(path: RoutePath): MetaFunction {
  return () =>
    createDescriptors(
      path,
      translateMetadata(routeMetadataKeys[path], getDocumentTranslation()),
    );
}

export const notFoundMeta: MetaFunction = () => {
  const t = getDocumentTranslation();

  return [
    { title: `${t('metadata.not_found.title')} · kfind` },
    {
      name: 'description',
      content: t('metadata.not_found.description'),
    },
    { name: 'robots', content: 'noindex' },
  ];
};

function isRoutePath(pathname: string): pathname is RoutePath {
  return Object.values(RoutePath).includes(pathname as RoutePath);
}

function setMetaContent(selector: string, content: string): void {
  document.querySelector(selector)?.setAttribute('content', content);
}

export function DocumentMetadataSync(): null {
  const { t } = useDocumentTranslation();
  const location = useLocation();

  useEffect(() => {
    if (!isRoutePath(location.pathname)) {
      return;
    }

    const metadata = translateMetadata(routeMetadataKeys[location.pathname], t);
    const title = `${metadata.title} · kfind`;

    document.title = title;
    setMetaContent('meta[name="description"]', metadata.description);
    setMetaContent('meta[property="og:title"]', title);
    setMetaContent('meta[property="og:description"]', metadata.description);
  }, [location.pathname, t]);

  return null;
}
