import type { DecorationSet } from '@codemirror/view';

import type { Match } from '../../kfind-wasm';

import { Field } from '@base-ui/react/field';
import { history, historyKeymap } from '@codemirror/commands';
import {
  Annotation,
  EditorState,
  StateEffect,
  StateField,
  Transaction,
} from '@codemirror/state';
import { Decoration, EditorView, keymap } from '@codemirror/view';
import { useLayoutEffect, useMemo, useRef } from 'react';

import { mergeMatchSpans } from '../../playground';

import * as styles from './search-editor.css';

interface SearchEditorProps {
  readonly matches: readonly Match[];
  readonly onValueChange: (value: string) => void;
  readonly value: string;
}

interface SearchHighlight {
  readonly end: number;
  readonly start: number;
}

const externalValueUpdate = Annotation.define<boolean>();
const setSearchHighlights = StateEffect.define<readonly SearchHighlight[]>();
const searchHighlightMark = Decoration.mark({ class: 'cm-kfind-match' });

const searchHighlightField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  provide: (field) => EditorView.decorations.from(field),
  update: (decorations, transaction) => {
    for (const effect of transaction.effects) {
      if (effect.is(setSearchHighlights)) {
        return Decoration.set(
          effect.value.map(({ start, end }) =>
            searchHighlightMark.range(start, end),
          ),
          true,
        );
      }
    }

    return transaction.docChanged ? Decoration.none : decorations;
  },
});

export function SearchEditor({
  matches,
  onValueChange,
  value,
}: SearchEditorProps): React.JSX.Element {
  const editorHostRef = useRef<HTMLDivElement>(null);
  const editorViewRef = useRef<EditorView>(null);
  const initialValueRef = useRef(value);
  const onValueChangeRef = useRef(onValueChange);
  const byteLength = useMemo(
    () => new TextEncoder().encode(value).byteLength,
    [value],
  );

  useLayoutEffect(() => {
    onValueChangeRef.current = onValueChange;
  }, [onValueChange]);

  useLayoutEffect(() => {
    const editorHost = editorHostRef.current;

    if (editorHost === null) {
      return;
    }

    const editorView = new EditorView({
      doc: initialValueRef.current,
      extensions: [
        EditorState.tabSize.of(2),
        EditorView.contentAttributes.of({
          'aria-describedby': 'text-editor-description',
          'aria-labelledby': 'text-editor-label',
          'aria-multiline': 'true',
          autocapitalize: 'off',
          autocomplete: 'off',
          spellcheck: 'false',
        }),
        EditorView.lineWrapping,
        history(),
        keymap.of(historyKeymap),
        searchHighlightField,
        EditorView.updateListener.of((update) => {
          const isExternalUpdate = update.transactions.some(
            (transaction) =>
              transaction.annotation(externalValueUpdate) === true,
          );

          if (update.docChanged && !isExternalUpdate) {
            onValueChangeRef.current(update.state.doc.toString());
          }
        }),
      ],
      parent: editorHost,
    });

    editorViewRef.current = editorView;

    return () => {
      editorViewRef.current = null;
      editorView.destroy();
    };
  }, []);

  useLayoutEffect(() => {
    const editorView = editorViewRef.current;

    if (editorView === null || editorView.state.doc.toString() === value) {
      return;
    }

    editorView.dispatch({
      annotations: [
        externalValueUpdate.of(true),
        Transaction.addToHistory.of(false),
      ],
      changes: {
        from: 0,
        insert: value,
        to: editorView.state.doc.length,
      },
    });
  }, [value]);

  useLayoutEffect(() => {
    const editorView = editorViewRef.current;

    if (editorView === null) {
      return;
    }

    editorView.dispatch({
      effects: setSearchHighlights.of(
        mergeMatchSpans(matches, editorView.state.doc.length),
      ),
    });
  }, [matches]);

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
      <div ref={editorHostRef} className={styles.editor} />
      <Field.Description
        className={styles.description}
        id="text-editor-description"
      >
        일치한 span은 입력 위치에 바로 표시됩니다.
      </Field.Description>
    </Field.Root>
  );
}
