import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...options] = process.argv.slice(2);
const finalProofreading = options.includes('--final-proofreading');
const caseTopic = options.includes('--case-topic');
const contract = options.includes('--review-contract');
const shared = options.includes('--align-shared-sentences');
const restored = options.includes('--restore-source-passages');
const proofread = options.includes('--proofread');
const independent = options.includes('--independent-proofreading');
const contextAnnotations = options.includes('--context-annotations');
const nominalModifiers = options.includes('--nominal-modifiers');
const derivedNominals = options.includes('--derived-nominals');
const scopeNominals = options.includes('--scope-nominals');
const jiNominals = options.includes('--ji-nominals');
const motSpacing = options.includes('--mot-spacing');
const ijenContext = options.includes('--ijen-context');
const quantifierAnnotations = options.includes('--quantifier-annotations');
const lexicalWhole = options.includes('--lexical-whole');
const contextLexemes = options.includes('--context-lexemes');
const receiveDerivation = options.includes('--receive-derivation');
const missegmentedLexemes = options.includes('--missegmented-lexemes');
const properBoundaries = options.includes('--proper-boundaries');
const nominalDerivedBoundaries = options.includes('--nominal-derived-boundaries');
const compoundBoundaries = options.includes('--compound-boundaries');
const predicateReanalysis = options.includes('--predicate-reanalysis');
const auxiliaryPos = options.includes('--auxiliary-pos');
const derivedPredicatePos = options.includes('--derived-predicate-pos');
const nominalHada = options.includes('--nominal-hada');
const derivedSegmentation = options.includes('--derived-segmentation');
const colloquialLemma = options.includes('--colloquial-lemma');
const odaSpacing = options.includes('--oda-spacing');
const linkedMainVerbSpacing = options.includes('--linked-main-verb-spacing');
const lexicalPredicates = options.includes('--lexical-predicates');
const predicateContinuations = options.includes('--predicate-continuations');
const colloquialPos = options.includes('--colloquial-pos');
const residualContext = options.includes('--residual-context');
const proofreadingJoins = options.includes('--proofreading-joins');
const spatialSpacing = options.includes('--spatial-spacing');
const nominalSpelling = options.includes('--nominal-spelling');
const ellipsisAndEntity = options.includes('--ellipsis-and-entity');
const trialAuxiliarySpacing = options.includes('--trial-auxiliary-spacing');
const saintDeterminer = options.includes('--saint-determiner');
const derivedWholeNouns = options.includes('--derived-whole-nouns');
const helpCompany = options.includes('--help-company');
const alignedNominalSpans = options.includes('--aligned-nominal-spans');
const mayorTitle = options.includes('--mayor-title');
const anonymousMo = options.includes('--anonymous-mo');
if (!input || (restored && !shared) || (proofread && !restored)
    || (finalProofreading || caseTopic || independent || contextAnnotations || nominalModifiers || derivedNominals || scopeNominals || jiNominals || motSpacing || ijenContext || quantifierAnnotations || lexicalWhole || contextLexemes || receiveDerivation || missegmentedLexemes || properBoundaries || nominalDerivedBoundaries || compoundBoundaries || predicateReanalysis || auxiliaryPos || derivedPredicatePos || nominalHada || derivedSegmentation || colloquialLemma || odaSpacing || linkedMainVerbSpacing || lexicalPredicates || predicateContinuations || colloquialPos || residualContext || proofreadingJoins || spatialSpacing || anonymousMo || mayorTitle || alignedNominalSpans || helpCompany || derivedWholeNouns || nominalSpelling || ellipsisAndEntity || trialAuxiliarySpacing || saintDeterminer ? options.length !== 1
      : options.length !== Number(contract) + Number(shared) + Number(restored) + Number(proofread))) {
  throw new Error('usage: node apply-gsd-sentence-edits.mjs GSD_POSITIVES.jsonl ([--review-contract] [--align-shared-sentences [--restore-source-passages [--proofread]]] | --independent-proofreading | --context-annotations | --nominal-modifiers | --derived-nominals | --scope-nominals | --ji-nominals | --mot-spacing | --ijen-context | --quantifier-annotations | --lexical-whole | --context-lexemes | --receive-derivation | --missegmented-lexemes | --proper-boundaries | --nominal-derived-boundaries | --compound-boundaries | --predicate-reanalysis | --auxiliary-pos | --derived-predicate-pos | --nominal-hada | --derived-segmentation | --colloquial-lemma | --oda-spacing | --linked-main-verb-spacing | --lexical-predicates | --predicate-continuations | --colloquial-pos | --residual-context | --proofreading-joins | --spatial-spacing | --anonymous-mo | --mayor-title | --aligned-nominal-spans | --help-company | --derived-whole-nouns | --nominal-spelling | --ellipsis-and-entity | --trial-auxiliary-spacing | --saint-determiner | --case-topic | --final-proofreading)');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
const previousInput = 'd34735e79fad865a62685522859f2664e56d9f37330a1a11d17614aab82cda1e';
const currentFixtureInput = '8f3aa236f20a0913e78e04139539b636835a538ab5d552891856a73a5e77e5da';
const contextPosReviewInput = '120ad5000b84b3d61fef05214b6d703e0f573eb55330f0741cf455583806dce9';
if (finalProofreading ? checksum !== '0ef48568e648372e3b50a0cbd273f96afeec753a82fdae6cee9dcdc6279d67a2'
  : caseTopic ? checksum !== 'f2be67e982e0b19ca1b93894bf1cd2699a162d9281304b04c8bf7570870927d0'
  : saintDeterminer ? checksum !== '4f6f14ec976d2696c5a9b0209ee3b01ec348acbcddba4f3da1649d56887aadb5'
  : trialAuxiliarySpacing ? checksum !== '4ee18bc38e91b230ddb0e3b3eefae2407ccba455b4c247f8fb61e45afa243744'
  : ellipsisAndEntity ? checksum !== '62e5d06e573f4bdb8340485342b9696d205734c410db2859d14d5fbec9606d14'
  : nominalSpelling ? checksum !== 'ff6c0a7f242b008e658866f7fc7c9c2cf23d6eaa0439868624a89ece149f4917'
  : derivedWholeNouns ? checksum !== '021369ff1f3751b18ea575d6d8cff7f85afdefab2b80c6d9ddd86d16f239f37b'
  : helpCompany ? checksum !== 'e9e472a41b3a3224059661de54607a80b97f154b7314e243c182d54108468197'
  : alignedNominalSpans ? checksum !== 'bc76e33f410286dcd215b4d35036b2e5b5fc035a3bc0cef2021150a306eb3b1a'
  : mayorTitle ? checksum !== 'c39fa1be092b9fc5ff7f2fa1455f506c50bf81993d6d1997af3099376ffaf933'
  : anonymousMo ? checksum !== '48830301af8d724e76ba8c10ccc52b0ea2a2905203da3f6e1e65caaa60934b53'
  : spatialSpacing ? checksum !== '98b314c1b09ac693954ebb8c7dcd6b0896ce788a3d93192ecaf51437af8c6eb5'
  : proofreadingJoins ? checksum !== 'f95cb1e13125a02593a1d02ff3c79a51338cbf6fd7bbb9cc7c6406b8a93f1bc1'
  : residualContext ? checksum !== '9f0a8e30eb76db721ca3ccb0887d7c5ae60a8a169e2e54209c45edc05428bcd8'
  : colloquialPos ? checksum !== '1c1dd7df1a4b9539ae1e9da68e9c230e941a7ecb87346a6254420f3b5d23a9a2'
  : predicateContinuations ? checksum !== '09aa09efdf801f00a282f2efae8f9ddde820a59852e5fb9929f2de2781515483'
  : lexicalPredicates ? checksum !== 'c8e1a2139a5e14851ac44bd37de5625bb95fe8a7dbfa6901d037fd1ef30c817b'
  : linkedMainVerbSpacing ? checksum !== 'df1bc57ccfe282af9adbcd50f26a2848b31e9268e90d70ad38229215bda6a6d0'
  : odaSpacing ? checksum !== 'b0fbd35526ef9061c4570d3c278e71566c057dbce23c47f7603b5a482303e01d'
  : colloquialLemma ? checksum !== 'c20589599d2a1cb5ebb4db6984b71958008338cda90b3a369f72a5bdc34c5ccd'
  : derivedSegmentation ? checksum !== '6eb493770093473105147fac8f9f1d3dc1442bdb26df8db8d50a4984c2639d2c'
  : nominalHada ? checksum !== '7dbe6129e7380a66cbb4b146092587d0d689f6921b9b7fc42c16f2ddf8fe2475'
  : derivedPredicatePos ? checksum !== 'eff0db6a94e16d956045f51f053d9b8555a7486e581ee02e4cb86d74019949f6'
  : auxiliaryPos ? checksum !== '77991d709a3fdfe0a3487c44cb2a2ad109d004c27d8df333e45eff221bd96ecd'
  : predicateReanalysis ? checksum !== '4169d0ddf9fb90ede23c7e7c1f75aeb3af30ca5bf807d036dd1a9f027d65d0ac'
  : compoundBoundaries ? checksum !== '9d23492fcaeb7aa26594f2905475c808b55f16369f59ab97ee102898afcc9ef5'
  : nominalDerivedBoundaries ? checksum !== '4864110461bd2282bc02b4bb8d849f85567dd36822691b0aa337b572c742dc53'
  : properBoundaries ? checksum !== '4fd8f6b94289f8012e9d0440cddcc2b37243bb1c841647dd4b3ff6f167b6391b'
  : missegmentedLexemes ? checksum !== 'b5188fbc43f2dea068610f3fcf7e2860b6991971d5e47b8f30d739695a0b6701'
  : receiveDerivation ? checksum !== '4671bd344b3c6fb23e134a08abae0eb0213cb24cb99eeb09b55e7a4af1bd3b45'
  : contextLexemes ? checksum !== '7bf39f19be1928899912ee520a01aa891ba0a588d840b4ad29a90f5108bff1d1'
  : lexicalWhole ? checksum !== 'bb994e0e82346fdba18c87d732cbef4797d5147ca9f097c3361b1ff70f67cff8'
  : quantifierAnnotations ? checksum !== '6a26f0297752a2b344e773de7ea7b67196f0c5608d9af173326c5092c7ac41b1'
  : ijenContext ? checksum !== '063db16df618a5494a4cd134ce516efe48ae59e8298ea0049cf4bed1cad5ecd7'
  : motSpacing ? checksum !== '1552fa35dfc7e06dd49e8e79208755f84fefb2e4a7e4673dba8b256194c3c1e6'
  : jiNominals ? checksum !== 'a990e0f0fa2eb5488ff817b3f177fa38c7561c06cae3a9df92437cad63edaeef'
  : scopeNominals ? checksum !== 'e09e933e865128f1f2bfd937a5d192e26d5250967800a82e4e786108c5554ff3'
  : derivedNominals ? checksum !== '957dad65dbc029053f56d531d8969638c4a52cb4270fa8b898c8380f722eef9d'
  : nominalModifiers ? checksum !== '1d8380f3c6c6f892fab50f510b0013ad8db55ea1105901ac24f15508510c4bb2'
  : contextAnnotations ? checksum !== '457c654f940283ce6010293d8497b9cd70faf39d254560b551fdb0eeb2f691ae'
  : independent ? checksum !== contextPosReviewInput
  : checksum !== previousInput && !(shared && checksum === currentFixtureInput)) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

function tsv(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8')
    .split(/\r?\n/).slice(1).filter(Boolean).map(line => line.split('\t'));
}

const editsBySentence = new Map();
const sentenceEdits = caseTopic || saintDeterminer || derivedWholeNouns || alignedNominalSpans || mayorTitle || anonymousMo || proofreadingJoins || residualContext || colloquialPos || predicateContinuations || lexicalPredicates || derivedSegmentation || nominalHada || derivedNominals || scopeNominals || jiNominals || ijenContext || quantifierAnnotations || lexicalWhole || receiveDerivation || properBoundaries || nominalDerivedBoundaries || compoundBoundaries ? [] : tsv(finalProofreading ? './gsd-final-proofreading-edits.tsv'
  : trialAuxiliarySpacing ? './gsd-trial-auxiliary-spacing-edits.tsv'
  : linkedMainVerbSpacing ? './gsd-linked-main-verb-spacing-edits.tsv'
  : nominalSpelling ? './gsd-nominal-spelling-edits.tsv'
  : helpCompany ? './gsd-help-company-edits.tsv'
  : spatialSpacing ? './gsd-spatial-spacing-edits.tsv'
  : odaSpacing ? './gsd-oda-spacing-edits.tsv'
  : colloquialLemma ? './gsd-colloquial-lemma-edits.tsv'
  : derivedPredicatePos ? './gsd-derived-predicate-pos-edits.tsv'
  : auxiliaryPos ? './gsd-auxiliary-pos-edits.tsv'
  : predicateReanalysis ? './gsd-predicate-reanalysis-edits.tsv'
  : missegmentedLexemes ? './gsd-missegmented-lexemes-edits.tsv'
  : motSpacing ? './gsd-mot-spacing-edits.tsv'
  : nominalModifiers ? './gsd-nominal-modifier-edits.tsv'
  : contextAnnotations ? './gsd-context-annotation-edits.tsv'
  : independent ? './gsd-independent-proofreading-edits.tsv'
  : contextLexemes ? './gsd-context-lexemes-edits.tsv' : './gsd-sentence-edits.tsv');
if (shared) sentenceEdits.push(...tsv('./gsd-shared-sentence-edits.tsv'));
if (restored) sentenceEdits.push(...tsv('./gsd-source-restoration-edits.tsv'));
if (proofread) sentenceEdits.push(...tsv('./gsd-proofreading-edits.tsv'));
for (const [id, source, replacement, reason] of sentenceEdits) {
  if (!source || source === replacement || !reason) throw new Error(`invalid edit: ${id}`);
  const edits = editsBySentence.get(id) ?? [];
  edits.push({ source, replacement });
  editsBySentence.set(id, edits);
}

const annotations = new Map();
const annotationEdits = tsv(finalProofreading ? './gsd-final-proofreading-annotations.tsv'
  : caseTopic ? './gsd-case-topic-annotations.tsv'
  : saintDeterminer ? './gsd-saint-determiner-annotations.tsv'
  : trialAuxiliarySpacing ? './gsd-trial-auxiliary-spacing-annotations.tsv'
  : ellipsisAndEntity ? './gsd-ellipsis-and-entity-annotations.tsv'
  : nominalSpelling ? './gsd-nominal-spelling-annotations.tsv'
  : derivedWholeNouns ? './gsd-derived-whole-nouns-annotations.tsv'
  : helpCompany ? './gsd-help-company-annotations.tsv'
  : alignedNominalSpans ? './gsd-aligned-nominal-spans-annotations.tsv'
  : mayorTitle ? './gsd-mayor-title-annotations.tsv'
  : anonymousMo ? './gsd-anonymous-mo-annotations.tsv'
  : spatialSpacing ? './gsd-spatial-spacing-annotations.tsv'
  : proofreadingJoins ? './gsd-proofreading-join-annotations.tsv'
  : residualContext ? './gsd-residual-context-annotations.tsv'
  : colloquialPos ? './gsd-colloquial-pos-annotations.tsv'
  : predicateContinuations ? './gsd-predicate-continuation-annotations.tsv'
  : lexicalPredicates ? './gsd-lexical-predicate-annotations.tsv'
  : linkedMainVerbSpacing ? './gsd-linked-main-verb-spacing-annotations.tsv'
  : odaSpacing ? './gsd-oda-spacing-annotations.tsv'
  : colloquialLemma ? './gsd-colloquial-lemma-annotations.tsv'
  : derivedSegmentation ? './gsd-derived-segmentation-annotations.tsv'
  : nominalHada ? './gsd-nominal-hada-annotations.tsv'
  : derivedPredicatePos ? './gsd-derived-predicate-pos-annotations.tsv'
  : auxiliaryPos ? './gsd-auxiliary-pos-annotations.tsv'
  : predicateReanalysis ? './gsd-predicate-reanalysis-annotations.tsv'
  : compoundBoundaries ? './gsd-compound-boundaries-annotations.tsv'
  : nominalDerivedBoundaries ? './gsd-nominal-derived-boundaries-annotations.tsv'
  : properBoundaries ? './gsd-proper-boundaries-annotations.tsv'
  : missegmentedLexemes ? './gsd-missegmented-lexemes-annotations.tsv'
  : receiveDerivation ? './gsd-receive-derivation-annotations.tsv'
  : contextLexemes ? './gsd-context-lexemes-annotations.tsv'
  : lexicalWhole ? './gsd-lexical-whole-annotations.tsv'
  : quantifierAnnotations ? './gsd-quantifier-annotations.tsv'
  : ijenContext ? './gsd-ijen-context-annotations.tsv'
  : motSpacing ? './gsd-mot-spacing-annotations.tsv'
  : jiNominals ? './gsd-ji-nominal-annotations.tsv'
  : scopeNominals ? './gsd-scope-nominal-annotations.tsv'
  : derivedNominals ? './gsd-derived-nominal-annotations.tsv'
  : nominalModifiers ? './gsd-nominal-modifier-annotations.tsv'
  : contextAnnotations ? './gsd-context-annotations.tsv'
  : independent ? './gsd-independent-proofreading-annotations.tsv' : './gsd-sentence-edit-annotations.tsv');
if (shared) annotationEdits.push(...tsv('./gsd-shared-sentence-annotations.tsv'));
if (restored) annotationEdits.push(...tsv('./gsd-source-restoration-annotations.tsv'));
if (proofread) annotationEdits.push(...tsv('./gsd-proofreading-annotations.tsv'));
for (const [id, query, pos, form, lemma, xpos, goldForm = form] of annotationEdits) {
  if (annotations.has(id) || (query !== '-' && (!query || !pos || !form || !lemma || !xpos
      || !goldForm || !form.startsWith(goldForm)))) {
    throw new Error(`invalid annotation: ${id}`);
  }
  // A joined word can replace two original annotations with one searchable lexeme.
  annotations.set(id, query === '-' ? null : { goldForm, fields: { query, pos, form, lemma, xpos } });
}

const excludedSentences = new Set();
if (contract && !restored) {
  for (const [id, reason, contrast] of tsv('./gsd-nonstandard-sentences.tsv')) {
    if (!reason || !contrast || excludedSentences.has(id)) {
      throw new Error(`invalid sentence exclusion: ${id}`);
    }
    excludedSentences.add(id);
  }
}

function applyEdit(text, start, end, { source, replacement }) {
  const index = text.indexOf(source);
  if (text.includes(replacement) && (index < 0 || replacement.includes(source))) {
    return { text, start, end };
  }
  if (index < 0 || text.indexOf(source, index + source.length) >= 0) {
    throw new Error(`edit source must occur once: ${source} in ${text}`);
  }
  // Keep common characters outside the changed range so adjacent annotations retain their span.
  let left = 0;
  let right = 0;
  while (left < source.length && left < replacement.length && source[left] === replacement[left]) {
    left++;
  }
  while (right < source.length - left && right < replacement.length - left
      && source.at(-1 - right) === replacement.at(-1 - right)) {
    right++;
  }
  const editStart = Buffer.byteLength(text.slice(0, index + left));
  const editEnd = Buffer.byteLength(text.slice(0, index + source.length - right));
  const delta = Buffer.byteLength(replacement) - Buffer.byteLength(source);
  return {
    text: text.slice(0, index) + replacement + text.slice(index + source.length),
    start: start >= editEnd ? start + delta : Math.min(start, editStart),
    end: end <= editStart ? end : Math.max(end, editEnd) + delta,
  };
}

const usedSentences = new Set();
const alignedSentences = new Map();
const output = original.toString('utf8').trimEnd().split('\n').flatMap(line => {
  const row = JSON.parse(line);
  const sentenceId = row.id.split(':')[1];
  const edits = editsBySentence.get(sentenceId);
  const excluded = excludedSentences.has(sentenceId);
  if (!edits && !excluded && !annotations.has(row.id)) return line;
  if (edits) usedSentences.add(sentenceId);
  let span = { text: row.text, start: row.gold_byte_start, end: row.gold_byte_end };
  const originalSpan = Buffer.from(span.text).subarray(span.start, span.end);
  for (const edit of edits ?? []) span = applyEdit(span.text, span.start, span.end, edit);
  if ((shared || independent || contextAnnotations || nominalModifiers || motSpacing) && edits) {
    const aligned = alignedSentences.get(sentenceId);
    if (aligned !== undefined && aligned !== span.text) {
      throw new Error(`inconsistent shared sentence: ${sentenceId}`);
    }
    alignedSentences.set(sentenceId, span.text);
  }

  const candidate = Buffer.from(span.text).subarray(span.start, span.end);
  const annotation = annotations.get(row.id);
  if (annotations.has(row.id)) {
    annotations.delete(row.id);
    if (!annotation) return [];
    const form = Buffer.from(annotation.fields.form);
    const offset = Buffer.from(span.text).indexOf(form, span.start);
    if (offset < 0 || offset >= span.end) {
      throw new Error(`corrected form missing from aligned span: ${row.id} in ${candidate}`);
    }
    span.start = offset;
    span.end = span.start + Buffer.byteLength(annotation.goldForm);
    Object.assign(row, annotation.fields);
  } else if (!candidate.equals(originalSpan)) {
    throw new Error(`annotation required: ${row.id}: ${originalSpan} → ${candidate}`);
  }
  row.text = span.text;
  row.gold_byte_start = span.start;
  row.gold_byte_end = span.end;
  const gold = Buffer.from(span.text).subarray(span.start, span.end);
  if (gold.length !== span.end - span.start || !gold.length
      || !gold.equals(Buffer.from(gold.toString('utf8')))) {
    throw new Error(`invalid corrected UTF-8 span: ${row.id}`);
  }
  if (excluded) {
    row.contract_expected = null;
    row.contract_reason = 'nonstandard-input';
  }
  return JSON.stringify(row);
});
if (annotations.size || usedSentences.size !== editsBySentence.size) {
  throw new Error(`unused edits or annotations: ${[...annotations.keys()]}`);
}
process.stdout.write(output.join('\n') + '\n');
