declare const __KFIND_SITE_VERSION__: string;
declare const __KFIND_SITE_BASE_PATH__: string;

export const currentDocumentVersion = __KFIND_SITE_VERSION__;

const baseUrl =
  __KFIND_SITE_BASE_PATH__ === '/' ? '/' : `${__KFIND_SITE_BASE_PATH__}/`;

export function siteAssetHref(path: string): string {
  if (!path.startsWith('/')) {
    throw new Error(`site asset path must start with a slash: ${path}`);
  }
  return `${baseUrl}${path.slice(1)}`;
}

export function isVersionedSiteBuild(): boolean {
  return /^\/versions\/[^/]+\/$/u.test(baseUrl);
}
