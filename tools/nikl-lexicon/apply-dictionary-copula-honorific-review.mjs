import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-copula-honorific-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '599c7274c2cb5107b0a1604013788f96516e5a28014f424a4a2f7167cf9507ed') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let reviewed = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== 'ㄴ:13') return line;

  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (reviewed || row.query !== '이다' || row.pos !== 'adjective' || row.expected !== true
      || row.source !== 'krdict' || row.source_id !== '78634' || row.surface !== '선생님이신'
      || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected honorific copula annotation: ${row.id}`);
  }
  reviewed = true;
  return JSON.stringify({ ...row, gold_byte_start: row.gold_byte_start + Buffer.byteLength('선생님') });
});

if (!reviewed) throw new Error('missing dictionary annotation: ㄴ:13');
process.stdout.write(output.join('\n') + '\n');
