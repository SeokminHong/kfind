import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (!input || process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-proper-name-review.mjs GSD_POSITIVES.jsonl');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '6606de93e41ce3cd475b5469c8d6673c087877e73549218e6c64c10d369a615a') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const reviews = new Map();
const registry = readFileSync(new URL('./gsd-proper-name-review.tsv', import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, sourceQuery, sourceLemma, sourceXpos, query, lemma, xpos, reason] = line.split('\t');
  if (reviews.has(id) || !query || !reason || !lemma.startsWith(query + '+')) {
    throw new Error(`invalid proper-name review: ${line}`);
  }
  reviews.set(id, { sourceQuery, sourceLemma, sourceXpos, query, lemma, xpos });
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const gold = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.expected !== true || row.pos !== 'noun'
      || row.query !== review.sourceQuery || row.lemma !== review.sourceLemma
      || row.xpos !== review.sourceXpos || !gold.equals(Buffer.from(review.lemma.split('+').join('')))) {
    throw new Error(`proper-name review no longer matches source: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, query: review.query, lemma: review.lemma, xpos: review.xpos });
});
if (reviews.size) throw new Error(`unused reviews: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
