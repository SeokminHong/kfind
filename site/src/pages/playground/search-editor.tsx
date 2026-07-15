import type { Match } from '../../kfind-wasm';

import { Field } from '@base-ui/react/field';
import { useLayoutEffect, useMemo, useRef } from 'react';

import { mergeMatchSpans } from '../../playground';

import * as styles from './search-editor.css';

interface SearchEditorProps {
  readonly matches: readonly Match[];
  readonly onValueChange: (value: string) => void;
  readonly value: string;
}

export function SearchEditor({
  matches,
  onValueChange,
  value,
}: SearchEditorProps): React.JSX.Element {
  const editorRef = useRef<HTMLDivElement>(null);
  const lastEmittedValue = useRef<string | undefined>(undefined);
  const byteLength = useMemo(
    () => new TextEncoder().encode(value).byteLength,
    [value],
  );

  useLayoutEffect(() => {
    const editor = editorRef.current;

    if (editor === null || lastEmittedValue.current === value) {
      return;
    }

    editor.textContent = value;
    lastEmittedValue.current = value;
  }, [value]);

  function emitEditorValue(editor: HTMLDivElement): void {
    const nextValue = editor.innerText.replace(/\r\n/gu, '\n');
    lastEmittedValue.current = nextValue;
    onValueChange(nextValue);
  }

  return (
    <Field.Root className={styles.field} name="text">
      <div className={styles.labelRow}>
        <Field.Label data-glossary-skip="" id="text-editor-label">
          검색할 텍스트
        </Field.Label>
        <span>
          {value.length.toLocaleString('ko-KR')}자 ·{' '}
          {byteLength.toLocaleString('ko-KR')} bytes
        </span>
      </div>
      <div className={styles.surface}>
        <div aria-hidden="true" className={styles.highlights}>
          <HighlightedText matches={matches} text={value} />
        </div>
        <div
          ref={editorRef}
          aria-labelledby="text-editor-label"
          aria-multiline="true"
          className={styles.editor}
          contentEditable="plaintext-only"
          onInput={(event) => {
            emitEditorValue(event.currentTarget);
          }}
          role="textbox"
          spellCheck={false}
          suppressContentEditableWarning
        />
      </div>
      <Field.Description className={styles.description}>
        일치한 span은 입력 위치에 바로 표시됩니다.
      </Field.Description>
    </Field.Root>
  );
}

function HighlightedText({
  matches,
  text,
}: {
  readonly matches: readonly Match[];
  readonly text: string;
}): React.JSX.Element {
  const spans = mergeMatchSpans(matches, text.length);
  const fragments: React.ReactNode[] = [];
  let cursor = 0;

  for (const [index, span] of spans.entries()) {
    fragments.push(text.slice(cursor, span.start));
    fragments.push(
      <mark key={`${span.start}-${span.end}-${index}`}>
        {text.slice(span.start, span.end)}
      </mark>,
    );
    cursor = span.end;
  }

  fragments.push(text.slice(cursor));
  return <>{fragments}</>;
}
