import type { MetaDescriptor, MetaFunction } from 'react-router';

import { RoutePath } from './navigation';

const siteOrigin = 'https://kfind.pages.dev';

interface DocumentMetadata {
  readonly title: string;
  readonly description: string;
}

const routeMetadata: Readonly<Record<RoutePath, DocumentMetadata>> = {
  [RoutePath.Overview]: {
    title: '개요',
    description: 'kfind의 목적, 검색 모델과 사용 경로를 설명합니다.',
  },
  [RoutePath.GettingStarted]: {
    title: '시작하기',
    description: 'kfind 설치와 첫 형태 검색 방법을 설명합니다.',
  },
  [RoutePath.Options]: {
    title: '쿼리와 옵션',
    description: '확장, 경계, 품사, 정규화와 구 검색 옵션을 설명합니다.',
  },
  [RoutePath.Glossary]: {
    title: '단어장',
    description: 'kfind 문서에서 사용하는 검색과 형태 분석 용어를 정의합니다.',
  },
  [RoutePath.Analysis]: {
    title: '형태 분석',
    description:
      'Query에서 표면형과 verifier를 만드는 형태 분석 원리를 설명합니다.',
  },
  [RoutePath.Architecture]: {
    title: '아키텍처',
    description:
      'Query compile부터 anchor scan, verifier와 출력까지의 구조를 설명합니다.',
  },
  [RoutePath.Optimization]: {
    title: '설계와 최적화',
    description: '검색 계획과 실행 엔진의 성능 설계를 설명합니다.',
  },
  [RoutePath.Benchmarks]: {
    title: '벤치마크',
    description: 'kfind의 품질과 성능 측정 계약을 설명합니다.',
  },
  [RoutePath.Playground]: {
    title: 'Playground',
    description: '브라우저에서 kfind WebAssembly 검색을 실행합니다.',
  },
};

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
  return () => createDescriptors(path, routeMetadata[path]);
}

export const notFoundMeta: MetaFunction = () => [
  { title: '페이지를 찾을 수 없음 · kfind' },
  { name: 'description', content: '요청한 kfind 문서 경로가 없습니다.' },
  { name: 'robots', content: 'noindex' },
];
