import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-danikka-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '6b33982873be248bae8b45c01e5b9304b0efdce1c329ba771a1ed04212dc1427') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const explicitCopula = new Set(['라니까:1', '라니까:2', '라니까:3']);
let correctedAuxiliary = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id === '다니까:5') {
    if (row.query !== '않다' || row.pos !== 'verb') {
      throw new Error(`unexpected auxiliary annotation: ${row.id}`);
    }
    correctedAuxiliary = true;
    return JSON.stringify({ ...row, pos: 'adjective' });
  }
  if (!explicitCopula.delete(row.id)) return line;
  if (row.query !== '이다' || !row.surface.endsWith('이라니까')) {
    throw new Error(`unexpected copula annotation: ${row.id}`);
  }
  return JSON.stringify({
    ...row,
    gold_byte_start: row.gold_byte_end - Buffer.byteLength('이라니까'),
  });
});
if (explicitCopula.size || !correctedAuxiliary) throw new Error('missing dictionary annotations');
process.stdout.write(output.join('\n') + '\n');
