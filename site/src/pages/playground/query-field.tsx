import { Field } from '@base-ui/react/field';
import { Input } from '@base-ui/react/input';
import { PreviewCard } from '@base-ui/react/preview-card';
import { useLayoutEffect, useRef } from 'react';

import * as styles from './query-field.css';

interface QueryFieldProps {
  readonly onValueChange: (value: string) => void;
  readonly value: string;
}

const atomTags = [
  ['n:', '명사'],
  ['pro:', '대명사'],
  ['num:', '수사'],
  ['v:', '동사'],
  ['adj:', '형용사'],
  ['det:', '관형사'],
  ['adv:', '부사'],
  ['j:', '조사'],
  ['intj:', '감탄사'],
  ['lit:', 'literal'],
] as const;
const atomTagTooltipId = 'playground-atom-tags';

export function QueryField({
  onValueChange,
  value,
}: QueryFieldProps): React.JSX.Element {
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
            aria-label="지원하는 atom 태그 보기"
            className={styles.tagTrigger}
            closeDelay={0}
            data-glossary-skip=""
            delay={0}
            render={
              <button aria-label="지원하는 atom 태그 보기" type="button" />
            }
          >
            Atom 태그
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
                <strong>지원 atom 태그</strong>
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
        공백으로 atom을 나눕니다. 태그를 생략하면 품사를 자동 분석합니다.
      </Field.Description>
    </Field.Root>
  );
}
