const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const profiles = [
  { name: "embedded", fullPos: false, component: false },
  { name: "embedded-component", fullPos: false, component: true },
  { name: "full-pos", fullPos: true, component: false },
  { name: "full-pos-component", fullPos: true, component: true },
];

if (process.argv[2] === "--probe") {
  runProbe(process.argv.slice(3));
} else {
  runBenchmark(process.argv.slice(2));
}

function runProbe(arguments_) {
  const [modulePath, componentPath, fullPosPath, profileName] = arguments_;
  const profile = profiles.find(({ name }) => name === profileName);
  if (!modulePath || !componentPath || !fullPosPath || !profile) {
    throw new Error(
      "usage: benchmark-npm-startup.cjs --probe MODULE COMPONENT FULL_POS PROFILE",
    );
  }
  if (typeof global.gc !== "function") {
    throw new Error("startup probe requires node --expose-gc");
  }

  const { Kfind } = require(path.resolve(modulePath));
  global.gc();

  const baseStarted = process.hrtime.bigint();
  let fullPosBytes;
  let engine;
  if (profile.fullPos) {
    fullPosBytes = fs.readFileSync(fullPosPath);
    engine = Kfind.withFullPos(fullPosBytes);
  } else {
    engine = new Kfind();
  }
  const baseInitializationSeconds = elapsedSeconds(baseStarted);
  fullPosBytes = undefined;
  global.gc();
  const baseRssBytes = process.memoryUsage().rss;

  let componentInitializationSeconds = null;
  if (profile.component) {
    const componentStarted = process.hrtime.bigint();
    let componentBytes = fs.readFileSync(componentPath);
    engine.loadComponentResource(componentBytes);
    componentInitializationSeconds = elapsedSeconds(componentStarted);
    componentBytes = undefined;
    global.gc();
  }
  const rssBytes = process.memoryUsage().rss;

  assert.equal(engine.fullPosLoaded, profile.fullPos);
  assert.equal(engine.componentResourceLoaded, profile.component);
  engine.free();

  process.stdout.write(
    JSON.stringify({
      profile: profile.name,
      base_initialization_seconds: baseInitializationSeconds,
      component_initialization_seconds: componentInitializationSeconds,
      initialization_seconds:
        baseInitializationSeconds + (componentInitializationSeconds ?? 0),
      base_rss_bytes: baseRssBytes,
      rss_bytes: rssBytes,
      rss_increase_bytes: rssBytes - baseRssBytes,
    }),
  );
}

function runBenchmark(arguments_) {
  const [modulePath, componentPath, fullPosPath, outputPath, runsText = "5"] =
    arguments_;
  const runs = Number(runsText);
  if (
    !modulePath ||
    !componentPath ||
    !fullPosPath ||
    !outputPath ||
    !Number.isSafeInteger(runs) ||
    runs < 3
  ) {
    throw new Error(
      "usage: benchmark-npm-startup.cjs MODULE COMPONENT FULL_POS OUTPUT RUNS>=3",
    );
  }

  const results = {};
  for (const profile of profiles) {
    executeProbe(modulePath, componentPath, fullPosPath, profile.name);
    const samples = Array.from({ length: runs }, () =>
      executeProbe(modulePath, componentPath, fullPosPath, profile.name),
    );
    results[profile.name] = aggregateSamples(samples);
  }

  const resolvedModulePath = path.resolve(modulePath);
  const report = {
    schema_version: 1,
    runtime: "wasm32-unknown-unknown via wasm-pack nodejs target",
    environment: {
      node: process.version,
      platform: process.platform,
      architecture: process.arch,
      wasm_bytes: fs.statSync(resolvedModulePath.replace(/\.js$/, "_bg.wasm")).size,
      component_bytes: fs.statSync(componentPath).size,
      full_pos_bytes: fs.statSync(fullPosPath).size,
    },
    warmup_runs: 1,
    measured_runs: runs,
    profiles: results,
  };
  const rendered = `${JSON.stringify(report, null, 2)}\n`;
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, rendered);
  process.stdout.write(rendered);
}

function executeProbe(modulePath, componentPath, fullPosPath, profile) {
  const result = spawnSync(
    process.execPath,
    [
      "--expose-gc",
      __filename,
      "--probe",
      modulePath,
      componentPath,
      fullPosPath,
      profile,
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `${profile} startup probe failed with exit ${result.status}: ${result.stderr.trim()}`,
    );
  }
  return JSON.parse(result.stdout);
}

function aggregateSamples(samples) {
  const metrics = [
    "base_initialization_seconds",
    "component_initialization_seconds",
    "initialization_seconds",
    "base_rss_bytes",
    "rss_bytes",
    "rss_increase_bytes",
  ];
  const result = { samples };
  for (const metric of metrics) {
    const values = samples
      .map((sample) => sample[metric])
      .filter((value) => value !== null);
    result[metric] = values.length === 0 ? null : median(values);
    result[`${metric}_min`] = values.length === 0 ? null : Math.min(...values);
    result[`${metric}_max`] = values.length === 0 ? null : Math.max(...values);
  }
  return result;
}

function median(values) {
  const ordered = [...values].sort((left, right) => left - right);
  const middle = Math.floor(ordered.length / 2);
  return ordered.length % 2 === 0
    ? (ordered[middle - 1] + ordered[middle]) / 2
    : ordered[middle];
}

function elapsedSeconds(started) {
  return Number(process.hrtime.bigint() - started) / 1_000_000_000;
}
