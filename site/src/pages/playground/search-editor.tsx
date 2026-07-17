import type { DecorationSet } from '@codemirror/view';

import type { Match } from '../../kfind-wasm';

import { Field } from '@base-ui/react/field';
import { history, historyKeymap } from '@codemirror/commands';
import {
  Annotation,
  EditorSelection,
  EditorState,
  StateEffect,
  StateField,
  Transaction,
} from '@codemirror/state';
import { Decoration, EditorView, keymap } from '@codemirror/view';
import {
  forwardRef,
  useCallback,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
} from 'react';

import { mergeMatchSpans } from '../../playground';

import * as styles from './search-editor.css';

interface SearchEditorProps {
  readonly matches: readonly Match[];
  readonly onValueChange: (value: string) => void;
  readonly value: string;
}

export interface SearchEditorHandle {
  readonly revealMatch: (match: Match) => void;
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

export const SearchEditor = forwardRef<SearchEditorHandle, SearchEditorProps>(
  ({ matches, onValueChange, value }, forwardedRef): React.JSX.Element => {
    const editorHostRef = useRef<HTMLDivElement>(null);
    const editorViewRef = useRef<EditorView>(null);
    const initialValueRef = useRef(value);
    const lastPublishedValueRef = useRef(value);
    const onValueChangeRef = useRef(onValueChange);
    const pendingHighlightsRef = useRef<readonly SearchHighlight[]>([]);
    const byteLength = useMemo(
      () => new TextEncoder().encode(value).byteLength,
      [value],
    );

    useImperativeHandle(forwardedRef, () => ({
      revealMatch(match) {
        const editorView = editorViewRef.current;

        if (editorView === null) {
          return;
        }

        const start = Math.max(
          0,
          Math.min(editorView.state.doc.length, match.start),
        );
        const end = Math.max(
          start,
          Math.min(editorView.state.doc.length, match.end),
        );
        const selection = EditorSelection.range(start, end);

        editorView.dispatch({
          effects: EditorView.scrollIntoView(selection, { y: 'center' }),
          selection: EditorSelection.create([selection]),
        });
        editorHostRef.current?.scrollIntoView({ block: 'center' });
        editorView.focus();
      },
    }));

    useLayoutEffect(() => {
      onValueChangeRef.current = onValueChange;
    }, [onValueChange]);

    const publishEditorValue = useCallback((editorView: EditorView): void => {
      const nextValue = editorView.state.doc.toString();

      if (nextValue === lastPublishedValueRef.current) {
        return;
      }

      lastPublishedValueRef.current = nextValue;
      onValueChangeRef.current(nextValue);
    }, []);

    const applySearchHighlights = useCallback(
      (editorView: EditorView): void => {
        if (editorView.composing) {
          return;
        }

        editorView.dispatch({
          effects: setSearchHighlights.of(pendingHighlightsRef.current),
        });
      },
      [],
    );

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
          EditorView.domEventHandlers({
            compositionend: (_event, editorView) => {
              globalThis.queueMicrotask(() => {
                publishEditorValue(editorView);
                applySearchHighlights(editorView);
              });
            },
          }),
          EditorView.updateListener.of((update) => {
            const isExternalUpdate = update.transactions.some(
              (transaction) =>
                transaction.annotation(externalValueUpdate) === true,
            );

            if (
              update.docChanged &&
              !isExternalUpdate &&
              !update.view.composing
            ) {
              publishEditorValue(update.view);
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
    }, [applySearchHighlights, publishEditorValue]);

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
      lastPublishedValueRef.current = value;
    }, [value]);

    useLayoutEffect(() => {
      const editorView = editorViewRef.current;

      if (editorView === null) {
        return;
      }

      pendingHighlightsRef.current = mergeMatchSpans(
        matches,
        editorView.state.doc.length,
      );
      applySearchHighlights(editorView);
    }, [applySearchHighlights, matches]);

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
  },
);

SearchEditor.displayName = 'SearchEditor';
