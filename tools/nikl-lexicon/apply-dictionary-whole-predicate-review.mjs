import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-whole-predicate-review.mjs RAW-CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '747e7e60df2c779d0cd6b79c27c088e283d5e81bf613988832c9a1b8233bf884') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

// These rows annotate the entire dictionary predicate, including 잘.
const reviews = new Map([
  ['겠:8', ['하다', '잘하겠다', '잘하다']],
  ['고:21', ['되다', '잘됐고', '잘되다']],
  ['대:5', ['생기다', '잘생겼대', '잘생기다']],
  ['더니만:9', ['하다', '잘하더니만', '잘하다']],
  ['더라면서:9', ['되다', '잘되겠더라면서', '잘되다']],
  ['더이까:2', ['하다', '잘하시더이까', '잘하다']],
  ['던걸:7', ['하다', '잘하던걸', '잘하다']],
  ['려나:7', ['하다', '잘하려나', '잘하다']],
  ['려면:10', ['되다', '잘되려면', '잘되다']],
  ['셔요:3', ['하다', '잘하셔요', '잘하다']],
  ['소:4', ['되다', '잘되었소', '잘되다']],
  ['습디다:7', ['하다', '잘하겠습디다', '잘하다']],
]);

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;

  const [query, surface, correctedQuery] = review;
  if (row.query !== query || row.surface !== surface) {
    throw new Error(`unexpected predicate annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, query: correctedQuery });
});

if (reviews.size) throw new Error('missing dictionary annotations');
process.stdout.write(output.join('\n') + '\n');
