import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-ppundeoreo-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'f09e800040022799a225fcc288854acc75ce26b4efcc78d0068830678079fa87') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const corrections = new Map([
  ['ㄹ뿐더러:2', { surface: '일뿐더러', gold_byte_start: 27 }],
  ['ㄹ뿐더러:5', { query: '잘하다' }],
]);
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const correction = corrections.get(row.id);
  if (!correction) return line;
  corrections.delete(row.id);
  return JSON.stringify({ ...row, ...correction });
});
if (corrections.size) throw new Error('missing dictionary corrections');
process.stdout.write(output.join('\n') + '\n');
