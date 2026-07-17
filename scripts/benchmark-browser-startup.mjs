import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, readFileSync, statSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

import { runChromeProbe } from "./lib/browser-benchmark.mjs";

const profiles = [
  "embedded",
  "component-copy",
  "full-pos-copy",
  "full-pos",
  "embedded-component",
  "full-pos-component",
  "full-pos-packed-attested-component",
  "full-pos-packed-validated-component",
];
const cacheModes = ["cold", "warm"];

const [
  releaseWasmDirectory,
  benchmarkWasmDirectory,
  componentPath,
  fullPosPath,
  fullPosPackedPath,
  pageModulePath,
  outputPath,
  runsText = "5",
  chromePath,
  revision,
] = process.argv.slice(2);
const runs = Number(runsText);

if (
  !releaseWasmDirectory ||
  !benchmarkWasmDirectory ||
  !componentPath ||
  !fullPosPath ||
  !fullPosPackedPath ||
  !pageModulePath ||
  !outputPath ||
  !chromePath ||
  !revision ||
  !Number.isSafeInteger(runs) ||
  runs < 3
) {
  throw new Error(
    "usage: benchmark-browser-startup.mjs RELEASE_WASM_DIR BENCHMARK_WASM_DIR COMPONENT FULL_POS FULL_POS_PACKED PAGE_MODULE OUTPUT RUNS>=3 CHROME REVISION",
  );
}

const assets = new Map([
  ["/release-wasm/kfind.js", path.join(releaseWasmDirectory, "kfind.js")],
  [
    "/release-wasm/kfind_bg.wasm",
    path.join(releaseWasmDirectory, "kfind_bg.wasm"),
  ],
  ["/benchmark-wasm/kfind.js", path.join(benchmarkWasmDirectory, "kfind.js")],
  [
    "/benchmark-wasm/kfind_bg.wasm",
    path.join(benchmarkWasmDirectory, "kfind_bg.wasm"),
  ],
  ["/benchmark-page.mjs", pageModulePath],
  ["/component.kfc", componentPath],
  ["/full-pos.bin", fullPosPath],
  ["/full-pos-packed.bin", fullPosPackedPath],
]);
const fullPosPackedSha256 = sha256File(fullPosPackedPath);

const server = createServer((request, response) => {
  const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
  setIsolationHeaders(response);

  if (requestUrl.pathname === "/" || requestUrl.pathname === "/index.html") {
    const body =
      '<!doctype html><html><head><meta charset="utf-8"></head>' +
      '<body><script type="module" src="/benchmark-page.mjs"></script></body></html>';
    response.writeHead(200, {
      "Cache-Control": "no-store",
      "Content-Length": Buffer.byteLength(body),
      "Content-Type": "text/html; charset=utf-8",
    });
    response.end(body);
    return;
  }

  const assetPath = assets.get(requestUrl.pathname);
  if (!assetPath) {
    response.writeHead(404);
    response.end("not found");
    return;
  }

  const stat = statSync(assetPath);
  response.writeHead(200, {
    "Cache-Control": "public, max-age=31536000, immutable",
    "Content-Length": stat.size,
    "Content-Type": contentType(assetPath),
    ETag: `"${stat.size.toString(16)}-${Math.trunc(stat.mtimeMs).toString(16)}"`,
  });
  createReadStream(assetPath).pipe(response);
});

await new Promise((resolve, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", resolve);
});

const address = server.address();
if (!address || typeof address === "string") {
  throw new Error("failed to resolve benchmark server address");
}

const browserVersion = await execute(chromePath, ["--version"]);
const results = {};

try {
  for (const profile of profiles) {
    results[profile] = {};
    for (const cacheMode of cacheModes) {
      const warmCacheDirectory =
        cacheMode === "warm" ? await createProfileDirectory() : null;
      try {
        const warmupProfileDirectory =
          warmCacheDirectory ?? (await createProfileDirectory());
        try {
          await runSample({
            cacheMode,
            chromePath,
            port: address.port,
            profile,
            profileDirectory: warmupProfileDirectory,
            sample: "warmup",
          });
        } finally {
          if (!warmCacheDirectory) {
            await rm(warmupProfileDirectory, { force: true, recursive: true });
          }
        }

        const samples = [];
        for (let index = 0; index < runs; index += 1) {
          const profileDirectory =
            warmCacheDirectory ?? (await createProfileDirectory());
          try {
            samples.push(
              await runSample({
                cacheMode,
                chromePath,
                port: address.port,
                profile,
                profileDirectory,
                sample: index + 1,
              }),
            );
          } finally {
            if (!warmCacheDirectory) {
              await rm(profileDirectory, { force: true, recursive: true });
            }
          }
        }
        results[profile][cacheMode] = aggregateSamples(samples);
      } finally {
        if (warmCacheDirectory) {
          await rm(warmCacheDirectory, { force: true, recursive: true });
        }
      }
    }
  }
} finally {
  await new Promise((resolve) => server.close(resolve));
}

const report = {
  schema_version: 1,
  revision,
  runtime: "wasm32-unknown-unknown web target in headless Chrome",
  environment: {
    browser: browserVersion.stdout.trim(),
    node: process.version,
    platform: process.platform,
    architecture: process.arch,
    server: "same-origin loopback HTTP without content encoding",
    production_bundle: {
      javascript: compressedSizes(path.join(releaseWasmDirectory, "kfind.js")),
      wasm: compressedSizes(path.join(releaseWasmDirectory, "kfind_bg.wasm")),
    },
    benchmark_bundle: {
      javascript: compressedSizes(
        path.join(benchmarkWasmDirectory, "kfind.js"),
      ),
      wasm: compressedSizes(path.join(benchmarkWasmDirectory, "kfind_bg.wasm")),
    },
    component: compressedSizes(componentPath),
    full_pos: compressedSizes(fullPosPath),
    full_pos_packed_prototype: compressedSizes(fullPosPackedPath),
  },
  warmup_runs: 1,
  measured_runs: runs,
  cache_modes: {
    cold: "fresh browser profile for every process",
    warm: "persistent HTTP cache across fresh browser processes",
  },
  profiles: results,
};

const rendered = `${JSON.stringify(report, null, 2)}\n`;
await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, rendered);

process.stdout.write(
  `${JSON.stringify(
    {
      output: path.resolve(outputPath),
      report_sha256: createHash("sha256").update(rendered).digest("hex"),
      revision,
      environment: report.environment,
      summary: summarize(results),
    },
    null,
    2,
  )}\n`,
);

async function runSample({
  cacheMode,
  chromePath: executable,
  port,
  profile,
  profileDirectory,
  sample,
}) {
  const target = new URL(`http://127.0.0.1:${port}/`);
  target.searchParams.set("cache", cacheMode);
  target.searchParams.set("profile", profile);
  target.searchParams.set("sample", String(sample));
  target.searchParams.set("fullPosPackedSha256", fullPosPackedSha256);

  try {
    const published = await runChromeProbe({
      executable,
      profileDirectory,
      target: target.href,
      timeoutMilliseconds: 120_000,
    });
    if (published.error) {
      throw new Error(decodeBase64(published.error));
    }
    return JSON.parse(decodeBase64(published.result));
  } catch (error) {
    throw new Error(
      `${profile}/${cacheMode}/${sample}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function execute(executable, arguments_) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, arguments_, {
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.once("error", reject);
    child.once("close", (code) => resolve({ code, stderr, stdout }));
  });
}

async function createProfileDirectory() {
  return mkdtemp(path.join(tmpdir(), "kfind-browser-startup-"));
}

function setIsolationHeaders(response) {
  response.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  response.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  response.setHeader("X-Content-Type-Options", "nosniff");
}

function contentType(filePath) {
  if (filePath.endsWith(".js") || filePath.endsWith(".mjs")) {
    return "text/javascript; charset=utf-8";
  }
  if (filePath.endsWith(".wasm")) {
    return "application/wasm";
  }
  return "application/octet-stream";
}

function compressedSizes(filePath) {
  const bytes = readFileSync(filePath);
  return {
    path: path.resolve(filePath),
    raw_bytes: bytes.byteLength,
    gzip_bytes: gzipSync(bytes, { level: 9 }).byteLength,
    brotli_bytes: brotliCompressSync(bytes, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
    }).byteLength,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

function sha256File(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

function aggregateSamples(samples) {
  const values = new Map();
  for (const sample of samples) {
    collectNumericValues(sample, "", values);
  }
  const metrics = {};
  for (const [name, metricValues] of [...values].sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    metrics[name] = {
      median: median(metricValues),
      min: Math.min(...metricValues),
      max: Math.max(...metricValues),
    };
  }
  return { metrics, samples };
}

function collectNumericValues(value, prefix, values) {
  if (typeof value === "number" && Number.isFinite(value)) {
    const metricValues = values.get(prefix) ?? [];
    metricValues.push(value);
    values.set(prefix, metricValues);
    return;
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    collectNumericValues(child, prefix ? `${prefix}.${key}` : key, values);
  }
}

function median(values) {
  const ordered = [...values].sort((left, right) => left - right);
  const middle = Math.floor(ordered.length / 2);
  return ordered.length % 2 === 0
    ? (ordered[middle - 1] + ordered[middle]) / 2
    : ordered[middle];
}

function decodeBase64(value) {
  return Buffer.from(value, "base64").toString("utf8");
}

function summarize(results_) {
  const summary = {};
  for (const [profile, cacheResults] of Object.entries(results_)) {
    summary[profile] = {};
    for (const [cacheMode, result] of Object.entries(cacheResults)) {
      summary[profile][cacheMode] = Object.fromEntries(
        [
          "module_milliseconds",
          "optional_activation_milliseconds",
          "copy_milliseconds",
          "engine_initialization_milliseconds",
          "component_initialization_milliseconds",
          "full_pos_initialization_milliseconds",
          "fetch_wall_milliseconds",
          "wasm_linear_peak_bytes",
        ]
          .filter((name) => result.metrics[name])
          .map((name) => [name, result.metrics[name].median]),
      );
    }
  }
  return summary;
}
