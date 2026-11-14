import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-geonman-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'd5274badf13dc978969d32ad6b2c117e4e01811a3a5d3030b9a2b7590ec25e54') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const explicitCopula = new Map([
  ['건마는:1', '이건마는'],
  ['건마는:7', '이건마는'],
  ['건만:6', '이건만'],
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
