import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-reon-review.mjs CASES.jsonl');
}
const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== '23a523049cc090b3dd6394dcbde7c2cd3bbec2c02cecfcd850f4d2866767c34d') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

const reviews = new Map([
  ['런가:1', ['421032:421032001', '이렇다', 'adjective', '고향', '이런가']],
  ['런들:1', ['424659:424659001', '이러다', 'verb', '사람', '이런들']],
]);
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const [sourceId, query, pos, nominal, copula] = review;
  const surface = nominal + copula;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.source !== 'stdict' || row.source_id !== sourceId || row.query !== query
      || row.pos !== pos || row.expected !== true || row.surface !== surface
      || !span.equals(Buffer.from(surface))) {
    throw new Error(`unexpected retrospective copula annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({
    ...row, query: '이다', pos: 'adjective',
    gold_byte_start: row.gold_byte_start + Buffer.byteLength(nominal),
  });
});
if (reviews.size) throw new Error(`missing dictionary annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
