import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-copula-concessive-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '4af55d3b080fc129afa8b1dbd9f0f8592bcebde344e4c35fc06a752d6e03e00b') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const reviews = new Map([
  ['언마는:1', ['455825:455825001', '작가이언마는', '작가']],
  ['언만:1', ['232438:232438001', '배우이언만', '배우']],
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const [sourceId, surface, host] = review;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.query !== '이다' || row.pos !== 'adjective' || row.expected !== true
      || row.source !== 'stdict' || row.source_id !== sourceId || row.surface !== surface
      || !surface.startsWith(`${host}이`) || !span.equals(Buffer.from(surface))) {
    throw new Error(`unexpected concessive copula annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, gold_byte_start: row.gold_byte_start + Buffer.byteLength(host) });
});

if (reviews.size) throw new Error(`missing dictionary annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
