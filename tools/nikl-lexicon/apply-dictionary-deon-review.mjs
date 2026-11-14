import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-deon-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'bf5963b4edf6082c7a4d44f3f26e1144d80a6354ead11826c31b072d6661457a') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '던:14') return line;
  if (row.query !== '이다' || !row.surface.endsWith('이던')) {
    throw new Error(`unexpected copula annotation: ${row.id}`);
  }
  corrected = true;
  return JSON.stringify({
    ...row,
    gold_byte_start: row.gold_byte_end - Buffer.byteLength('이던'),
  });
});
if (!corrected) throw new Error('missing dictionary annotation');
process.stdout.write(output.join('\n') + '\n');
