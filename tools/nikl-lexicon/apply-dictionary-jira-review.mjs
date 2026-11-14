import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-jira-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'b361bce8702eadb9d935e739c0c5c7cd4334f33fc92df89c357c5b26ec7638ad') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

let corrected = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== 'ㄴ지라:1') return line;
  if (row.query !== '이다' || !row.surface.endsWith('인지라')) {
    throw new Error(`unexpected copula annotation: ${row.id}`);
  }
  corrected = true;
  return JSON.stringify({
    ...row,
    gold_byte_start: row.gold_byte_end - Buffer.byteLength('인지라'),
  });
});
if (!corrected) throw new Error('missing dictionary annotation');
process.stdout.write(output.join('\n') + '\n');
