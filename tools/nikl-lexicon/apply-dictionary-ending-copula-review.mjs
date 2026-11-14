import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-ending-copula-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '1c27f7dd2195d37d2e44d009b8e99215d4a2ee348c4c05283ff7e136d09acd59') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const reviews = new Map();
const registry = readFileSync(new URL('./dictionary-ending-copula-review.tsv', import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, source, sourceId, surface, copula] = line.split('\t');
  if (!id || !source || !sourceId || !copula || reviews.has(id)
      || !surface.endsWith(copula) || surface === copula) {
    throw new Error(`invalid copula review: ${line}`);
  }
  reviews.set(id, { source, sourceId, surface, copula });
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;

  const { source, sourceId, surface, copula } = review;
  const text = Buffer.from(row.text);
  const start = row.gold_byte_end - Buffer.byteLength(copula);
  if (row.expected !== true || row.query !== '이다' || row.pos !== 'adjective'
      || row.source !== source || row.source_id !== sourceId || row.surface !== surface
      || row.gold_byte_start >= start
      || !text.subarray(row.gold_byte_start, row.gold_byte_end).equals(Buffer.from(surface))) {
    throw new Error(`unexpected copula annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, gold_byte_start: start });
});

if (reviews.size) throw new Error(`missing copula annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
