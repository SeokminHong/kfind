import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const reviews = new Map([
  [undefined, {
    checksum: 'c1ee2680c8faba318b3d19245933a690d27f193df17458fd2b05ac6a1780c4d0',
    registry: './gsd-copula-span-review.tsv',
    pos: 'noun',
  }],
  ['--after-name-review', {
    checksum: '132f6c5dd4efea5000e9a9b0cdaa064da9b058226e0b0eff2af4ff5e42b3d365',
    registry: './gsd-copula-boundary-audit.tsv',
    pos: 'noun',
  }],
  ['--pronouns', {
    checksum: '7441ca03d27db97675c7e713ebaa57bd478e29aa5da404a70eabdbdc4fe566e3',
    registry: './gsd-pronoun-copula-span-review.tsv',
    pos: 'pronoun',
  }],
  ['--numerals', {
    checksum: '2599698e1404840a25bc5119b771dffcb7dc7a8a1d5d47c66d302b73e168c49d',
    registry: './gsd-numeral-copula-span-review.tsv',
    pos: 'numeral',
  }],
  ['--after-colloquial-pos', {
    checksum: '5e5b16769d5489b5589e1aceb5980ab726e6cdd3aab55fad4cb628e1da844f62',
    registry: './gsd-colloquial-noun-copula-span-review.tsv',
    pos: 'noun',
  }],
]);
const [input, mode, ...extra] = process.argv.slice(2);
const selectedReview = reviews.get(mode);
if (!input || !selectedReview || extra.length) {
  throw new Error('usage: node apply-gsd-copula-span-review.mjs GSD_POSITIVES.jsonl [--after-name-review|--pronouns|--numerals|--after-colloquial-pos]');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== selectedReview.checksum) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}
const review = readFileSync(new URL(selectedReview.registry, import.meta.url), 'utf8');
const cases = new Map();
for (const line of review.trimEnd().split('\n').slice(1)) {
  const [id, query, form, reason, goldForm = query, lemma, xpos, ...rest] = line.split('\t');
  if (cases.has(id) || !query || !form?.startsWith(goldForm) || !goldForm.startsWith(query)
      || form === goldForm || !reason || Boolean(lemma) !== Boolean(xpos) || rest.length
      || (lemma && (!lemma.startsWith(query + '+') || !xpos.split('+').includes('VCP')))) {
    throw new Error(`invalid review: ${line}`);
  }
  cases.set(id, { query, form, goldForm, lemma, xpos });
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const corrected = cases.get(row.id);
  if (!corrected) return line;
  const gold = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.pos !== selectedReview.pos || row.expected !== true || row.query !== corrected.query
      || row.form !== corrected.form || !gold.equals(Buffer.from(corrected.form))
      || (corrected.lemma ? !row.xpos.split('+').includes('VCP')
        : !row.lemma.startsWith(row.query + '+이+'))) {
    throw new Error(`review no longer matches original annotation: ${row.id}`);
  }
  cases.delete(row.id);
  row.gold_byte_end = row.gold_byte_start + Buffer.byteLength(corrected.goldForm);
  if (corrected.lemma) {
    row.lemma = corrected.lemma;
    row.xpos = corrected.xpos;
  }
  return JSON.stringify(row);
});
if (cases.size) throw new Error(`unused reviews: ${[...cases.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
