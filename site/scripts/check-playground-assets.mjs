import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const corpusUrl = new URL(
  '../public/playground/korean-wikipedia-20231101-ko-1mib.txt',
  import.meta.url,
);
const manifestUrl = new URL(
  '../public/playground/korean-wikipedia-20231101-ko-1mib.sources.json',
  import.meta.url,
);
const expectedCorpusPath =
  'site/public/playground/korean-wikipedia-20231101-ko-1mib.txt';

const [corpus, manifestSource] = await Promise.all([
  readFile(corpusUrl),
  readFile(manifestUrl, 'utf8'),
]);
const manifest = JSON.parse(manifestSource);
const output = manifest.output;

if (
  output?.path !== expectedCorpusPath ||
  !Number.isSafeInteger(output.utf8_bytes) ||
  !/^[0-9a-f]{64}$/u.test(output.sha256)
) {
  throw new Error(
    `invalid playground corpus manifest: ${fileURLToPath(manifestUrl)}`,
  );
}

if (corpus.byteLength !== output.utf8_bytes) {
  throw new Error(
    `playground corpus size mismatch: expected ${output.utf8_bytes}, got ${corpus.byteLength}`,
  );
}

const sha256 = createHash('sha256').update(corpus).digest('hex');

if (sha256 !== output.sha256) {
  throw new Error(
    `playground corpus checksum mismatch: expected ${output.sha256}, got ${sha256}`,
  );
}

process.stdout.write(
  `Verified playground corpus (${corpus.byteLength} bytes, ${sha256}).\n`,
);
