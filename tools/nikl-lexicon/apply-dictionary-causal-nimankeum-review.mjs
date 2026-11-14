import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-causal-nimankeum-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '9b44f92068945bd764bb75ee4bd92145ebb2f50e40986deead8af796683eb5a7') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const explicitCopula = new Map([
  ['니만치:2', '이니만치'],
  ['니만큼:3', '이니만큼'],
  ['니만큼:5', '이니만큼'],
]);
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const suffix = explicitCopula.get(row.id);
  if (!suffix) return line;
  if (row.query !== '이다' || !row.surface.endsWith(suffix)) {
    throw new Error(`unexpected copula annotation: ${row.id}`);
  }
  explicitCopula.delete(row.id);
  return JSON.stringify({
    ...row,
    gold_byte_start: row.gold_byte_end - Buffer.byteLength(suffix),
  });
});
if (explicitCopula.size) throw new Error('missing dictionary annotations');
process.stdout.write(output.join('\n') + '\n');
