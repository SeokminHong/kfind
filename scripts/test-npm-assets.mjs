import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readFile, stat } from "node:fs/promises";
import { createServer } from "node:http";

import {
  componentResourceFileUrl,
  enrichedPredicatesFileUrl,
} from "@kfind/kfind/assets";

const componentResourcePath = "/morphology-component-compact.kfc";
const enrichedSourcePath = process.argv[2];
const componentChecksumPath = process.argv[3];
assert(
  enrichedSourcePath && componentChecksumPath,
  "usage: test-npm-assets.mjs ENRICHED_SOURCE COMPONENT_CHECKSUM",
);

assert.equal(componentResourceFileUrl.protocol, "file:");
assert.equal(enrichedPredicatesFileUrl.protocol, "file:");

const componentResourceStat = await stat(componentResourceFileUrl);
const componentChecksum = (await readFile(componentChecksumPath, "utf8")).trim();

assert.deepEqual(
  await readFile(enrichedPredicatesFileUrl),
  await readFile(enrichedSourcePath),
);

const server = createServer((request, response) => {
  if (request.url !== componentResourcePath) {
    response.writeHead(404).end();
    return;
  }

  response.writeHead(200, {
    "Content-Length": componentResourceStat.size,
    "Content-Type": "application/octet-stream",
    "X-Content-Type-Options": "nosniff",
  });

  const stream = createReadStream(componentResourceFileUrl);
  stream.on("error", (error) => {
    response.destroy(error);
  });
  stream.pipe(response);
});

await new Promise((resolve, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolve);
});

try {
  const address = server.address();
  assert(address !== null && typeof address !== "string");

  const response = await fetch(
    `http://127.0.0.1:${address.port}${componentResourcePath}`,
  );
  assert.equal(response.status, 200);
  assert.equal(
    response.headers.get("content-type"),
    "application/octet-stream",
  );
  assert.equal(
    Number(response.headers.get("content-length")),
    componentResourceStat.size,
  );

  let streamedBytes = 0;
  const streamedHash = createHash("sha256");
  assert(response.body !== null);
  for await (const chunk of response.body) {
    streamedBytes += chunk.byteLength;
    streamedHash.update(chunk);
  }
  assert.equal(streamedBytes, componentResourceStat.size);
  assert.equal(streamedHash.digest("hex"), componentChecksum);
} finally {
  await new Promise((resolve, reject) => {
    server.close((error) => {
      if (error === undefined) {
        resolve();
      } else {
        reject(error);
      }
    });
  });
}

console.log(
  `npm asset resolver streamed ${componentResourceStat.size} component bytes`,
);
