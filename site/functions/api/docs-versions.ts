import { readVersionManifest } from '../../server/versioned-site';

export const onRequestGet: PagesFunction<CloudflareBindings> = async (
  context,
) => {
  try {
    const stored = await readVersionManifest(context.env.KFIND_ASSETS);
    if (stored === undefined) {
      return Response.json(
        { error: 'document versions are not available' },
        { status: 404 },
      );
    }
    if (context.request.headers.get('If-None-Match') === stored.etag) {
      return new Response(null, {
        headers: { ETag: stored.etag },
        status: 304,
      });
    }
    return Response.json(stored.manifest, {
      headers: {
        'Cache-Control': 'no-cache',
        ETag: stored.etag,
        'X-Content-Type-Options': 'nosniff',
      },
    });
  } catch (error: unknown) {
    // eslint-disable-next-line no-console -- Cloudflare captures structured Worker logs.
    console.error(
      JSON.stringify({
        error: error instanceof Error ? error.message : String(error),
        message: 'document version manifest request failed',
      }),
    );
    return Response.json(
      { error: 'document versions are unavailable' },
      { status: 502 },
    );
  }
};
