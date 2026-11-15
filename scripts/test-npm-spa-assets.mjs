import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readdir, readFile, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";

const distDirectory = process.argv[2];
const enrichedSourcePath = process.argv[3];
const componentChecksumPath = process.argv[4];
if (!distDirectory || !enrichedSourcePath || !componentChecksumPath) {
  throw new Error(
    "usage: test-npm-spa-assets.mjs DIST_DIRECTORY ENRICHED_SOURCE COMPONENT_CHECKSUM",
  );
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
const componentResourceSize = (await stat(componentAssetPath)).size;
const componentChecksum = (await readFile(componentChecksumPath, "utf8")).trim();

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
  const componentResponse = await fetch(`${origin}/assets/${componentAsset}`);
  assert.equal(componentResponse.status, 200);
  assert(componentResponse.body !== null);
  const componentHash = createHash("sha256");
  let streamedBytes = 0;
  for await (const chunk of componentResponse.body) {
    streamedBytes += chunk.byteLength;
    componentHash.update(chunk);
  }
  assert.equal(streamedBytes, componentResourceSize);
  assert.equal(componentHash.digest("hex"), componentChecksum);
  const enrichedResponse = await fetch(`${origin}/assets/${enrichedAsset}`);
  assert.equal(enrichedResponse.status, 200);
  assert.deepEqual(
    Buffer.from(await enrichedResponse.arrayBuffer()),
    await readFile(enrichedSourcePath),
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
