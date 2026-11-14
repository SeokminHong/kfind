import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-respectful-rio-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'b71de3ffffcb86a27c668093bdd5615a17beb39091389a005c22d3cd6aa29bcd') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let reviewed = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '리오:3') return line;

  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (reviewed || row.query !== '오다' || row.pos !== 'verb' || row.expected !== true
      || row.source !== 'krdict' || row.source_id !== '79282' || row.surface !== '읽으오리오'
      || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected respectful ending annotation: ${row.id}`);
  }
  reviewed = true;
  return JSON.stringify({ ...row, query: '읽다' });
});

if (!reviewed) throw new Error('missing dictionary annotation: 리오:3');
process.stdout.write(output.join('\n') + '\n');
