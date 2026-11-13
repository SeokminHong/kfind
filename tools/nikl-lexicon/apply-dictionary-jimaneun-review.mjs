import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-jimaneun-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'd8d77a3329a8347ca8a463884f102bdd93d83d51aead1358e6ad62c0646d5bbf') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '지마는:12') return line;

  corrected++;
  return JSON.stringify({ ...row, query: '말씀드리다' });
});
if (corrected !== 1) throw new Error(`unexpected correction count: ${corrected}`);
process.stdout.write(output.join('\n') + '\n');
