import assert from "node:assert/strict";
import { createReadStream } from "node:fs";
import { readdir, readFile, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";

const distDirectory = process.argv[2];
if (distDirectory === undefined) {
  throw new Error("usage: test-npm-spa-assets.mjs DIST_DIRECTORY");
}

const absoluteDistDirectory = resolve(distDirectory);
const assetsDirectory = resolve(absoluteDistDirectory, "assets");
const assetNames = await readdir(assetsDirectory);
const componentAsset = exactlyOne(
  assetNames.filter(
    (name) =>
      name.startsWith("morphology-component-compact-") && name.endsWith(".kfc"),
  ),
  "bundled component asset",
);
const enrichedAsset = exactlyOne(
  assetNames.filter(
    (name) => name.startsWith("predicates.enriched-") && name.endsWith(".tsv"),
  ),
  "bundled enriched-predicate asset",
);
const entryScript = exactlyOne(
  assetNames.filter(
    (name) => name.startsWith("index-") && name.endsWith(".js"),
  ),
  "SPA entry script",
);

const componentAssetPath = resolve(assetsDirectory, componentAsset);
const enrichedAssetPath = resolve(assetsDirectory, enrichedAsset);
const componentResourceSize = 37_103_813;
const enrichedPredicatesSize = 42_910;

assert.equal((await stat(componentAssetPath)).size, componentResourceSize);
assert.equal((await stat(enrichedAssetPath)).size, enrichedPredicatesSize);

const entrySource = await readFile(
  resolve(assetsDirectory, entryScript),
  "utf8",
);
assert(entrySource.includes(componentAsset));
assert(entrySource.includes(enrichedAsset));

const routes = new Map([
  [
    "/",
    {
      contentType: "text/html; charset=utf-8",
      filePath: resolve(absoluteDistDirectory, "index.html"),
    },
  ],
  [
    `/assets/${componentAsset}`,
    { contentType: "application/octet-stream", filePath: componentAssetPath },
  ],
  [
    `/assets/${enrichedAsset}`,
    {
      contentType: "text/tab-separated-values; charset=utf-8",
      filePath: enrichedAssetPath,
    },
  ],
]);
const server = createServer((request, response) => {
  const route = routes.get(request.url ?? "");

  if (route === undefined) {
    response.writeHead(404).end();
    return;
  }

  response.writeHead(200, {
    "Content-Type": route.contentType,
    "X-Content-Type-Options": "nosniff",
  });
  const stream = createReadStream(route.filePath);
  stream.on("error", (error) => {
    response.destroy(error);
  });
  stream.pipe(response);
});

await new Promise((resolvePromise, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolvePromise);
});

try {
  const address = server.address();
  assert(address !== null && typeof address !== "string");
  const origin = `http://127.0.0.1:${address.port}`;

  assert.equal((await fetch(origin)).status, 200);
  assert.equal(
    await streamedByteLength(`${origin}/assets/${componentAsset}`),
    componentResourceSize,
  );
  assert.equal(
    await streamedByteLength(`${origin}/assets/${enrichedAsset}`),
    enrichedPredicatesSize,
  );
} finally {
  await new Promise((resolvePromise, reject) => {
    server.close((error) => {
      if (error === undefined) {
        resolvePromise();
      } else {
        reject(error);
      }
    });
  });
}

console.log(
  `Vite SPA emitted and served ${componentResourceSize} component bytes`,
);

function exactlyOne(values, label) {
  assert.equal(values.length, 1, `expected one ${label}: ${values.join(", ")}`);
  return values[0];
}

async function streamedByteLength(url) {
  const response = await fetch(url);
  assert.equal(response.status, 200);
  assert(response.body !== null);

  let byteLength = 0;
  for await (const chunk of response.body) {
    byteLength += chunk.byteLength;
  }
  return byteLength;
}
