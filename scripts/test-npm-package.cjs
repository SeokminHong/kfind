const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const [modulePath, declarationPath, componentPath, enrichedPath] =
  process.argv.slice(2);
if (!modulePath || !declarationPath || !componentPath || !enrichedPath) {
  throw new Error(
    "usage: test-npm-package.cjs MODULE_PATH DECLARATION_PATH COMPONENT_PATH ENRICHED_PATH",
  );
}

const { Kfind } = require(path.resolve(modulePath));
const engine = new Kfind();
assert.equal(engine.fullPosLoaded, false);
assert.equal(engine.enrichedPredicatesLoaded, false);
assert.equal(engine.componentResourceLoaded, false);

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

const disjunction = engine.compile("lit:alpha|lit:beta");
assert.deepEqual(
  disjunction.findAll("beta then alpha").map(({ start, end }) => [start, end]),
  [
    [0, 4],
    [10, 15],
  ],
);
assert.throws(
  () => engine.compile("alpha | beta gamma"),
  /phrase atoms and `\|` alternatives cannot be mixed/,
);

assert.throws(() => engine.compile("", {}), /failed to compile query/);
assert.throws(
  () => engine.compile("걷다", { expand: "inflection", literal: true }),
  /literal conflicts with --expand/,
);
assert.throws(() => engine.compile("걷다", { unknown: true }), /unknown field/);
assert.throws(
  () => engine.compile("권한"),
  /component resource is required for this smart query/,
);

const componentResource = fs.readFileSync(componentPath);
const enrichedPredicates = fs.readFileSync(enrichedPath, "utf8");
engine.loadComponentResource(componentResource);
assert.equal(engine.componentResourceLoaded, true);

assert.throws(() => new Kfind(new Uint8Array([0])), /failed to initialize/);
assert.throws(
  () => Kfind.withFullPos(new Uint8Array([0])),
  /failed to initialize/,
);
assert.throws(
  () => engine.loadComponentResource(new Uint8Array([0])),
  /failed to initialize/,
);
assert.equal(engine.componentResourceLoaded, true);

assert.throws(
  () => Kfind.withResources({ unknown: true }),
  /invalid kfind resources: unknown field/,
);
assert.throws(
  () => Kfind.withResources({ component: "not bytes" }),
  /component.*Uint8Array/,
);
assert.throws(
  () => Kfind.withResources({ enrichedPredicates: new Uint8Array() }),
  /enrichedPredicates.*string/,
);
assert.throws(
  () => Kfind.withResources({ fullPos: new Uint8Array([0]) }),
  /failed to initialize/,
);
assert.throws(
  () => Kfind.withResources({ enrichedPredicates: "lemma\tpos\ninvalid\tVV\n" }),
  /failed to initialize/,
);

const enriched = Kfind.withResources({ enrichedPredicates });
assert.equal(enriched.fullPosLoaded, false);
assert.equal(enriched.enrichedPredicatesLoaded, true);
assert.equal(enriched.componentResourceLoaded, false);
const enrichedMatch = enriched.compile("가깝다", { boundary: "any" });
assert.equal(enrichedMatch.findAll("더 가까워졌다").length, 1);
enrichedMatch.free();
enriched.free();

const bundled = Kfind.withResources({
  enrichedPredicates,
  component: componentResource,
});
assert.equal(bundled.enrichedPredicatesLoaded, true);
assert.equal(bundled.componentResourceLoaded, true);
bundled.free();

const componentMatch = engine.compile("권한");
assert.equal(componentMatch.findAll("사용자권한").length, 1);
componentMatch.free();
const sourceComponent = engine.compile("학교");
assert.equal(sourceComponent.findAll("대학교").length, 1);
sourceComponent.free();

const preloaded = new Kfind(componentResource);
assert.equal(preloaded.componentResourceLoaded, true);
preloaded.free();

const declarations = fs.readFileSync(declarationPath, "utf8");
assert.match(declarations, /interface CompileOptions/);
assert.match(declarations, /interface ResourceBundle/);
assert.match(
  declarations,
  /constructor\(component_resource\?: Uint8Array \| null\)/,
);
assert.match(
  declarations,
  /withFullPos\(full_pos: Uint8Array, component_resource\?: Uint8Array \| null\)/,
);
assert.match(declarations, /withResources\(resources: ResourceBundle\): Kfind/);
assert.match(
  declarations,
  /loadComponentResource\(component_resource: Uint8Array\): void/,
);
assert.match(declarations, /readonly componentResourceLoaded: boolean/);
assert.match(declarations, /readonly enrichedPredicatesLoaded: boolean/);
assert.match(declarations, /compile\(query: string, options\?: CompileOptions\): Matcher/);
assert.match(declarations, /findAll\(text: string\): readonly Match\[\]/);

literal.free();
matcher.free();
engine.free();
