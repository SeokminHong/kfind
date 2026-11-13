import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...spanCorrections] = process.argv.slice(2);
const explicitCopula = spanCorrections.includes('--explicit-copula-noun-spans');
const fusedCopula = spanCorrections.includes('--fused-copula-noun-spans');
const honorificNoun = spanCorrections.includes('--honorific-noun-spans');
if (!input || spanCorrections.length !== Number(explicitCopula) + Number(fusedCopula) + Number(honorificNoun)) {
  throw new Error('usage: node apply-gsd-positive-corrections.mjs GSD_POSITIVES.jsonl [--explicit-copula-noun-spans] [--fused-copula-noun-spans] [--honorific-noun-spans]');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'a6955363dbf837f16b5ec4ef6465eabf8e6611df43c591a21588aa638efde129') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

function tsv(path, width) {
  return readFileSync(new URL(path, import.meta.url), 'utf8')
    .split(/\r?\n/).slice(1).filter(line => line.length).map(line => {
      const fields = line.split('\t');
      if (fields.length !== width) throw new Error(`invalid TSV row: ${line}`);
      return fields;
    });
}

const corrections = new Map();
for (const [id, fixture, reason] of tsv('./gsd-positive-input-corrections.tsv', 3)) {
  if (corrections.has(id) || !reason.trim()) throw new Error(`invalid correction: ${id}`);
  corrections.set(id, fixture);
}

const fixtures = new Map();
const fixtureIds = new Set(corrections.values());
for (const row of tsv('../../data/fixtures/ud_morphology_cases.tsv', 7)) {
  if (!fixtureIds.has(row[0])) continue;
  if (fixtures.has(row[0])) throw new Error(`duplicate correction fixture: ${row[0]}`);
  fixtures.set(row[0], row);
}

const particleSurfaces = explicitCopula
  ? new Set(tsv('../../data/lexicon/particles.tsv', 5)
    .flatMap(([, , , variants]) => variants.split('|')))
  : new Set();

const fusedCopulaEndings = new Map([
  ['인', 'ㄴ'], ['입니다', 'ㅂ니다'], ['다', '다'],
]);

function hasAlignedHonorificSuffix(row) {
  if (row.pos !== 'noun' || row.expected !== true || !/^[가-힣]{2,}$/.test(row.query)
      || !/^(NNG|NNP)\+XSN(?:\+XSN)?(?:\+J[A-Z]+)*$/.test(row.xpos)) {
    return false;
  }
  const lemmas = row.lemma.split('+');
  const positions = row.xpos.split('+');
  return lemmas[0] === row.query && lemmas[1] === '님'
    && lemmas.length === positions.length
    && (positions[2] !== 'XSN' || lemmas[2] === '들')
    && lemmas.join('') === row.form
    && Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end)
      .toString('utf8') === row.form;
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const fixture = corrections.get(row.id);
  if (!fixture) {
    if (honorificNoun && hasAlignedHonorificSuffix(row)) {
      row.gold_byte_end = row.gold_byte_start + Buffer.byteLength(row.query);
      return JSON.stringify(row);
    }
    if ((!explicitCopula && !fusedCopula) || row.pos !== 'noun' || row.expected !== true
        || !/^(NNG|NNB|NNP)\+VCP(?:\+EP)*\+(EC|EF|ETM)$/.test(row.xpos)
        || !row.lemma.startsWith(row.query + '+이+')
        || !row.form.startsWith(row.query)
        || Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end)
          .toString('utf8') !== row.form) {
      return line;
    }
    const tail = row.form.slice(row.query.length);
    const explicit = explicitCopula && tail.startsWith('이') && !particleSurfaces.has(tail);
    const ending = fusedCopulaEndings.get(tail);
    const last = row.query.codePointAt(row.query.length - 1);
    const openSyllable = last >= 0xac00 && last <= 0xd7a3 && (last - 0xac00) % 28 === 0;
    const fused = fusedCopula && ending && (tail !== '다' || openSyllable)
      && row.lemma === row.query + '+이+' + ending;
    if (!explicit && !fused) return line;
    row.gold_byte_end = row.gold_byte_start + Buffer.byteLength(row.query);
    return JSON.stringify(row);
  }
  const [, query, pos, expectation, start, end, text] = fixtures.get(fixture) ?? [];
  if (query !== row.query || pos !== row.pos || expectation !== 'smart-exact'
      || !fixture.startsWith(`corpus.ud-gsd.test.${row.id.split(':')[1]}.`)
      || row.expected !== true) {
    throw new Error(`correction identity differs: ${row.id} / ${fixture}`);
  }
  const span = Buffer.from(text).subarray(Number(start), Number(end));
  if (!span.length || span.length !== Number(end) - Number(start)
      || !span.equals(Buffer.from(span.toString('utf8')))) {
    throw new Error(`invalid correction span: ${fixture}`);
  }
  corrections.delete(row.id);
  return JSON.stringify({
    id: row.id, query, pos, text, expected: true,
    gold_byte_start: Number(start), gold_byte_end: Number(end),
  });
});
if (corrections.size) throw new Error(`unused corrections: ${[...corrections.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
