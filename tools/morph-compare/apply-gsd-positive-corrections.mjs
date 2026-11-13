import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-positive-corrections.mjs GSD_POSITIVES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'a6955363dbf837f16b5ec4ef6465eabf8e6611df43c591a21588aa638efde129') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

function tsv(path, width) {
  return readFileSync(new URL(path, import.meta.url), 'utf8').trimEnd()
    .split('\n').slice(1).map(line => {
      const fields = line.split('\t');
      if (fields.length !== width) throw new Error(`invalid TSV row: ${line}`);
      return fields;
    });
}

const corrections = new Map();
for (const [id, fixture, reason] of tsv('./gsd-positive-input-corrections.tsv', 3)) {
  if (corrections.has(id) || !reason.trim()) throw new Error(`invalid correction: ${id}`);
  corrections.set(id, fixture);
}

const fixtures = new Map();
const fixtureIds = new Set(corrections.values());
for (const row of tsv('../../data/fixtures/ud_morphology_cases.tsv', 7)) {
  if (!fixtureIds.has(row[0])) continue;
  if (fixtures.has(row[0])) throw new Error(`duplicate correction fixture: ${row[0]}`);
  fixtures.set(row[0], row);
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const fixture = corrections.get(row.id);
  if (!fixture) return line;
  const [, query, pos, expectation, start, end, text] = fixtures.get(fixture) ?? [];
  if (query !== row.query || pos !== row.pos || expectation !== 'smart-exact'
      || !fixture.startsWith(`corpus.ud-gsd.test.${row.id.split(':')[1]}.`)
      || row.expected !== true) {
    throw new Error(`correction identity differs: ${row.id} / ${fixture}`);
  }
  const span = Buffer.from(text).subarray(Number(start), Number(end));
  if (!span.length || span.length !== Number(end) - Number(start)
      || !span.equals(Buffer.from(span.toString('utf8')))) {
    throw new Error(`invalid correction span: ${fixture}`);
  }
  corrections.delete(row.id);
  return JSON.stringify({
    id: row.id, query, pos, text, expected: true,
    gold_byte_start: Number(start), gold_byte_end: Number(end),
  });
});
if (corrections.size) throw new Error(`unused corrections: ${[...corrections.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
