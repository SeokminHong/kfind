import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-predicate-lemma-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'eea0253d969e5af109b945aa6f95776da44c550cfe9e4ea9cd8a45aa2c529fe2') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const reviews = new Map([
  ['ㄹ:15', ['하다', 'verb', '숨바꼭질할', '숨바꼭질하다', 'verb']],
  ['ㄹ거나:2', ['하다', 'verb', '한잔할거나', '한잔하다', 'verb']],
  ['ㅂ죠:5', ['하다', 'verb', '잘합죠', '잘하다', 'verb']],
  ['긴:5', ['하다', 'verb', '체하긴', '체하다', 'verb']],
  ['세요:9', ['이다', 'adjective', '저세요', '절다', 'verb']],
  ['여서야:3', ['하다', 'verb', '못하여서야', '못하다', 'verb']],
  ['으니라:2', ['않다', 'verb', '않으니라', '않다', 'adjective']],
  ['으면:10', ['하다', 'verb', '잘했으면', '잘하다', 'verb']],
  ['을걸:10', ['하다', 'verb', '잘했을걸', '잘하다', 'verb']],
  ['자고:6', ['하다', 'verb', '그만하자고', '그만하다', 'verb']],
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const [query, pos, surface, correctedQuery, correctedPos] = review;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.query !== query || row.pos !== pos || row.expected !== true
      || row.surface !== surface || !span.equals(Buffer.from(surface))) {
    throw new Error(`unexpected predicate annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, query: correctedQuery, pos: correctedPos });
});

if (reviews.size) throw new Error(`missing dictionary annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
