import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (!input || process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-nominal-suffix-review.mjs GSD_POSITIVES.jsonl');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '3161e0bb04c405518322600dce3188f0fd94f50f85b5ca05f2975360379b3919') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

// The registry records reviewed morphology rather than treating every XSN tag as reliable.
const reviews = new Map();
const registry = readFileSync(new URL('./gsd-nominal-suffix-review.tsv', import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, query, form, suffix, reason] = line.split('\t');
  if (reviews.has(id) || !query || !suffix || !form?.startsWith(query + suffix) || !reason) {
    throw new Error(`invalid review: ${line}`);
  }
  reviews.set(id, { query, form, suffix });
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const review = reviews.get(row.id);
  if (!review) return line;
  const gold = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.pos !== 'noun' || row.expected !== true || row.query !== review.query
      || row.form !== review.form || !gold.equals(Buffer.from(review.form))
      || !/^NNG\+XSN(?:\+J[A-Z]+)*$/.test(row.xpos)
      || row.lemma.split('+')[1] !== review.suffix) {
    throw new Error(`review no longer matches original annotation: ${row.id}`);
  }
  reviews.delete(row.id);
  row.gold_byte_end = row.gold_byte_start + Buffer.byteLength(row.query);
  return JSON.stringify(row);
});
if (reviews.size) throw new Error(`unused reviews: ${[...reviews.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
