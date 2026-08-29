import { serveVersionedSite } from '../server/versioned-site';

const englishLocaleQuery = 'en';
const englishAssetPrefix = '/_i18n/en';

function withContentLanguage(
  response: Response,
  language: 'en' | 'ko',
): Response {
  const headers = new Headers(response.headers);

  headers.set('Content-Language', language);
  return new Response(response.body, {
    headers,
    status: response.status,
    statusText: response.statusText,
  });
}

export const onRequest: PagesFunction<CloudflareBindings> = async (context) => {
  const url = new URL(context.request.url);

  if (url.pathname.startsWith('/versions/')) {
    try {
      return await serveVersionedSite(
        context.request,
        context.env.KFIND_ASSETS,
      );
    } catch (error: unknown) {
      // eslint-disable-next-line no-console -- Cloudflare captures structured Worker logs.
      console.error(
        JSON.stringify({
          error: error instanceof Error ? error.message : String(error),
          message: 'versioned site request failed',
          path: url.pathname,
        }),
      );
      return new Response('Versioned documentation is unavailable', {
        headers: {
          'Content-Type': 'text/plain; charset=utf-8',
          'X-Robots-Tag': 'noindex',
        },
        status: 502,
      });
    }
  }
  if (url.pathname.startsWith('/_i18n/')) {
    return new Response('Not Found', {
      headers: {
        'Content-Type': 'text/plain; charset=utf-8',
        'X-Robots-Tag': 'noindex',
      },
      status: 404,
    });
  }
  if (url.pathname.startsWith('/api/')) {
    return context.next();
  }
  if (url.searchParams.get('hl') !== englishLocaleQuery) {
    return withContentLanguage(await context.next(), 'ko');
  }

  const assetUrl = new URL(context.request.url);
  assetUrl.pathname =
    url.pathname === '/'
      ? `${englishAssetPrefix}/`
      : `${englishAssetPrefix}${url.pathname}`;
  assetUrl.search = '';
  assetUrl.hash = '';

  const response = await context.env.ASSETS.fetch(
    new Request(assetUrl, context.request),
  );
  return withContentLanguage(response, 'en');
};
