import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-anhta-pos-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '9d41ef2cac5673fed7e37c9a9ac851fcc0d779cf0d4c11913bd5e087121db235') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const auxiliaryAdjectiveIds = new Set([
  '거들랑:6', '아서야:6', '으니:15', '으려니와:2', '은가:6', '은가:11',
  '은걸:4', '은데:7', '은지:3', '은지:8', '은지:10', '을걸:16',
  '을는지:12', '을는지:14', '을지라도:5',
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (!auxiliaryAdjectiveIds.delete(row.id)) return line;
  if (row.query !== '않다' || row.pos !== 'verb') {
    throw new Error(`unexpected auxiliary annotation: ${row.id}`);
  }
  return JSON.stringify({ ...row, pos: 'adjective' });
});

if (auxiliaryAdjectiveIds.size) throw new Error('missing auxiliary annotations');
process.stdout.write(output.join('\n') + '\n');
