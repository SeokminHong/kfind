import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-familiar-final-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'd35bf8700c24aefacb5c1cc3cf162c8ec45d88105feab13e833de3158f03d54d') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = 0;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '이:2') return line;

  const reply = '그렇네. 내 아들이지만 정말 장하이.';
  const text = `${row.text}\n${reply}`;
  const surface = '장하이';
  const end = Buffer.byteLength(text) - Buffer.byteLength('.');
  corrected++;
  return JSON.stringify({
    ...row,
    query: '장하다',
    text,
    surface,
    gold_byte_start: end - Buffer.byteLength(surface),
    gold_byte_end: end,
  });
});
if (corrected !== 1) throw new Error(`unexpected correction count: ${corrected}`);
process.stdout.write(output.join('\n') + '\n');
