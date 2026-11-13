import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-deul-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'a3e452ecc5549608f7313f7102e6ae3f834ea371e079cd59f51d3b544e6eab3c') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== 'ㄴ들:4') return line;

  corrected++;
  return JSON.stringify({ ...row, surface: '인들', gold_byte_start: 29 });
});
if (corrected !== 1) throw new Error(`unexpected correction count: ${corrected}`);
process.stdout.write(output.join('\n') + '\n');
