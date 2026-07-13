const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const [modulePath, declarationPath, componentPath] = process.argv.slice(2);
if (!modulePath || !declarationPath || !componentPath) {
  throw new Error(
    "usage: test-npm-package.cjs MODULE_PATH DECLARATION_PATH COMPONENT_PATH",
  );
}

const { Kfind } = require(path.resolve(modulePath));
const componentResource = fs.readFileSync(componentPath);
const engine = new Kfind(componentResource);
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
assert.throws(() => new Kfind(new Uint8Array([0])), /failed to initialize/);
assert.throws(
  () => Kfind.withFullPos(componentResource, new Uint8Array([0])),
  /failed to initialize/,
);

const componentMatch = engine.compile("권한");
assert.equal(componentMatch.findAll("사용자권한").length, 1);
componentMatch.free();
const crossingSubstring = engine.compile("학교");
assert.equal(crossingSubstring.findAll("대학교").length, 0);
crossingSubstring.free();

const declarations = fs.readFileSync(declarationPath, "utf8");
assert.match(declarations, /interface CompileOptions/);
assert.match(declarations, /constructor\(component_resource: Uint8Array\)/);
assert.match(
  declarations,
  /withFullPos\(component_resource: Uint8Array, full_pos: Uint8Array\)/,
);
assert.match(declarations, /compile\(query: string, options\?: CompileOptions\): Matcher/);
assert.match(declarations, /findAll\(text: string\): readonly Match\[\]/);

literal.free();
matcher.free();
engine.free();
