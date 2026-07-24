import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const DATASET = 'wikimedia/wikipedia';
const DATASET_REVISION = 'b04c8d1ceb2f5cd4588862100d08de323dccfbaa';
const CONFIG = '20231101.ko';
const SPLIT = 'train';
const TARGET_BYTE_LENGTH = 1024 * 1024;
const PAGE_LENGTH = 100;
const OUTPUT_URL = new URL(
  '../public/playground/korean-wikipedia-20231101-ko-1mib.txt',
  import.meta.url,
);
const MANIFEST_URL = new URL(
  '../public/playground/korean-wikipedia-20231101-ko-1mib.sources.json',
  import.meta.url,
);

const outputPath = fileURLToPath(OUTPUT_URL);
const manifestPath = fileURLToPath(MANIFEST_URL);

await verifyDatasetRevision();

const chunks = [];
const sources = [];
let byteLength = 0;
let paddingBytes = 0;

const rowPages = await Promise.all(
  [0, PAGE_LENGTH].map((rowOffset) => fetchRows(rowOffset)),
);
const rows = rowPages.flat();

for (const { row, row_idx: rowIndex } of rows) {
  const article = Buffer.from(formatArticle(row));
  const remainingBytes = TARGET_BYTE_LENGTH - byteLength;
  const complete = article.byteLength <= remainingBytes;
  const content = complete ? article : truncateUtf8(article, remainingBytes);

  chunks.push(content);
  byteLength += content.byteLength;

  sources.push({
    row_index: rowIndex,
    id: row.id,
    title: row.title,
    url: row.url,
    complete,
    included_bytes: content.byteLength,
  });

  if (!complete) {
    paddingBytes = TARGET_BYTE_LENGTH - byteLength;
    chunks.push(Buffer.alloc(paddingBytes, 0x2e));
    byteLength += paddingBytes;
    break;
  }
}

if (byteLength !== TARGET_BYTE_LENGTH) {
  throw new Error(`200행 안에서 1 MiB를 채우지 못했습니다: ${byteLength}`);
}

const corpus = Buffer.concat(chunks);
const sha256 = createHash('sha256').update(corpus).digest('hex');
const manifest = {
  schema_version: 1,
  dataset: DATASET,
  dataset_revision: DATASET_REVISION,
  dataset_url: `https://huggingface.co/datasets/${DATASET}`,
  config: CONFIG,
  split: SPLIT,
  license: 'CC BY-SA 3.0',
  license_url: 'https://creativecommons.org/licenses/by-sa/3.0/',
  extraction: {
    order: 'Dataset Viewer row index ascending from 0',
    article_format: String.raw`<title>\n<url>\n\n<text>\n\n`,
    line_trailing_whitespace_removed: true,
    target_utf8_bytes: TARGET_BYTE_LENGTH,
    final_article_truncated_at_utf8_boundary: sources.some(
      (source) => !source.complete,
    ),
    padding_byte: 'ASCII FULL STOP',
    padding_bytes: paddingBytes,
  },
  output: {
    path: 'site/public/playground/korean-wikipedia-20231101-ko-1mib.txt',
    utf8_bytes: corpus.byteLength,
    sha256,
  },
  sources,
};

await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, corpus);
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

process.stdout.write(
  `Generated ${corpus.byteLength.toLocaleString('en')} bytes from ${sources.length} articles (${sha256}).\n`,
);

async function verifyDatasetRevision() {
  const response = await fetch(
    `https://huggingface.co/api/datasets/${DATASET}`,
  );

  if (!response.ok) {
    throw new Error(
      `데이터셋 revision 확인 실패: ${response.status} ${response.statusText}`,
    );
  }

  const metadata = await response.json();

  if (metadata.sha !== DATASET_REVISION) {
    throw new Error(
      `데이터셋 revision이 변경되었습니다: ${metadata.sha ?? 'unknown'}`,
    );
  }
}

async function fetchRows(rowOffset) {
  const query = new URLSearchParams({
    dataset: DATASET,
    config: CONFIG,
    split: SPLIT,
    revision: DATASET_REVISION,
    offset: String(rowOffset),
    length: String(PAGE_LENGTH),
  });
  const response = await fetch(
    `https://datasets-server.huggingface.co/rows?${query}`,
  );

  if (!response.ok) {
    throw new Error(
      `데이터셋 ${rowOffset}행 로드 실패: ${response.status} ${response.statusText}`,
    );
  }

  const payload = await response.json();
  return payload.rows;
}

function formatArticle(row) {
  const text = row.text.trim().replace(/[\t ]+$/gmu, '');
  return `${row.title}\n${row.url}\n\n${text}\n\n`;
}

function truncateUtf8(buffer, maximumByteLength) {
  const decoder = new TextDecoder('utf-8', { fatal: true });

  for (let end = maximumByteLength; end >= maximumByteLength - 3; end -= 1) {
    try {
      return Buffer.from(decoder.decode(buffer.subarray(0, end)));
    } catch {
      continue;
    }
  }

  throw new Error('UTF-8 경계에서 마지막 문서를 자를 수 없습니다.');
}
