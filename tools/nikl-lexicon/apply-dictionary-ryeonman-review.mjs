import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-ryeonman-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'e5695883e7d6c056fdc5b49f48c1ab8de1ecd5b9aae1856df94805773b6fbd0c') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '련마는:8') return line;

  corrected++;
  return JSON.stringify({ ...row, surface: '이련마는', gold_byte_start: 23 });
});
if (corrected !== 1) throw new Error(`unexpected correction count: ${corrected}`);
process.stdout.write(output.join('\n') + '\n');
