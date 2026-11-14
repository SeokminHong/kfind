import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-gu-exclamative-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '015d97b184dcfb36303fc55e07fbfafe1f44f6ac16d268277ca184fa5000e667') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const explicitCopula = new Map([
  ['구나:3', '이구나'],
  ['구나:8', '이구나'],
  ['구나:9', '이시구나'],
  ['구려:1', '이구려'],
  ['구려:10', '이구려'],
  ['더구먼:2', '이더구먼'],
]);
let correctedLemma = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id === '더구나:4') {
    if (row.query !== '되다' || row.surface !== '잘되더구나') {
      throw new Error(`unexpected predicate annotation: ${row.id}`);
    }
    correctedLemma = true;
    return JSON.stringify({ ...row, query: '잘되다' });
  }
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
if (explicitCopula.size || !correctedLemma) throw new Error('missing dictionary annotations');
process.stdout.write(output.join('\n') + '\n');
