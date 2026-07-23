#!/usr/bin/env node

import { createRequire } from "node:module";
import { lstat, readFile, readdir } from "node:fs/promises";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import process from "node:process";

const require = createRequire(import.meta.url);
const { Kfind } = require("../node/kfind.js");

const packageRoot = new URL("../", import.meta.url);
const componentResourceUrl = new URL(
  "assets/morphology-component-compact.kfc",
  packageRoot,
);
const enrichedPredicatesUrl = new URL(
  "assets/predicates.enriched.tsv",
  packageRoot,
);
const packageMetadataUrl = new URL("package.json", packageRoot);
const excludedDirectoryNames = new Set([".git", "node_modules", "target"]);
const utf8Decoder = new TextDecoder("utf-8", { fatal: true });

process.stdout.on("error", (error) => {
  if (error.code === "EPIPE") {
    process.exit(0);
  }
  throw error;
});

class UsageError extends Error {}

function usage() {
  return `kfind - 한국어 표제어와 활용형 검색

사용법:
  kfind [옵션] <질의> [경로 ...]

질의:
  공백은 순서 구, |는 대안 검색입니다.
  shell에서 |를 사용할 때는 전체 질의를 따옴표로 묶습니다:
  kfind '걷다|사용자' src

옵션:
  --expand <literal|inflection|derivation>
  --boundary <smart|token|any>
  --pos <auto|noun|pronoun|numeral|verb|adjective|determiner|adverb|particle|interjection|literal>
  --normalization <nfc|canonical|none>
  --max-gap <정수>
  --literal                 --expand literal 단축 옵션
  --json                    match마다 JSON object 한 줄 출력
  -h, --help                도움말 출력
  -V, --version             버전 출력

경로가 없으면 TTY에서는 현재 디렉터리를, pipe에서는 표준 입력을 검색합니다.
`;
}

function optionValue(arguments_, index, name, inlineValue) {
  if (inlineValue !== undefined) {
    return [inlineValue, index];
  }
  const value = arguments_[index + 1];
  if (value === undefined) {
    throw new UsageError(`${name} 옵션에 값이 필요합니다.`);
  }
  return [value, index + 1];
}

function parseArguments(arguments_) {
  const compileOptions = {};
  const positional = [];
  let json = false;
  let optionsEnabled = true;

  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (optionsEnabled && argument === "--") {
      optionsEnabled = false;
      continue;
    }
    if (!optionsEnabled || !argument.startsWith("-") || argument === "-") {
      positional.push(argument);
      continue;
    }
    if (argument === "-h" || argument === "--help") {
      return { kind: "help" };
    }
    if (argument === "-V" || argument === "--version") {
      return { kind: "version" };
    }
    if (argument === "--literal") {
      compileOptions.literal = true;
      continue;
    }
    if (argument === "--json") {
      json = true;
      continue;
    }

    const equals = argument.indexOf("=");
    const name = equals === -1 ? argument : argument.slice(0, equals);
    const inlineValue = equals === -1 ? undefined : argument.slice(equals + 1);
    if (
      name === "--expand" ||
      name === "--boundary" ||
      name === "--pos" ||
      name === "--normalization"
    ) {
      const [value, valueIndex] = optionValue(
        arguments_,
        index,
        name,
        inlineValue,
      );
      compileOptions[
        name === "--normalization" ? "normalization" : name.slice(2)
      ] = value;
      index = valueIndex;
      continue;
    }
    if (name === "--max-gap") {
      const [value, valueIndex] = optionValue(
        arguments_,
        index,
        name,
        inlineValue,
      );
      if (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value))) {
        throw new UsageError("--max-gap에는 0 이상의 정수가 필요합니다.");
      }
      compileOptions.maxGap = Number(value);
      index = valueIndex;
      continue;
    }
    throw new UsageError(`알 수 없는 옵션입니다: ${argument}`);
  }

  const [query, ...paths] = positional;
  if (query === undefined || query.length === 0) {
    throw new UsageError("검색 질의가 필요합니다.");
  }
  return { kind: "search", compileOptions, json, paths, query };
}

function isExcludedDirectory(root, directory) {
  const name = directory.slice(directory.lastIndexOf(sep) + 1);
  if (excludedDirectoryNames.has(name)) {
    return true;
  }
  return relative(root, directory).split(sep).join("/") === "site/build";
}

async function collectDirectory(root, directory, files) {
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, "en"));

  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      continue;
    }
    if (entry.isDirectory()) {
      if (!isExcludedDirectory(root, path)) {
        await collectDirectory(root, path, files);
      }
      continue;
    }
    if (entry.isFile()) {
      files.add(path);
    }
  }
}

async function collectSources(paths) {
  const requested =
    paths.length > 0 ? paths : process.stdin.isTTY ? ["."] : ["-"];
  const files = new Set();
  let stdinRequested = false;

  for (const requestedPath of requested) {
    if (requestedPath === "-") {
      if (stdinRequested) {
        throw new UsageError("표준 입력은 한 번만 지정할 수 있습니다.");
      }
      stdinRequested = true;
      continue;
    }

    const absolutePath = resolve(requestedPath);
    const metadata = await lstat(absolutePath);
    if (metadata.isSymbolicLink()) {
      continue;
    }
    if (metadata.isDirectory()) {
      await collectDirectory(absolutePath, absolutePath, files);
    } else if (metadata.isFile()) {
      files.add(absolutePath);
    }
  }

  return { files: [...files].sort(), stdinRequested };
}

function displayPath(absolutePath) {
  const localPath = relative(process.cwd(), absolutePath);
  if (localPath === "") {
    return ".";
  }
  if (localPath.startsWith(`..${sep}`) || isAbsolute(localPath)) {
    return absolutePath;
  }
  return localPath.split(sep).join("/");
}

function decodeText(bytes, path) {
  if (bytes.includes(0)) {
    process.stderr.write(`kfind: binary file skipped: ${path}\n`);
    return undefined;
  }
  try {
    return utf8Decoder.decode(bytes);
  } catch {
    process.stderr.write(`kfind: invalid UTF-8 skipped: ${path}\n`);
    return undefined;
  }
}

function lineStarts(text) {
  const starts = [0];
  for (let index = 0; index < text.length; index += 1) {
    if (text.charCodeAt(index) === 10) {
      starts.push(index + 1);
    }
  }
  return starts;
}

function sourcePosition(starts, offset) {
  let low = 0;
  let high = starts.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (starts[middle] <= offset) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  const lineIndex = Math.max(0, low - 1);
  return {
    line: lineIndex + 1,
    column: offset - starts[lineIndex] + 1,
  };
}

function writeMatches(path, text, matches, json) {
  const starts = lineStarts(text);
  for (const match of matches) {
    const { line, column } = sourcePosition(starts, match.start);
    const surface = text.slice(match.start, match.end);
    if (json) {
      process.stdout.write(
        `${JSON.stringify({
          path,
          line,
          column,
          start: match.start,
          end: match.end,
          surface,
          atoms: match.atoms,
        })}\n`,
      );
    } else {
      const printableSurface = surface
        .replaceAll("\r", "\\r")
        .replaceAll("\n", "\\n");
      process.stdout.write(`${path}:${line}:${column}:${printableSurface}\n`);
    }
  }
  return matches.length;
}

async function compileMatcher(query, options) {
  const enrichedPredicates = await readFile(enrichedPredicatesUrl, "utf8");
  const engine = Kfind.withResources({ enrichedPredicates });
  try {
    return { engine, matcher: engine.compile(query, options) };
  } catch (error) {
    if (!String(error).includes("component resource is required")) {
      engine.free();
      throw error;
    }
    try {
      const component = await readFile(componentResourceUrl);
      engine.loadComponentResource(component);
      return { engine, matcher: engine.compile(query, options) };
    } catch (componentError) {
      engine.free();
      throw componentError;
    }
  }
}

async function readStandardInput() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

async function runSearch(command) {
  const { files, stdinRequested } = await collectSources(command.paths);
  const { engine, matcher } = await compileMatcher(
    command.query,
    command.compileOptions,
  );
  let matchCount = 0;

  try {
    if (stdinRequested) {
      const text = decodeText(await readStandardInput(), "<stdin>");
      if (text !== undefined) {
        matchCount += writeMatches(
          "<stdin>",
          text,
          matcher.findAll(text),
          command.json,
        );
      }
    }
    for (const file of files) {
      const path = displayPath(file);
      const text = decodeText(await readFile(file), path);
      if (text !== undefined) {
        matchCount += writeMatches(
          path,
          text,
          matcher.findAll(text),
          command.json,
        );
      }
    }
  } finally {
    matcher.free();
    engine.free();
  }

  return matchCount === 0 ? 1 : 0;
}

async function main() {
  try {
    const command = parseArguments(process.argv.slice(2));
    if (command.kind === "help") {
      process.stdout.write(usage());
      return 0;
    }
    if (command.kind === "version") {
      const metadata = JSON.parse(await readFile(packageMetadataUrl, "utf8"));
      process.stdout.write(`${metadata.version}\n`);
      return 0;
    }
    return await runSearch(command);
  } catch (error) {
    if (error instanceof UsageError) {
      process.stderr.write(`kfind: ${error.message}\n\n${usage()}`);
    } else {
      const message = error instanceof Error ? error.message : String(error);
      process.stderr.write(`kfind: ${message}\n`);
    }
    return 2;
  }
}

process.exitCode = await main();
