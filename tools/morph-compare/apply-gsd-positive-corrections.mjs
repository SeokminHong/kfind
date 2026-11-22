import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...options] = process.argv.slice(2);
const explicitCopula = options.includes('--explicit-copula-noun-spans');
const fusedCopula = options.includes('--fused-copula-noun-spans');
const honorificNoun = options.includes('--honorific-noun-spans');
const compoundNoun = options.includes('--compound-noun-spans');
const reviewedSpelling = options.includes('--reviewed-spelling');
if (!input || options.length !== Number(explicitCopula) + Number(fusedCopula)
    + Number(honorificNoun) + Number(compoundNoun) + Number(reviewedSpelling)) {
  throw new Error('usage: node apply-gsd-positive-corrections.mjs GSD_POSITIVES.jsonl [--explicit-copula-noun-spans] [--fused-copula-noun-spans] [--honorific-noun-spans] [--compound-noun-spans] [--reviewed-spelling]');
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
for (const [id, fixture, reason, query, pos, start, end, text] of tsv('./gsd-positive-input-corrections.tsv', 8)) {
  if (corrections.has(id) || !reason.trim()) throw new Error(`invalid correction: ${id}`);
  corrections.set(id, { fixture, query, pos, start: Number(start), end: Number(end), text });
}

// The fixed GSD input labels the typo in test-s501 as an adverb.
const spellingSource = '선생님들 너무 친절하시고 가격도 괜찮고 무척 맘에 듬!';
const spellingTarget = '선생님들 너무 친절하시고 가격도 괜찮고 무척 맘에 듦!';
function correctReviewedSpelling(row) {
  const source = Buffer.from(row.text);
  const corrected = Buffer.from(spellingTarget);
  const byteStart = 70;
  const byteEnd = 73;
  if (source.length !== corrected.length
      || !source.subarray(0, byteStart).equals(corrected.subarray(0, byteStart))
      || !source.subarray(byteEnd).equals(corrected.subarray(byteEnd))
      || source.subarray(byteStart, byteEnd).toString('utf8') !== '듬'
      || corrected.subarray(byteStart, byteEnd).toString('utf8') !== '듦') {
    throw new Error(`invalid spelling correction: ${row.id}`);
  }
  row.text = spellingTarget;
  if (row.gold_byte_end <= byteStart || row.gold_byte_start >= byteEnd) return;
  if (row.gold_byte_start !== byteStart || row.gold_byte_end !== byteEnd
      || row.query !== '듬' || row.pos !== 'adverb' || row.form !== '듬'
      || row.lemma !== '듬' || row.xpos !== 'MAG') {
    throw new Error(`unexpected annotation at corrected spelling: ${row.id}`);
  }
  Object.assign(row, { query: '들다', pos: 'verb', form: '듦', lemma: '들+ㅁ', xpos: 'VV+ETN' });
}

const particleSurfaces = explicitCopula
  ? new Set(tsv('../../data/lexicon/particles.tsv', 5)
    .flatMap(([, , , variants]) => variants.split('|')))
  : new Set();

const fusedCopulaEndings = new Map([
  ['인', 'ㄴ'], ['입니다', 'ㅂ니다'], ['다', '다'],
]);

function alignedNounParts(row) {
  if (row.pos !== 'noun' || row.expected !== true || !/^[가-힣]{2,}$/.test(row.query)) {
    return null;
  }
  const lemmas = row.lemma.split('+');
  const positions = row.xpos.split('+');
  if (lemmas[0] !== row.query || lemmas.length !== positions.length
      || lemmas.join('') !== row.form
      || Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end)
        .toString('utf8') !== row.form) {
    return null;
  }
  return { lemmas, positions };
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (reviewedSpelling && row.text === spellingSource) {
    correctReviewedSpelling(row);
    line = JSON.stringify(row);
  }
  const correction = corrections.get(row.id);
  if (!correction) {
    const honorific = honorificNoun && /^(NNG|NNP)\+XSN(?:\+XSN)?(?:\+J[A-Z]+)*$/.test(row.xpos);
    const compound = compoundNoun && /^NNG(?:\+NNG)+(?:\+J[A-Z]+)*$/.test(row.xpos);
    const nounParts = (honorific || compound) && alignedNounParts(row);
    if (nounParts && (compound || (nounParts.lemmas[1] === '님'
        && (nounParts.positions[2] !== 'XSN' || nounParts.lemmas[2] === '들')))) {
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
  const { fixture, query, pos, start, end, text } = correction;
  if (query !== row.query || pos !== row.pos
      || !fixture.startsWith(`corpus.ud-gsd.test.${row.id.split(':')[1]}.`)
      || row.expected !== true) {
    throw new Error(`correction identity differs: ${row.id} / ${fixture}`);
  }
  const span = Buffer.from(text).subarray(start, end);
  if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 0
      || !span.length || span.length !== end - start
      || !span.equals(Buffer.from(span.toString('utf8')))) {
    throw new Error(`invalid correction span: ${fixture}`);
  }
  corrections.delete(row.id);
  return JSON.stringify({
    id: row.id, query, pos, text, expected: true,
    gold_byte_start: start, gold_byte_end: end,
  });
});
if (corrections.size) throw new Error(`unused corrections: ${[...corrections.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
