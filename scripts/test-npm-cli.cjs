const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const [cliPath] = process.argv.slice(2);
if (!cliPath) {
  throw new Error("usage: test-npm-cli.cjs CLI_PATH");
}

const fixtureDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "kfind-npm-"));
const nestedDirectory = path.join(fixtureDirectory, "nested");
fs.mkdirSync(nestedDirectory);
fs.writeFileSync(
  path.join(fixtureDirectory, "input.txt"),
  "😀 길을 걸어 갔다.\n권한을 확인했다.\n",
);
fs.writeFileSync(path.join(nestedDirectory, "other.txt"), "걷지 않았다.\n");

function execute(arguments_, options = {}) {
  return spawnSync(process.execPath, [path.resolve(cliPath), ...arguments_], {
    encoding: "utf8",
    ...options,
  });
}

try {
  const text = execute(["--pos", "verb", "걷다", fixtureDirectory]);
  assert.equal(text.status, 0, text.stderr);
  assert.match(text.stdout, /input\.txt:1:7:걸어/);
  assert.match(text.stdout, /nested\/other\.txt:1:1:걷지/);

  const json = execute([
    "--pos=verb",
    "--json",
    "걷다",
    path.join(fixtureDirectory, "input.txt"),
  ]);
  assert.equal(json.status, 0, json.stderr);
  const match = JSON.parse(json.stdout.trim());
  assert.equal(match.surface, "걸어");
  assert.equal(match.line, 1);
  assert.equal(match.column, 7);
  assert.deepEqual(match.atoms[0].origins[0].rulePath, [
    "lexical.d-to-l",
    "ending.aoeo",
  ]);

  const component = execute([
    "--json",
    "권한",
    path.join(fixtureDirectory, "input.txt"),
  ]);
  assert.equal(component.status, 0, component.stderr);
  assert.equal(JSON.parse(component.stdout.trim()).surface, "권한을");

  const disjunction = execute([
    "lit:걸어|lit:권한을",
    path.join(fixtureDirectory, "input.txt"),
  ]);
  assert.equal(disjunction.status, 0, disjunction.stderr);
  assert.match(
    disjunction.stdout,
    /input\.txt:1:7:걸어\n.*input\.txt:2:1:권한을\n$/s,
  );

  const stdin = execute(["--literal", "걸어"], {
    input: "다시 걸어 보자.\n",
  });
  assert.equal(stdin.status, 0, stdin.stderr);
  assert.equal(stdin.stdout, "<stdin>:1:4:걸어\n");

  const noMatch = execute([
    "--literal",
    "없음",
    path.join(fixtureDirectory, "input.txt"),
  ]);
  assert.equal(noMatch.status, 1, noMatch.stderr);
  assert.equal(noMatch.stdout, "");

  const invalid = execute(["--unknown", "걷다"]);
  assert.equal(invalid.status, 2);
  assert.match(invalid.stderr, /알 수 없는 옵션/);

  const help = execute(["--help"]);
  assert.equal(help.status, 0, help.stderr);
  assert.match(help.stdout, /kfind \[옵션\]/);
  assert.match(help.stdout, /kfind '걷다\|사용자' src/);

  const version = execute(["--version"]);
  assert.equal(version.status, 0, version.stderr);
  assert.equal(version.stdout, "1.0.0-rc.1\n");
} finally {
  fs.rmSync(fixtureDirectory, { force: true, recursive: true });
}
