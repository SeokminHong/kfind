import type { RouteConfig } from '@react-router/dev/routes';

import type { DocumentGroupIndex } from './app/document-index';
import type { RoutePath as RoutePathValue } from './app/navigation';

import { index, route } from '@react-router/dev/routes';

import {
  agentsGroup,
  benchmarksGroup,
  cliGroup,
  guideGroup,
  internalsGroup,
  referenceGroup,
} from './app/document-index';
import { RoutePath } from './app/navigation';

function technicalRoutes(
  group: DocumentGroupIndex,
  excludedPaths: ReadonlySet<RoutePathValue>,
) {
  return group.categories
    .flatMap((category) => category.pages)
    .filter((page) => !excludedPaths.has(page.path))
    .map((page) =>
      route(page.path, 'pages/document.tsx', {
        id: `technical-${page.path.split('/').join('-')}`,
      }),
    );
}

export default [
  index('pages/document.tsx'),
  route(RoutePath.GettingStarted, 'pages/document.tsx', {
    id: 'getting-started',
  }),
  route(RoutePath.Options, 'pages/document.tsx', { id: 'cli-overview' }),
  route(RoutePath.Agents, 'pages/document.tsx', { id: 'agents-overview' }),
  route(RoutePath.Glossary, 'pages/document.tsx', { id: 'glossary' }),
  route(RoutePath.Analysis, 'pages/document.tsx', {
    id: 'morphology-overview',
  }),
  route(RoutePath.Architecture, 'pages/document.tsx', {
    id: 'architecture-overview',
  }),
  route(RoutePath.Optimization, 'pages/document.tsx', {
    id: 'performance-overview',
  }),
  route(RoutePath.Benchmarks, 'pages/document.tsx', {
    id: 'benchmarks-overview',
  }),
  route(RoutePath.BenchmarkCurrent, 'pages/document.tsx', {
    id: 'benchmarks-current',
  }),
  ...technicalRoutes(guideGroup, new Set([RoutePath.GettingStarted])),
  ...technicalRoutes(cliGroup, new Set([RoutePath.Options])),
  ...technicalRoutes(agentsGroup, new Set([RoutePath.Agents])),
  ...technicalRoutes(
    internalsGroup,
    new Set([
      RoutePath.Analysis,
      RoutePath.Architecture,
      RoutePath.Optimization,
    ]),
  ),
  ...technicalRoutes(
    benchmarksGroup,
    new Set([RoutePath.Benchmarks, RoutePath.BenchmarkCurrent]),
  ),
  ...technicalRoutes(referenceGroup, new Set([RoutePath.Glossary])),
  route(RoutePath.Playground, 'pages/playground/page.tsx'),
  route('*', 'pages/not-found.tsx'),
] satisfies RouteConfig;
