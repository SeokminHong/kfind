const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const [modulePath, declarationPath] = process.argv.slice(2);
if (!modulePath || !declarationPath) {
  throw new Error("usage: test-npm-package.cjs MODULE_PATH DECLARATION_PATH");
}

const { Kfind } = require(path.resolve(modulePath));
const engine = new Kfind();
assert.equal(engine.fullPosLoaded, false);

const matcher = engine.compile("걷다");
const text = "😀 길을 걸어 갔다.";
const matches = matcher.findAll(text);

assert.equal(matches.length, 1);
assert.equal(text.slice(matches[0].start, matches[0].end), "걸어");
assert.deepEqual(matches[0].atoms[0].origins[0].rulePath, [
  "lexical.d-to-l",
  "ending.aoeo",
]);

const literal = engine.compile("걸어", { literal: true });
assert.equal(literal.findAll("다시 걸어 보자.").length, 1);

assert.throws(() => engine.compile("", {}), /failed to compile query/);
assert.throws(
  () => engine.compile("걷다", { expand: "inflection", literal: true }),
  /literal conflicts with --expand/,
);
assert.throws(() => engine.compile("걷다", { unknown: true }), /unknown field/);
assert.throws(() => Kfind.withFullPos(new Uint8Array([0])), /failed to initialize/);

const declarations = fs.readFileSync(declarationPath, "utf8");
assert.match(declarations, /interface CompileOptions/);
assert.match(declarations, /compile\(query: string, options\?: CompileOptions\): Matcher/);
assert.match(declarations, /findAll\(text: string\): readonly Match\[\]/);

literal.free();
matcher.free();
engine.free();
