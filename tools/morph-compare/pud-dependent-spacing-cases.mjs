import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [corpusPath, negativesPath] = process.argv.slice(2);
if (process.argv.length !== 4) {
    throw Error('usage: node pud-dependent-spacing-cases.mjs PUD.conllu NEGATIVES.jsonl');
}
const corpus = readFileSync(corpusPath, 'utf8');
const negatives = readFileSync(negativesPath, 'utf8');
const digest = text => createHash('sha256').update(text).digest('hex');
assert.equal(digest(corpus), '55f9c8d465400e070664c5319091d31ce35a16ec220b0bebc5e3c9b12f530948');
assert.equal(digest(negatives), '482e55d417516e8db503dbd421725fcc7affc6c6afcf5b5d42cec3bbd4d35d54');

const targets = new Set(['수', '중', '것']);
const withoutSpace = text => text.replace(/\s/gu, '');
const sentences = new Map();
for (const block of corpus.trimEnd().split(/\n\n+/u)) {
    const lines = block.split('\n');
    const field = name => lines.find(line => line.startsWith(`# ${name} = `))?.slice(name.length + 5);
    const id = field('sent_id');
    const text = field('text');
    const original = field('ToDoOrigText');
    if (!original || original === text || withoutSpace(original) !== withoutSpace(text)) continue;

    let cursor = 0;
    const tokens = [];
    for (const line of lines.filter(line => /^\d+\t/u.test(line))) {
        const [id, form, , pos, tag, , head, relation, , misc] = line.split('\t');
        while (/\s/u.test(text[cursor] ?? '')) cursor++;
        assert.ok(text.startsWith(form, cursor), `${id}: ${form}`);
        tokens.push({ id, form, pos, tag, head, relation, misc, start: cursor, end: cursor + form.length });
        cursor += form.length;
    }
    const originalCharacters = [...original.matchAll(/\S/gu)];
    const matches = [];
    const edits = new Map();
    for (const [index, token] of tokens.entries()) {
        if (!targets.has(token.form) || token.pos !== 'NOUN' || token.tag !== 'NNB') continue;
        const previous = tokens[index - 1];
        if (previous?.end !== token.start || !previous.misc.split('|').includes('SpaceAfter=No')) continue;
        const sourceOffset = [...withoutSpace(text.slice(0, token.start))].length;
        if (!/\s/u.test(original[originalCharacters[sourceOffset].index - 1] ?? '')) continue;
        edits.set(token.start, { end: token.start, replacement: ' ' });

        const next = tokens[index + 1];
        // A restrictive particle attaches to its dependent noun under spelling rules 41 and 42.
        const hasRestrictiveParticle = token.form === '수' && next?.form === '밖에'
            && next.pos === 'PART' && next.relation === 'case' && next.head === token.id;
        if (hasRestrictiveParticle && token.end < next.start) {
            edits.set(token.end, { end: next.start, replacement: '' });
        }
        matches.push({ token, start: previous.start, end: hasRestrictiveParticle ? next.end : token.end });
    }
    if (matches.length === 0) continue;

    let corrected = '';
    cursor = 0;
    const orderedEdits = [...edits].sort(([left], [right]) => left - right);
    for (const [start, { end, replacement }] of orderedEdits) {
        assert.ok(start >= cursor, id);
        corrected += text.slice(cursor, start) + replacement;
        cursor = end;
    }
    corrected += text.slice(cursor);
    const offset = (at, afterInsertion) => orderedEdits.reduce((position, [start, edit]) =>
        start < at || (afterInsertion && start === at && start === edit.end)
            ? position + edit.replacement.length - (edit.end - start) : position, at);
    sentences.set(id, { text, corrected, matches, offset });
}

for (const line of negatives.trimEnd().split('\n')) {
    const row = JSON.parse(line);
    if (!row.id.startsWith('pud:') || row.pos !== 'noun' || !targets.has(row.query)) continue;
    const sentence = sentences.get(row.id.split(':')[1]);
    if (!sentence) continue;
    assert.equal(row.text, sentence.text, row.id);
    assert.equal(row.expected, false, row.id);
    const { text, corrected, matches, offset } = sentence;
    for (const { token, start, end } of matches.filter(({ token }) => token.form === row.query)) {
        const goldStart = offset(token.start, true);
        const goldEnd = offset(end, false);
        console.log(JSON.stringify({
            id: `${row.id}/${token.id}`, source_id: row.id, query: row.query, pos: row.pos,
            text: corrected, expected: true,
            gold_byte_start: Buffer.byteLength(corrected.slice(0, goldStart)),
            gold_byte_end: Buffer.byteLength(corrected.slice(0, goldEnd)),
            source_text: text, source_surface: text.slice(start, end),
            contrast_surface: corrected.slice(offset(start, false), goldEnd),
            source_start: Buffer.byteLength(text.slice(0, token.start)),
            source_end: Buffer.byteLength(text.slice(0, token.end)),
            source_token: token.id,
        }));
    }
}
