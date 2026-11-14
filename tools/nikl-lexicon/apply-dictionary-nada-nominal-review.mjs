import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-nada-nominal-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '6cf0042bcc8ca58e6729c40e67306023fded20039742357711d0d4c619c84e31') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const reviews = new Map([
  ['ㄹ라:7', ['나다', '병날라', '병나다', 'verb']],
  ['네:14', ['나다', '병나겠네', '병나다', 'verb']],
  ['ㅁ:1', ['기다', '김', '아니다', 'adjective']],
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const [query, surface, correctedQuery, pos] = review;
  const text = Buffer.from(row.text);
  if (row.query !== query || row.pos !== 'verb' || row.expected !== true
      || row.surface !== surface
      || !text.subarray(row.gold_byte_start, row.gold_byte_end).equals(Buffer.from(surface))) {
    throw new Error(`unexpected dictionary annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  if (row.id === 'ㅁ:1') {
    // The example illustrates the nominalized negative copula and its subject particle.
    const nominalized = Buffer.from('아님이');
    const start = text.indexOf(nominalized);
    if (start < 0 || text.indexOf(nominalized, start + nominalized.length) !== -1) {
      throw new Error('expected one nominalized negative copula: ㅁ:1');
    }
    return JSON.stringify({
      ...row, query: correctedQuery, pos, surface: nominalized.toString(),
      gold_byte_start: start, gold_byte_end: start + nominalized.length,
    });
  }
  return JSON.stringify({ ...row, query: correctedQuery, pos });
});

if (reviews.size) throw new Error(`missing dictionary annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
