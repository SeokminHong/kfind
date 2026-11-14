import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-desiderative-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'caddc70312825f862c5f16c3c121a2aeb739999b399a9cd331090eabb2d7934d') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const reviews = new Map([
  ['아지이다:1', ['나아지다', '나아지이다', '낫다', 'verb']],
  ['어지이다:1', ['이루어지다', '이루어지이다', '이루다', 'verb']],
  ['어지이다:2', ['깊어지다', '깊어지이다', '깊다', 'adjective']],
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;

  const [query, surface, reviewedQuery, reviewedPos] = review;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.query !== query || row.pos !== 'verb' || row.expected !== true
      || row.source !== 'stdict' || row.surface !== surface || !span.equals(Buffer.from(surface))) {
    throw new Error(`unexpected desiderative annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, query: reviewedQuery, pos: reviewedPos });
});

if (reviews.size) throw new Error(`missing dictionary annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
