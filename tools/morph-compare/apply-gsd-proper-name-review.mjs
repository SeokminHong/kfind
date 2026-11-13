import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, mode, ...extra] = process.argv.slice(2);
if (!input || (mode !== undefined && mode !== '--person-names') || extra.length) {
  throw new Error('usage: node apply-gsd-proper-name-review.mjs GSD_POSITIVES.jsonl [--person-names]');
}
const personNames = mode === '--person-names';
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
const expectedChecksum = personNames
  ? '149aa2f31b2ccaf769bea4afe09ec213b080c47dd9ca37f447cd8177cf2487aa'
  : '6606de93e41ce3cd475b5469c8d6673c087877e73549218e6c64c10d369a615a';
if (checksum !== expectedChecksum) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const reviews = new Map();
const registryPath = personNames ? './gsd-person-name-review.tsv' : './gsd-proper-name-review.tsv';
const registry = readFileSync(new URL(registryPath, import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, sourceQuery, sourceLemma, sourceXpos, query, lemma, xpos, reason, form] = line.split('\t');
  if (reviews.has(id) || !query || !reason || !(lemma === query || lemma.startsWith(query + '+'))) {
    throw new Error(`invalid proper-name review: ${line}`);
  }
  reviews.set(id, { sourceQuery, sourceLemma, sourceXpos, query, lemma, xpos, form: form ?? lemma.split('+').join('') });
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const gold = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.expected !== true || row.pos !== 'noun'
      || row.query !== review.sourceQuery || row.lemma !== review.sourceLemma
      || row.xpos !== review.sourceXpos || !gold.equals(Buffer.from(review.form))) {
    throw new Error(`proper-name review no longer matches source: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, query: review.query, lemma: review.lemma, xpos: review.xpos });
});
if (reviews.size) throw new Error(`unused reviews: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
