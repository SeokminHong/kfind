import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, mode, ...extra] = process.argv.slice(2);
if (!input || (mode !== undefined && mode !== '--after-name-review') || extra.length) {
  throw new Error('usage: node apply-gsd-copula-span-review.mjs GSD_POSITIVES.jsonl [--after-name-review]');
}
const afterNameReview = mode === '--after-name-review';
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
const expectedChecksum = afterNameReview
  ? '132f6c5dd4efea5000e9a9b0cdaa064da9b058226e0b0eff2af4ff5e42b3d365'
  : 'c1ee2680c8faba318b3d19245933a690d27f193df17458fd2b05ac6a1780c4d0';
if (checksum !== expectedChecksum) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const registry = afterNameReview ? './gsd-copula-boundary-audit.tsv' : './gsd-copula-span-review.tsv';
const review = readFileSync(new URL(registry, import.meta.url), 'utf8');
const cases = new Map();
for (const line of review.trimEnd().split('\n').slice(1)) {
  const [id, query, form, reason] = line.split('\t');
  if (cases.has(id) || !query || !form?.startsWith(query) || form === query || !reason) {
    throw new Error(`invalid review: ${line}`);
  }
  cases.set(id, { query, form });
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const corrected = cases.get(row.id);
  if (!corrected) return line;
  const gold = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.pos !== 'noun' || row.expected !== true || row.query !== corrected.query
      || row.form !== corrected.form || !gold.equals(Buffer.from(corrected.form))
      || !row.lemma.startsWith(row.query + '+이+')) {
    throw new Error(`review no longer matches original annotation: ${row.id}`);
  }
  cases.delete(row.id);
  row.gold_byte_end = row.gold_byte_start + Buffer.byteLength(row.query);
  return JSON.stringify(row);
});
if (cases.size) throw new Error(`unused reviews: ${[...cases.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
