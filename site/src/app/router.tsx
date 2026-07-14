import type { ComponentType } from 'react';
import type { LazyRouteFunction, NonIndexRouteObject } from 'react-router';

import { createBrowserRouter } from 'react-router';

import { RoutePath } from './navigation';
import { DocumentLoading, Shell } from './shell';

export interface DocumentRouteHandle {
  readonly title: string;
  readonly description: string;
}

interface PageModule {
  readonly default: ComponentType;
}

function lazyPage(
  load: () => Promise<PageModule>,
): LazyRouteFunction<NonIndexRouteObject> {
  return async () => {
    const module = await load();
    return { Component: module.default };
  };
}

const routeMetadata: Readonly<Record<RoutePath, DocumentRouteHandle>> = {
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

export const router = createBrowserRouter([
  {
    path: RoutePath.Overview,
    Component: Shell,
    HydrateFallback: DocumentLoading,
    children: [
      {
        index: true,
        handle: routeMetadata[RoutePath.Overview],
        lazy: lazyPage(async () => import('../pages/overview')),
      },
      {
        path: RoutePath.GettingStarted,
        handle: routeMetadata[RoutePath.GettingStarted],
        lazy: lazyPage(async () => import('../pages/getting-started')),
      },
      {
        path: RoutePath.Options,
        handle: routeMetadata[RoutePath.Options],
        lazy: lazyPage(async () => import('../pages/options')),
      },
      {
        path: RoutePath.Analysis,
        handle: routeMetadata[RoutePath.Analysis],
        lazy: lazyPage(async () => import('../pages/analysis')),
      },
      {
        path: RoutePath.Architecture,
        handle: routeMetadata[RoutePath.Architecture],
        lazy: lazyPage(async () => import('../pages/architecture')),
      },
      {
        path: RoutePath.Optimization,
        handle: routeMetadata[RoutePath.Optimization],
        lazy: lazyPage(async () => import('../pages/optimization')),
      },
      {
        path: RoutePath.Benchmarks,
        handle: routeMetadata[RoutePath.Benchmarks],
        lazy: lazyPage(async () => import('../pages/benchmarks')),
      },
      {
        path: RoutePath.Playground,
        handle: routeMetadata[RoutePath.Playground],
        lazy: lazyPage(async () => import('../pages/playground/page')),
      },
      {
        path: '*',
        handle: {
          title: '페이지를 찾을 수 없음',
          description: '요청한 kfind 문서 경로가 없습니다.',
        } satisfies DocumentRouteHandle,
        lazy: lazyPage(async () => import('../pages/not-found')),
      },
    ],
  },
]);
