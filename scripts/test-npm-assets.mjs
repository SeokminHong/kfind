import assert from "node:assert/strict";
import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import { createServer } from "node:http";

import {
  componentResourceFileUrl,
  enrichedPredicatesFileUrl,
} from "@kfind/kfind/assets";

const componentResourcePath = "/morphology-component-compact.kfc";
const componentResourceSize = 37_103_813;
const enrichedPredicatesSize = 42_910;

assert.equal(componentResourceFileUrl.protocol, "file:");
assert.equal(enrichedPredicatesFileUrl.protocol, "file:");

const componentResourceStat = await stat(componentResourceFileUrl);
const enrichedPredicatesStat = await stat(enrichedPredicatesFileUrl);

assert.equal(componentResourceStat.size, componentResourceSize);
assert.equal(enrichedPredicatesStat.size, enrichedPredicatesSize);

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
    componentResourceSize,
  );

  let streamedBytes = 0;
  assert(response.body !== null);
  for await (const chunk of response.body) {
    streamedBytes += chunk.byteLength;
  }
  assert.equal(streamedBytes, componentResourceSize);
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
  `npm asset resolver streamed ${componentResourceSize} component bytes`,
);
