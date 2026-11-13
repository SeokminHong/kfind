import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const reviewsByMode = new Map([
  [undefined, ['6606de93e41ce3cd475b5469c8d6673c087877e73549218e6c64c10d369a615a', './gsd-proper-name-review.tsv']],
  ['--person-names', ['149aa2f31b2ccaf769bea4afe09ec213b080c47dd9ca37f447cd8177cf2487aa', './gsd-person-name-review.tsv']],
  ['--entity-names', ['b7a0726517a62c72866525ff882bab0c9dda293a705e9ae886e15bd334e2f617', './gsd-entity-name-review.tsv']],
  ['--lexical-units', ['e9e89fc3fc5e0b385963e6eb346c2fb2db381660d1ccda35f51981afc5504eac', './gsd-lexical-unit-pos-review.tsv']],
  ['--numeral-compounds', ['2a936b51b352b1ecd5ad2388e60303d89b398fb5c5eb42c1dc4b102aad620188', './gsd-numeral-compound-review.tsv']],
]);
const [input, mode, ...extra] = process.argv.slice(2);
const reviewSource = reviewsByMode.get(mode);
if (!input || !reviewSource || extra.length) {
  throw new Error('usage: node apply-gsd-proper-name-review.mjs GSD_POSITIVES.jsonl [--person-names|--entity-names|--lexical-units|--numeral-compounds]');
}
const [expectedChecksum, registryPath] = reviewSource;
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== expectedChecksum) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const reviews = new Map();
const registry = readFileSync(new URL(registryPath, import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, sourceQuery, sourceLemma, sourceXpos, query, lemma, xpos, reason, form, sourcePos = 'noun'] = line.split('\t');
  if (reviews.has(id) || !query || !reason || !(lemma === query || lemma.startsWith(query + '+'))) {
    throw new Error(`invalid proper-name review: ${line}`);
  }
  reviews.set(id, { sourceQuery, sourceLemma, sourceXpos, sourcePos, query, lemma, xpos, form: form ?? lemma.split('+').join('') });
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const gold = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.expected !== true || row.pos !== review.sourcePos
      || row.query !== review.sourceQuery || row.lemma !== review.sourceLemma
      || row.xpos !== review.sourceXpos || !gold.equals(Buffer.from(review.form))) {
    throw new Error(`proper-name review no longer matches source: ${row.id}`);
  }
  reviews.delete(row.id);
  return JSON.stringify({ ...row, pos: 'noun', query: review.query, lemma: review.lemma, xpos: review.xpos });
});
if (reviews.size) throw new Error(`unused reviews: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
