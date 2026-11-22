import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, mkdtempSync, openSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const [input] = process.argv.slice(2);
if (process.argv.length !== 3) {
    throw new Error('usage: node replay-gsd-audit-positives.mjs GSD_RAW_POSITIVES.jsonl');
}
const sha256 = data => createHash('sha256').update(data).digest('hex');
const raw = readFileSync(input);
assert.equal(sha256(raw), 'eaafbae7757dbb62c9a0ca6b370dadf0f41b53c3e3d044f21b191c2aab8e3682',
    'unexpected raw GSD positives SHA-256');
const { sentences, annotations } = JSON.parse(readFileSync(new URL('./gsd-initial-review.json', import.meta.url), 'utf8'));
const initial = raw.toString('utf8').trimEnd().split('\n').map(line => {
    const row = JSON.parse(line);
    const sentence = sentences[row.id.split(':')[1]];
    const annotation = annotations[row.id];
    if (!sentence && !annotation) return line;
    return JSON.stringify({ ...row, text: sentence ?? row.text, ...annotation });
}).join('\n') + '\n';
assert.equal(sha256(initial), 'a6955363dbf837f16b5ec4ef6465eabf8e6611df43c591a21588aa638efde129',
    'unexpected initial GSD review SHA-256');

const stages = [
    ['positive-corrections', '--explicit-copula-noun-spans', '--fused-copula-noun-spans',
        '--honorific-noun-spans', '--compound-noun-spans', '--reviewed-spelling'],
    ['sentence-edits', '--review-contract', '--align-shared-sentences', '--restore-source-passages', '--proofread'],
    ['copula-span-review'],
    ['jeok-span-review'],
    ['nominal-suffix-review'],
    ['proper-name-review'],
    ['joined-word-review'],
    ['context-pos-review'],
    ['sentence-edits', '--independent-proofreading'],
    ['context-pos-review', '--pronouns'],
    ['context-pos-review', '--whole-nouns'],
    ['proper-name-review', '--person-names'],
    ['copula-span-review', '--after-name-review'],
    ['copula-span-review', '--pronouns'],
    ['sentence-edits', '--context-annotations'],
    ['proper-name-review', '--entity-names'],
    ['proper-name-review', '--lexical-units'],
    ['copula-span-review', '--numerals'],
    ['proper-name-review', '--numeral-compounds'],
    ['proper-name-review', '--compound-names'],
    ...[
        'nominal-modifiers', 'derived-nominals', 'scope-nominals', 'ji-nominals',
        'mot-spacing', 'ijen-context', 'quantifier-annotations', 'lexical-whole',
        'context-lexemes', 'receive-derivation', 'missegmented-lexemes',
        'proper-boundaries', 'nominal-derived-boundaries', 'compound-boundaries',
        'predicate-reanalysis', 'auxiliary-pos', 'derived-predicate-pos', 'nominal-hada',
        'derived-segmentation', 'colloquial-lemma', 'oda-spacing', 'linked-main-verb-spacing',
        'lexical-predicates', 'predicate-continuations', 'colloquial-pos',
    ].map(mode => ['sentence-edits', '--' + mode]),
    ['copula-span-review', '--after-colloquial-pos'],
    ...[
        'residual-context', 'proofreading-joins', 'spatial-spacing', 'anonymous-mo',
        'mayor-title', 'aligned-nominal-spans', 'help-company', 'derived-whole-nouns',
        'nominal-spelling', 'ellipsis-and-entity', 'trial-auxiliary-spacing',
        'saint-determiner', 'case-topic',
    ].map(mode => ['sentence-edits', '--' + mode]),
];
const folder = mkdtempSync(join(tmpdir(), 'kfind-gsd-audit-'));
try {
    const paths = [join(folder, 'left.jsonl'), join(folder, 'right.jsonl')];
    let index = 0;
    writeFileSync(paths[index], initial);
    for (const [stage, ...options] of stages) {
        const script = fileURLToPath(new URL(`./apply-gsd-${stage}.mjs`, import.meta.url));
        const nextIndex = 1 - index;
        const output = openSync(paths[nextIndex], 'w');
        try {
            const result = spawnSync(process.execPath, [script, paths[index], ...options],
                { stdio: ['ignore', output, 'inherit'] });
            if (result.error) throw result.error;
            assert.equal(result.status, 0, `GSD review failed: ${stage} ${options.join(' ')}`);
        } finally {
            closeSync(output);
        }
        index = nextIndex;
    }
    const corrected = readFileSync(paths[index]);
    assert.equal(sha256(corrected), 'cd3a8e532e67ba0748875311d10eba91c5fcb0d644e5c541bf3a4d22ac7c5a37',
        'unexpected corrected GSD positives SHA-256');
    process.stdout.write(corrected);
} finally {
    rmSync(folder, { recursive: true, force: true });
}
