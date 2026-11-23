import { createHash } from "node:crypto";
import fs from "node:fs";
import { pathToFileURL } from "node:url";

const [input, dir, lexicon, component, predicates] = process.argv.slice(2);
if (!input || !dir || !lexicon || !component || !predicates) {
  throw new Error("입력, WASM 패키지 디렉터리, full POS, component, 확장 용언 경로가 필요합니다.");
}
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");
const source = fs.readFileSync(input);
const rows = source.toString("utf8").trimEnd().split("\n").map(JSON.parse)
  .filter(({pos}) => pos === "verb" || pos === "adjective")
  .map(({id, query, pos, text}) => ({id, query, pos, text: text.normalize("NFD")}));
if (!rows.length) {
  throw new Error("측정할 동사·형용사 질의가 없습니다.");
}
const inputHash = sha256(JSON.stringify(rows));
const initStart = performance.now();
const { Kfind } = await import(pathToFileURL(`${dir}/kfind.js`));
const fullPos = fs.readFileSync(lexicon);
const resource = fs.readFileSync(component);
const enrichedPredicates = fs.readFileSync(predicates, "utf8");
const engine = Kfind.withResources({fullPos, component: resource, enrichedPredicates});
const initialization = performance.now() - initStart;

const timings = [];
const spans = [];
const start = performance.now();
for (const {id, query, pos, text} of rows) {
  const caseStart = performance.now();
  const queryMatcher = engine.compile(query, {boundary: "smart", pos, normalization: "canonical"});
  const result = queryMatcher.findAll(text);
  queryMatcher.free();
  timings.push(performance.now() - caseStart);
  spans.push([id, result.map(({start, end}) => [start, end])]);
}
const elapsed = performance.now() - start;
const peakRss = process.resourceUsage().maxRSS;
engine.free();

timings.sort((a, b) => a - b);
console.log(JSON.stringify({
  source_sha256: sha256(source), nfd_input_sha256: inputHash, rows: rows.length,
  full_pos_sha256: sha256(fullPos), component_sha256: sha256(resource),
  predicates_sha256: sha256(enrichedPredicates),
  wasm_sha256: sha256(fs.readFileSync(`${dir}/kfind_bg.wasm`)),
  result_sha256: sha256(JSON.stringify(spans)),
  initialization_ms: initialization, cases_per_second: rows.length / (elapsed / 1000),
  p95_ms: timings[Math.ceil(timings.length * 0.95) - 1], peak_rss_kib: peakRss,
}));
