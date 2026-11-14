import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-deodeul-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== '8cacf7375485a8504450a518121221121d905e03ca26ae985a1d32be66dd5a2e') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

let count = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '는지:9') return line;
  const target = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.source !== 'stdict' || row.source_id !== '409801:409801001'
      || row.query !== '뜨다' || row.pos !== 'verb' || row.expected !== true
      || row.surface !== '떠드는지' || !target.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected interrogative annotation: ${row.id}`);
  }
  count += 1;
  return JSON.stringify({ ...row, query: '떠들다' });
});
if (count !== 1) throw new Error(`unexpected number of reviewed cases: ${count}`);
process.stdout.write(output.join('\n') + '\n');
