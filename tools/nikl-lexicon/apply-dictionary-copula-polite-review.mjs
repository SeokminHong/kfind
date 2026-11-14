import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-copula-polite-review.mjs CASES.jsonl');
}
const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== 'fcdcf745b1d43c1e6fdf1cf4feca328a42139863b66a5fb4fd4f095c5635520b') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

let reviewed = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '셔요:4') return line;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (reviewed || row.source !== 'stdict' || row.source_id !== '191144:191144001'
      || row.query !== '이다' || row.pos !== 'adjective' || row.expected !== true
      || row.surface !== '은인이셔요' || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected polite copula annotation: ${row.id}`);
  }
  reviewed = true;
  return JSON.stringify({
    ...row, gold_byte_start: row.gold_byte_start + Buffer.byteLength('은인'),
  });
});
if (!reviewed) throw new Error('missing dictionary annotation: 셔요:4');
process.stdout.write(output.join('\n') + '\n');
