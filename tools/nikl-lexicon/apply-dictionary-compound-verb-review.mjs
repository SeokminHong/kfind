import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-compound-verb-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'dd2ca7f70d6f369aa67edfe0d6d5437e4500436548c92294836780c1c9d6eccc') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const reviews = new Map([
  ['ㄹ꼬:13', ['살다', '먹고살꼬', '먹고살다']],
  ['ㄹ세라:12', ['채다', '눈치챌세라', '눈치채다']],
  ['ㅂ시다:3', ['돕다', '도와줍시다', '도와주다']],
  ['련:8', ['돕다', '도와주련', '도와주다']],
  ['리다:5', ['드리다', '말씀드리리다', '말씀드리다']],
  ['세나:3', ['두다', '그만두세나', '그만두다']],
  ['십사:2', ['돕다', '도와주십사', '도와주다']],
  ['우:3', ['돕다', '도와주우', '도와주다']],
  ['으라고:1', ['입다', '갈아입으라고', '갈아입다']],
  ['을라고:1', ['채다', '눈치챘을라고', '눈치채다']],
  ['을세라:3', ['채다', '눈치챘을세라', '눈치채다']],
  ['자고:2', ['살다', '먹고살자고', '먹고살다']],
  ['자니까:3', ['오다', '다녀오자니까', '다녀오다']],
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;

  const [query, surface, correctedQuery] = review;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.query !== query || row.pos !== 'verb' || !row.expected
      || row.surface !== surface || !span.equals(Buffer.from(surface))) {
    throw new Error(`unexpected compound verb annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, query: correctedQuery });
});

if (reviews.size) throw new Error(`missing dictionary annotations: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
