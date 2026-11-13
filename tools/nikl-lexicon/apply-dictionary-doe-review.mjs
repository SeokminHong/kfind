import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-doe-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'e42f6c66a4588485b3bb4bf25cebb57ab40fb5dc8f85497eb1c27b1929c1ae38') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const corrections = new Map([
  ['되:11', '잘하다'],
  ['되:15', '알아오다'],
]);
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const query = corrections.get(row.id);
  if (!query) return line;
  corrections.delete(row.id);
  return JSON.stringify({ ...row, query });
});
if (corrections.size) throw new Error('missing dictionary corrections');
process.stdout.write(output.join('\n') + '\n');
