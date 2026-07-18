import type { RouteConfig } from '@react-router/dev/routes';

import { index, route } from '@react-router/dev/routes';

import { RoutePath } from './app/navigation';

export default [
  index('pages/overview.tsx'),
  route(RoutePath.GettingStarted, 'pages/getting-started.tsx'),
  route(RoutePath.Options, 'pages/options.tsx'),
  route(RoutePath.Glossary, 'pages/glossary.tsx'),
  route(RoutePath.Analysis, 'pages/analysis.tsx'),
  route(RoutePath.Architecture, 'pages/architecture.tsx'),
  route(RoutePath.Optimization, 'pages/optimization.tsx'),
  route(RoutePath.Benchmarks, 'pages/benchmarks.tsx'),
  route(RoutePath.Playground, 'pages/playground/page.tsx'),
  route('*', 'pages/not-found.tsx'),
] satisfies RouteConfig;
