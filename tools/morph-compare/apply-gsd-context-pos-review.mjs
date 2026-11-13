import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (!input || process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-context-pos-review.mjs GSD_POSITIVES.jsonl');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '16efe1b8b9eb19be9bf81bfa7a505362fe77fa6337653db88574ccf45d7abcf1') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const reviews = new Map();
const registry = readFileSync(new URL('./gsd-context-pos-review.tsv', import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, sourceQuery, sourcePos, form, query, pos, lemma, xpos, reason] = line.split('\t');
  if (reviews.has(id) || !reason || !sourceQuery || !sourcePos || !form
      || !query || !pos || !lemma || !xpos || sourcePos === pos) {
    throw new Error(`invalid contextual POS review: ${line}`);
  }
  reviews.set(id, { sourceQuery, sourcePos, form, query, pos, lemma, xpos });
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  if (row.expected !== true || row.query !== review.sourceQuery || row.pos !== review.sourcePos
      || row.form !== review.form
      || !Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end)
        .equals(Buffer.from(review.form))) {
    throw new Error(`contextual POS review no longer matches source: ${row.id}`);
  }
  reviews.delete(row.id);
  const { query, pos, lemma, xpos } = review;
  return JSON.stringify({ ...row, query, pos, lemma, xpos });
});
if (reviews.size) throw new Error(`unused reviews: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
