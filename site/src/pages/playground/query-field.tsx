import { Field } from '@base-ui/react/field';
import { Input } from '@base-ui/react/input';
import { PreviewCard } from '@base-ui/react/preview-card';
import { useLayoutEffect, useRef } from 'react';

import { DocumentLocale } from '../../app/i18n';

import * as styles from './query-field.css';

interface QueryFieldProps {
  readonly locale: DocumentLocale;
  readonly onValueChange: (value: string) => void;
  readonly value: string;
}

const atomTagTooltipId = 'playground-atom-tags';

export function QueryField({
  locale,
  onValueChange,
  value,
}: QueryFieldProps): React.JSX.Element {
  const isKorean = locale === DocumentLocale.Korean;
  const atomTags = [
    ['n:', isKorean ? '명사' : 'noun'],
    ['pro:', isKorean ? '대명사' : 'pronoun'],
    ['num:', isKorean ? '수사' : 'numeral'],
    ['v:', isKorean ? '동사' : 'verb'],
    ['adj:', isKorean ? '형용사' : 'adjective'],
    ['det:', isKorean ? '관형사' : 'determiner'],
    ['adv:', isKorean ? '부사' : 'adverb'],
    ['j:', isKorean ? '조사' : 'particle'],
    ['intj:', isKorean ? '감탄사' : 'interjection'],
    ['lit:', 'literal'],
  ] as const;
  const tagHelpLabel = isKorean ? '지원하는 atom 태그' : 'Supported atom tags';
  const inputRef = useRef<HTMLInputElement>(null);
  const isComposingRef = useRef(false);
  const lastPublishedValueRef = useRef(value);

  useLayoutEffect(() => {
    const input = inputRef.current;

    if (input !== null && input.value !== value) {
      input.value = value;
    }

    lastPublishedValueRef.current = value;
  }, [value]);

  function publishValue(nextValue: string): void {
    if (nextValue === lastPublishedValueRef.current) {
      return;
    }

    lastPublishedValueRef.current = nextValue;
    onValueChange(nextValue);
  }

  return (
    <Field.Root className="field field-query" name="query">
      <div className={styles.labelRow}>
        <Field.Label data-glossary-skip="">Query</Field.Label>
        <PreviewCard.Root>
          <PreviewCard.Trigger
            aria-describedby={atomTagTooltipId}
            aria-label={tagHelpLabel}
            className={styles.tagTrigger}
            closeDelay={0}
            data-glossary-skip=""
            delay={0}
            render={<button aria-label={tagHelpLabel} type="button" />}
          >
            {isKorean ? 'Atom 태그' : 'Atom tags'}
          </PreviewCard.Trigger>
          <PreviewCard.Portal>
            <PreviewCard.Positioner
              className={styles.positioner}
              side="top"
              sideOffset={8}
            >
              <PreviewCard.Popup
                className={styles.tooltip}
                id={atomTagTooltipId}
                role="tooltip"
              >
                <strong>{tagHelpLabel}</strong>
                <dl className={styles.tagList}>
                  {atomTags.map(([tag, label]) => (
                    <div key={tag}>
                      <dt>
                        <code>{tag}</code>
                      </dt>
                      <dd>{label}</dd>
                    </div>
                  ))}
                </dl>
              </PreviewCard.Popup>
            </PreviewCard.Positioner>
          </PreviewCard.Portal>
        </PreviewCard.Root>
      </div>
      <Input
        ref={inputRef}
        className="text-control"
        autoComplete="off"
        defaultValue={value}
        onChange={(event) => {
          if (!isComposingRef.current) {
            publishValue(event.currentTarget.value);
          }
        }}
        onCompositionEnd={(event) => {
          isComposingRef.current = false;
          publishValue(event.currentTarget.value);
        }}
        onCompositionStart={() => {
          isComposingRef.current = true;
        }}
      />
      <Field.Description>
        {isKorean
          ? '공백으로 atom을 나눕니다. 태그가 없으면 품사를 자동 분석합니다.'
          : 'Spaces separate atoms. Without a tag, kfind infers the part of speech.'}
      </Field.Description>
    </Field.Root>
  );
}
