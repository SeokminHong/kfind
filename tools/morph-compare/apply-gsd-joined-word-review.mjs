import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (!input || process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-joined-word-review.mjs GSD_POSITIVES.jsonl');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'dfe68b16383e6dca96829cd30ce9051e939029f67b16479f1254eadf8a864e6a') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const lines = original.toString('utf8').trimEnd().split('\n');
const rows = lines.map(line => JSON.parse(line));
const indexes = new Map(rows.map((row, index) => [row.id, index]));
const used = new Set();
const skipped = new Set();
const registry = readFileSync(new URL('./gsd-joined-word-review.tsv', import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [firstId, secondId, firstForm, secondForm, query, pos, lemma, xpos] = line.split('\t');
  const firstIndex = indexes.get(firstId);
  const secondIndex = indexes.get(secondId);
  const first = rows[firstIndex];
  const second = rows[secondIndex];
  const form = firstForm + secondForm;
  if (!first || !second || secondIndex !== firstIndex + 1 || used.has(firstIndex)
      || used.has(secondIndex) || first.expected !== true || second.expected !== true
      || first.text !== second.text || first.form !== firstForm || second.form !== secondForm
      || first.gold_byte_end !== second.gold_byte_start || !query || !pos || !xpos
      || !lemma || lemma.split('+')[0] !== query || lemma.split('+').join('') !== form
      || !Buffer.from(first.text).subarray(first.gold_byte_start, second.gold_byte_end)
        .equals(Buffer.from(form))) {
    throw new Error(`joined-word review no longer matches source: ${line}`);
  }
  lines[firstIndex] = JSON.stringify({
    ...first, query, pos, form, lemma, xpos, gold_byte_end: second.gold_byte_end,
  });
  used.add(firstIndex);
  used.add(secondIndex);
  skipped.add(secondIndex);
}
process.stdout.write(lines.filter((_, index) => !skipped.has(index)).join('\n') + '\n');
