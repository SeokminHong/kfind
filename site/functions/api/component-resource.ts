const componentObjectKey = 'morphology-component-compact.kfc';

export const onRequestGet: PagesFunction<CloudflareBindings> = async (
  context,
) => {
  const object = await context.env.KFIND_ASSETS.get(componentObjectKey);

  if (object === null) {
    return Response.json(
      { error: 'component resource is not available' },
      { status: 404 },
    );
  }

  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set('Content-Type', 'application/octet-stream');
  headers.set('Content-Length', object.size.toString());
  headers.set('ETag', object.httpEtag);
  headers.set('X-Content-Type-Options', 'nosniff');

  return new Response(object.body, { headers });
};
