import type { DocumentGroupIndex, DocumentPageIndex } from './document-index';
import type {
  RoutePath as RoutePathType,
  RoutePath as RoutePathValue,
} from './route-path';

import {
  agentsGroup,
  benchmarksGroup,
  cliGroup,
  guideGroup,
  homeGroup,
  internalsGroup,
  referenceGroup,
} from './document-index';
import { documentRoutePaths, RoutePath as RoutePaths } from './route-path';

export { documentRoutePaths } from './route-path';
export const RoutePath = RoutePaths;
export type RoutePath = RoutePathType;

export interface PrimaryNavigationItem {
  readonly labelKey: DocumentGroupIndex['labelKey'];
  readonly path: RoutePathValue;
}

export const navigationGroups: readonly DocumentGroupIndex[] = [
  homeGroup,
  guideGroup,
  cliGroup,
  agentsGroup,
  internalsGroup,
  benchmarksGroup,
  referenceGroup,
];

function pages(group: DocumentGroupIndex): readonly DocumentPageIndex[] {
  return group.categories.flatMap((category) => category.pages);
}

function firstNavigationItem(group: DocumentGroupIndex): DocumentPageIndex {
  const item = pages(group)[0];
  if (item === undefined) {
    throw new Error(`navigation group ${group.labelKey} has no pages`);
  }
  return item;
}

export const primaryNavigationItems: readonly PrimaryNavigationItem[] =
  navigationGroups.map((group) => ({
    labelKey: group.labelKey,
    path: firstNavigationItem(group).path,
  }));

export function knownRoutePathFromPathname(
  pathname: string,
): RoutePathValue | undefined {
  const normalized =
    pathname.length > 1 && pathname.endsWith('/')
      ? pathname.slice(0, -1)
      : pathname;
  const candidate = normalized as RoutePathValue;
  return documentRoutePaths.includes(candidate) ? candidate : undefined;
}

export function routePathFromPathname(pathname: string): RoutePathValue {
  return knownRoutePathFromPathname(pathname) ?? RoutePath.Overview;
}

export function navigationGroupForPath(
  pathname: RoutePathValue,
): DocumentGroupIndex {
  const group = navigationGroups.find((candidate) =>
    pages(candidate).some((item) => item.path === pathname),
  );
  if (group !== undefined) {
    return group;
  }

  const fallback = navigationGroups[0];
  if (fallback === undefined) {
    throw new Error('documentation navigation has no groups');
  }
  return fallback;
}

export function navigationPageForPath(
  pathname: RoutePathValue,
): DocumentPageIndex | undefined {
  return navigationGroups
    .flatMap((group) => pages(group))
    .find((item) => item.path === pathname);
}
