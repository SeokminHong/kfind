import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-auxiliary-pos-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '3244af368f136d555d9a39d9fffa0f1e8a960f840d4c1dda4008e41d4b9321a9') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const auxiliaryAdjectiveIds = new Set([
  'ㄴ데:12', '나:2', '다만:3', '려니와:9',
  '여라:2', '여라:6', '여라:8', '지마는:1', '지마는:10',
]);
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (!auxiliaryAdjectiveIds.delete(row.id)) return line;
  if (row.query !== '하다' || row.pos !== 'verb') {
    throw new Error(`unexpected auxiliary annotation: ${row.id}`);
  }
  return JSON.stringify({ ...row, pos: 'adjective' });
});
if (auxiliaryAdjectiveIds.size) throw new Error('missing auxiliary annotations');
process.stdout.write(output.join('\n') + '\n');
