import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, mode, ...extra] = process.argv.slice(2);
if (!input || (mode !== undefined && !['--pronouns', '--whole-nouns'].includes(mode)) || extra.length) {
  throw new Error('usage: node apply-gsd-context-pos-review.mjs GSD_POSITIVES.jsonl [--pronouns | --whole-nouns]');
}
const pronouns = mode === '--pronouns';
const wholeNouns = mode === '--whole-nouns';
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
const expectedChecksum = wholeNouns
  ? '0824c82573c7ccf16fbd22758d08fe3c8155b2c871b8395b800f82b2cd7c204f'
  : pronouns
  ? 'f32a177315f52acff2c691341d527b9191590050b3c1dff28f57ca5fdf17e1ed'
  : '16efe1b8b9eb19be9bf81bfa7a505362fe77fa6337653db88574ccf45d7abcf1';
if (checksum !== expectedChecksum) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const reviews = new Map();
const registryPath = wholeNouns ? './gsd-whole-noun-context-review.tsv'
  : pronouns ? './gsd-pronoun-context-review.tsv' : './gsd-context-pos-review.tsv';
const registry = readFileSync(new URL(registryPath, import.meta.url), 'utf8');
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
