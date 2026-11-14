import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-gajyeodajuda-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== '1f8096bcf6ce08018ea5e6da5b8e05905730691714a4be04196688ff45bd5ee4') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

let count = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '게:47') return line;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.source !== 'stdict' || row.source_id !== '16621:16621001'
      || row.query !== '가지다' || row.pos !== 'verb' || row.expected !== true
      || row.surface !== '가져다주게' || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected compound annotation: ${row.id}`);
  }
  count += 1;
  return JSON.stringify({ ...row, query: '가져다주다' });
});
if (count !== 1) throw new Error(`unexpected number of reviewed cases: ${count}`);
process.stdout.write(output.join('\n') + '\n');
