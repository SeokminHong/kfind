const parameters = new URLSearchParams(location.search);
const profile = parameters.get("profile");
const copyProfile = profile === "component-copy" || profile === "full-pos-copy";
const packedProfile = profile?.startsWith("full-pos-packed-") ?? false;
const benchmarkProfile = copyProfile || packedProfile;

try {
  const totalStartedAt = performance.now();
  const moduleStartedAt = performance.now();
  const module = await import(
    benchmarkProfile ? "/benchmark-wasm/kfind.js" : "/release-wasm/kfind.js"
  );
  const wasm = await module.default();
  const moduleMilliseconds = performance.now() - moduleStartedAt;

  if (!(wasm.memory instanceof WebAssembly.Memory)) {
    throw new Error("WASM module did not expose linear memory");
  }

  const memoryCheckpoints = {
    after_module: memorySnapshot(wasm.memory),
  };
  const result = {
    profile,
    cache_mode: parameters.get("cache"),
    module_milliseconds: moduleMilliseconds,
    module_resources: resourceTimings([
      benchmarkProfile ? "/benchmark-wasm/kfind.js" : "/release-wasm/kfind.js",
      benchmarkProfile
        ? "/benchmark-wasm/kfind_bg.wasm"
        : "/release-wasm/kfind_bg.wasm",
    ]),
  };

  let profileResult;
  if (profile === "embedded") {
    const initializationStartedAt = performance.now();
    const engine = new module.Kfind();
    const engineInitializationMilliseconds =
      performance.now() - initializationStartedAt;
    profileResult = {
      engine,
      measurements: {
        engine_initialization_milliseconds: engineInitializationMilliseconds,
        total_milliseconds:
          moduleMilliseconds + engineInitializationMilliseconds,
      },
      memory: { after_engine: memorySnapshot(wasm.memory) },
    };
  } else if (profile === "component-copy" || profile === "full-pos-copy") {
    profileResult = await runCopyProfile(
      module,
      wasm.memory,
      profile === "component-copy" ? "/component.kfc" : "/full-pos.bin",
      moduleMilliseconds,
    );
  } else if (profile === "embedded-component") {
    profileResult = await runEmbeddedComponentProfile(
      module,
      wasm.memory,
      moduleMilliseconds,
    );
  } else if (profile === "full-pos") {
    profileResult = await runFullPosProfile(
      module,
      wasm.memory,
      moduleMilliseconds,
    );
  } else if (profile === "full-pos-component") {
    profileResult = await runFullPosComponentProfile(
      module,
      wasm.memory,
      moduleMilliseconds,
    );
  } else if (packedProfile) {
    const expectedArtifactSha256 = parameters.get("fullPosPackedSha256");
    if (!expectedArtifactSha256) {
      throw new Error("direct packed prototype SHA-256 is missing");
    }
    profileResult = await runFullPosPackedComponentProfile(
      module,
      wasm.memory,
      moduleMilliseconds,
      expectedArtifactSha256,
      profile === "full-pos-packed-validated-component",
    );
  } else {
    throw new Error(`unknown benchmark profile: ${profile}`);
  }

  Object.assign(result, profileResult.measurements);
  Object.assign(memoryCheckpoints, profileResult.memory);
  memoryCheckpoints.after_resource_release = await memorySnapshotAfterGc(
    wasm.memory,
  );
  result.memory = memoryCheckpoints;
  result.wasm_linear_peak_bytes = Math.max(
    ...Object.values(memoryCheckpoints).map(
      (checkpoint) => checkpoint.wasm_linear_bytes,
    ),
  );
  result.observed_wall_milliseconds = performance.now() - totalStartedAt;

  if (profileResult.engine) {
    profileResult.engine.free();
  }
  if (profileResult.fullPosIndex) {
    profileResult.fullPosIndex.free();
  }
  publish("result", result);
} catch (error) {
  publish("error", {
    message: error instanceof Error ? error.message : String(error),
    stack: error instanceof Error ? error.stack : null,
  });
}

async function runCopyProfile(
  module,
  memory,
  resourcePath,
  moduleMilliseconds,
) {
  const fetched = await fetchResource(resourcePath);
  const afterResourceBuffers = memorySnapshot(memory, [fetched.bytes]);
  const copyStartedAt = performance.now();
  const copiedLength = module.benchmarkCopyBytes(fetched.bytes);
  const copyMilliseconds = performance.now() - copyStartedAt;
  if (copiedLength !== fetched.bytes.byteLength) {
    throw new Error(
      `copy probe length mismatch: ${copiedLength} != ${fetched.bytes.byteLength}`,
    );
  }
  return {
    engine: null,
    measurements: {
      copy_milliseconds: copyMilliseconds,
      resource: fetched.timing,
      total_milliseconds:
        moduleMilliseconds +
        fetched.timing.total_milliseconds +
        copyMilliseconds,
    },
    memory: {
      after_copy: memorySnapshot(memory, [fetched.bytes]),
      after_resource_buffers: afterResourceBuffers,
    },
  };
}

async function runEmbeddedComponentProfile(module, memory, moduleMilliseconds) {
  const embeddedStartedAt = performance.now();
  const engine = new module.Kfind();
  const embeddedEngineMilliseconds = performance.now() - embeddedStartedAt;
  const fetched = await fetchResource("/component.kfc");
  const fetchWallMilliseconds = fetched.timing.total_milliseconds;
  const afterResourceBuffers = memorySnapshot(memory, [fetched.bytes]);
  const initializationStartedAt = performance.now();
  engine.loadComponentResource(fetched.bytes);
  const engineInitializationMilliseconds =
    performance.now() - initializationStartedAt;
  const optionalActivationMilliseconds =
    fetchWallMilliseconds + engineInitializationMilliseconds;
  return {
    engine,
    measurements: {
      component: fetched.timing,
      embedded_engine_initialization_milliseconds: embeddedEngineMilliseconds,
      engine_initialization_milliseconds: engineInitializationMilliseconds,
      fetch_wall_milliseconds: fetchWallMilliseconds,
      optional_activation_milliseconds: optionalActivationMilliseconds,
      total_milliseconds:
        moduleMilliseconds +
        embeddedEngineMilliseconds +
        optionalActivationMilliseconds,
    },
    memory: {
      after_engine_with_resource_buffers: memorySnapshot(memory, [
        fetched.bytes,
      ]),
      after_resource_buffers: afterResourceBuffers,
    },
  };
}

async function runFullPosProfile(module, memory, moduleMilliseconds) {
  const fetched = await fetchResource("/full-pos.bin");
  const afterResourceBuffers = memorySnapshot(memory, [fetched.bytes]);
  const initializationStartedAt = performance.now();
  const engine = module.Kfind.withFullPos(fetched.bytes);
  const engineInitializationMilliseconds =
    performance.now() - initializationStartedAt;
  const optionalActivationMilliseconds =
    fetched.timing.total_milliseconds + engineInitializationMilliseconds;
  return {
    engine,
    measurements: {
      engine_initialization_milliseconds: engineInitializationMilliseconds,
      fetch_wall_milliseconds: fetched.timing.total_milliseconds,
      full_pos: fetched.timing,
      optional_activation_milliseconds: optionalActivationMilliseconds,
      total_milliseconds: moduleMilliseconds + optionalActivationMilliseconds,
    },
    memory: {
      after_engine_with_resource_buffers: memorySnapshot(memory, [
        fetched.bytes,
      ]),
      after_resource_buffers: afterResourceBuffers,
    },
  };
}

async function runFullPosComponentProfile(module, memory, moduleMilliseconds) {
  const fetchStartedAt = performance.now();
  const [fullPos, component] = await Promise.all([
    fetchResource("/full-pos.bin"),
    fetchResource("/component.kfc"),
  ]);
  const fetchWallMilliseconds = performance.now() - fetchStartedAt;
  const afterResourceBuffers = memorySnapshot(memory, [
    fullPos.bytes,
    component.bytes,
  ]);
  const initializationStartedAt = performance.now();
  const engine = module.Kfind.withResources({
    component: component.bytes,
    fullPos: fullPos.bytes,
  });
  const engineInitializationMilliseconds =
    performance.now() - initializationStartedAt;
  const optionalActivationMilliseconds =
    fetchWallMilliseconds + engineInitializationMilliseconds;
  return {
    engine,
    measurements: {
      component: component.timing,
      engine_initialization_milliseconds: engineInitializationMilliseconds,
      fetch_wall_milliseconds: fetchWallMilliseconds,
      full_pos: fullPos.timing,
      optional_activation_milliseconds: optionalActivationMilliseconds,
      total_milliseconds: moduleMilliseconds + optionalActivationMilliseconds,
    },
    memory: {
      after_engine_with_resource_buffers: memorySnapshot(memory, [
        fullPos.bytes,
        component.bytes,
      ]),
      after_resource_buffers: afterResourceBuffers,
    },
  };
}

async function runFullPosPackedComponentProfile(
  module,
  memory,
  moduleMilliseconds,
  expectedArtifactSha256,
  fullValidation,
) {
  const embeddedStartedAt = performance.now();
  const engine = new module.Kfind();
  const embeddedEngineMilliseconds = performance.now() - embeddedStartedAt;
  const fetchStartedAt = performance.now();
  const [fullPos, component] = await Promise.all([
    fetchResource("/full-pos-packed.bin"),
    fetchResource("/component.kfc"),
  ]);
  const fetchWallMilliseconds = performance.now() - fetchStartedAt;
  const afterResourceBuffers = memorySnapshot(memory, [
    fullPos.bytes,
    component.bytes,
  ]);

  const fullPosStartedAt = performance.now();
  const fullPosIndex = new module.BenchmarkFullPosPacked(
    fullPos.bytes,
    expectedArtifactSha256,
    fullValidation,
  );
  const fullPosInitializationMilliseconds =
    performance.now() - fullPosStartedAt;
  const afterFullPosWithResourceBuffers = memorySnapshot(memory, [
    fullPos.bytes,
    component.bytes,
  ]);

  const componentStartedAt = performance.now();
  engine.loadComponentResource(component.bytes);
  const componentInitializationMilliseconds =
    performance.now() - componentStartedAt;
  const engineInitializationMilliseconds =
    embeddedEngineMilliseconds +
    fullPosInitializationMilliseconds +
    componentInitializationMilliseconds;
  const optionalActivationMilliseconds =
    fetchWallMilliseconds + engineInitializationMilliseconds;
  return {
    engine,
    fullPosIndex,
    measurements: {
      component: component.timing,
      component_initialization_milliseconds:
        componentInitializationMilliseconds,
      embedded_engine_initialization_milliseconds: embeddedEngineMilliseconds,
      engine_initialization_milliseconds: engineInitializationMilliseconds,
      fetch_wall_milliseconds: fetchWallMilliseconds,
      full_pos: fullPos.timing,
      full_pos_entry_count: fullPosIndex.entryCount,
      full_pos_initialization_milliseconds: fullPosInitializationMilliseconds,
      full_pos_lemma_count: fullPosIndex.lemmaCount,
      optional_activation_milliseconds: optionalActivationMilliseconds,
      total_milliseconds: moduleMilliseconds + optionalActivationMilliseconds,
    },
    memory: {
      after_engine_with_resource_buffers: memorySnapshot(memory, [
        fullPos.bytes,
        component.bytes,
      ]),
      after_full_pos_with_resource_buffers: afterFullPosWithResourceBuffers,
      after_resource_buffers: afterResourceBuffers,
    },
  };
}

async function fetchResource(path) {
  const startedAt = performance.now();
  const response = await fetch(path);
  const headersAt = performance.now();
  if (!response.ok) {
    throw new Error(`${path} download failed: HTTP ${response.status}`);
  }
  const arrayBufferStartedAt = performance.now();
  const bytes = new Uint8Array(await response.arrayBuffer());
  const completedAt = performance.now();
  const timing = resourceTimings([path])[0] ?? {};
  return {
    bytes,
    timing: {
      ...timing,
      bytes: bytes.byteLength,
      headers_milliseconds: headersAt - startedAt,
      array_buffer_milliseconds: completedAt - arrayBufferStartedAt,
      total_milliseconds: completedAt - startedAt,
    },
  };
}

function resourceTimings(paths) {
  return paths.map((path) => {
    const url = new URL(path, location.href).href;
    const entries = performance.getEntriesByName(url, "resource");
    const entry = entries.at(-1);
    return entry
      ? {
          name: path,
          duration_milliseconds: entry.duration,
          transfer_size_bytes: entry.transferSize,
          encoded_body_size_bytes: entry.encodedBodySize,
          decoded_body_size_bytes: entry.decodedBodySize,
        }
      : { name: path };
  });
}

function memorySnapshot(memory, keepAlive = []) {
  const heap = performance.memory;
  return {
    retained_resource_bytes: keepAlive.reduce(
      (total, bytes) => total + bytes.byteLength,
      0,
    ),
    used_js_heap_bytes: heap?.usedJSHeapSize ?? null,
    total_js_heap_bytes: heap?.totalJSHeapSize ?? null,
    wasm_linear_bytes: memory.buffer.byteLength,
  };
}

async function memorySnapshotAfterGc(memory) {
  globalThis.gc?.();
  await new Promise((resolve) => setTimeout(resolve, 0));
  return memorySnapshot(memory);
}

function publish(kind, value) {
  const json = JSON.stringify(value);
  const bytes = new TextEncoder().encode(json);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  document.documentElement.dataset[kind] = btoa(binary);
}
