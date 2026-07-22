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
  file: string,
  excludedPaths: ReadonlySet<RoutePathValue>,
) {
  return group.categories
    .flatMap((category) => category.pages)
    .filter((page) => !excludedPaths.has(page.path))
    .map((page) =>
      route(page.path, file, {
        id: `technical-${page.path.split('/').join('-')}`,
      }),
    );
}

export default [
  index('pages/overview.tsx'),
  route(RoutePath.GettingStarted, 'pages/getting-started.tsx'),
  route(RoutePath.Options, 'pages/options.tsx'),
  route(RoutePath.Agents, 'pages/agents.tsx'),
  route(RoutePath.Glossary, 'pages/glossary.tsx'),
  route(RoutePath.Analysis, 'pages/analysis.tsx'),
  route(RoutePath.Architecture, 'pages/architecture.tsx'),
  route(RoutePath.Optimization, 'pages/optimization.tsx'),
  route(RoutePath.Benchmarks, 'pages/benchmarks.tsx', {
    id: 'benchmarks-overview',
  }),
  route(RoutePath.BenchmarkCurrent, 'pages/benchmarks.tsx', {
    id: 'benchmarks-current',
  }),
  ...technicalRoutes(
    guideGroup,
    'pages/technical/guide.tsx',
    new Set([RoutePath.GettingStarted]),
  ),
  ...technicalRoutes(
    cliGroup,
    'pages/technical/cli.tsx',
    new Set([RoutePath.Options]),
  ),
  ...technicalRoutes(
    agentsGroup,
    'pages/technical/agents.tsx',
    new Set([RoutePath.Agents]),
  ),
  ...technicalRoutes(
    internalsGroup,
    'pages/technical/internals.tsx',
    new Set([
      RoutePath.Analysis,
      RoutePath.Architecture,
      RoutePath.Optimization,
    ]),
  ),
  ...technicalRoutes(
    benchmarksGroup,
    'pages/technical/benchmarks.tsx',
    new Set([RoutePath.Benchmarks, RoutePath.BenchmarkCurrent]),
  ),
  ...technicalRoutes(
    referenceGroup,
    'pages/technical/reference.tsx',
    new Set([RoutePath.Glossary]),
  ),
  route(RoutePath.Playground, 'pages/playground/page.tsx'),
  route('*', 'pages/not-found.tsx'),
] satisfies RouteConfig;
