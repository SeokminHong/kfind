import { Select } from '@base-ui/react/select';

import * as styles from './select-field.css';

interface SelectOption<Value extends string> {
  readonly description?: string;
  readonly label: string;
  readonly value: Value;
}

interface SelectFieldProps<Value extends string> {
  readonly description?: string;
  readonly id: string;
  readonly label: string;
  readonly name: string;
  readonly onValueChange: (value: Value) => void;
  readonly options: ReadonlyArray<SelectOption<Value>>;
  readonly placeholder?: string;
  readonly value: Value | null;
}

export function SelectField<Value extends string>({
  description,
  id,
  label,
  name,
  onValueChange,
  options,
  placeholder,
  value,
}: SelectFieldProps<Value>): React.JSX.Element {
  const descriptionId =
    description === undefined ? undefined : `${id}-description`;

  return (
    <div className="field">
      <Select.Root
        id={id}
        items={[...options]}
        name={name}
        onValueChange={(nextValue) => {
          if (nextValue !== null) {
            onValueChange(nextValue);
          }
        }}
        value={value}
      >
        <Select.Label className={styles.label}>{label}</Select.Label>
        <Select.Trigger
          aria-describedby={descriptionId}
          className={styles.trigger}
        >
          <Select.Value placeholder={placeholder} />
          <Select.Icon className={styles.icon}>▾</Select.Icon>
        </Select.Trigger>
        <Select.Portal>
          <Select.Positioner
            alignItemWithTrigger={false}
            className={styles.positioner}
            sideOffset={4}
          >
            <Select.Popup className={styles.popup}>
              <Select.List className={styles.list}>
                {options.map((option) => (
                  <Select.Item
                    className={styles.item}
                    key={option.value}
                    value={option.value}
                  >
                    <Select.ItemText>
                      <span className={styles.itemText}>
                        <span>{option.label}</span>
                        {option.description === undefined ? null : (
                          <small>{option.description}</small>
                        )}
                      </span>
                    </Select.ItemText>
                  </Select.Item>
                ))}
              </Select.List>
            </Select.Popup>
          </Select.Positioner>
        </Select.Portal>
      </Select.Root>
      {description === undefined ? null : (
        <p className={styles.description} id={descriptionId}>
          {description}
        </p>
      )}
    </div>
  );
}
