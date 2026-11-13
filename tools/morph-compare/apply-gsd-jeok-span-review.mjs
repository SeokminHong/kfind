import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (!input || process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-jeok-span-review.mjs GSD_POSITIVES.jsonl');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'ad32efd10519f4c4100ab3107e3910074c68b28c67f48b14a15716a9da90d3ce') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.pos !== 'noun' || row.expected !== true
      || !/^NNG\+XSN(?:(?:\+J[A-Z]+)+|\+VCP(?:\+EP)*\+(?:EF|EC|ETM))?$/.test(row.xpos)
      || !row.lemma.startsWith(row.query + '+적')
      || row.lemma.split('+')[1] !== '적'
      || !row.form.startsWith(row.query + '적')
      || !Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end)
        .equals(Buffer.from(row.form))) {
    return line;
  }

  // 海賊 is an independent noun. Its 賊 is distinct from the derivational suffix 的.
  if (row.form === '해적이' && row.lemma === '해+적+이' && row.xpos === 'NNG+XSN+JKS') {
    return JSON.stringify({
      ...row, query: '해적', lemma: '해적+이', xpos: 'NNG+JKS',
    });
  }

  row.gold_byte_end = row.gold_byte_start + Buffer.byteLength(row.query);
  return JSON.stringify(row);
});
process.stdout.write(output.join('\n') + '\n');
