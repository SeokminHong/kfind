import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-rieulreoni-review.mjs CASES.jsonl');
}
const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== 'ac91e18f57a547ca95d65d6eedc3e90df344444b3bb44e06262072d7abd30bd6') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

let reviewed = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== 'ㄹ러니:1') return line;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (reviewed || row.source !== 'stdict' || row.source_id !== '99329:99329001'
      || row.query !== '이르다' || row.pos !== 'verb' || row.expected !== true
      || row.surface !== '사람일러니' || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected retrospective copula annotation: ${row.id}`);
  }
  reviewed = true;
  return JSON.stringify({
    ...row, query: '이다', pos: 'adjective',
    gold_byte_start: row.gold_byte_start + Buffer.byteLength('사람'),
  });
});
if (!reviewed) throw new Error('missing dictionary annotation: ㄹ러니:1');
process.stdout.write(output.join('\n') + '\n');
