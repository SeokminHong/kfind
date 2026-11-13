import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-formal-dap-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'a717f14452c1ab358c458e674d879792b98a27e98667ad1d6e5552b13b9dd806') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== 'ㄴ답니까:1') return line;

  corrected++;
  return JSON.stringify({ ...row, query: '좋아하다', pos: 'verb' });
});
if (corrected !== 1) throw new Error(`unexpected correction count: ${corrected}`);
process.stdout.write(output.join('\n') + '\n');
