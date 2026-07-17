import type { ComponentProps, HTMLAttributes } from 'react';

import { Dialog } from '@base-ui/react/dialog';

import * as styles from './modal.css';

type ModalProps = ComponentProps<typeof Dialog.Root>;
type ModalTriggerProps = Omit<
  ComponentProps<typeof Dialog.Trigger>,
  'className'
>;
type ModalCloseProps = Omit<ComponentProps<typeof Dialog.Close>, 'className'>;
type ModalTitleProps = Omit<ComponentProps<typeof Dialog.Title>, 'className'>;
type ModalDescriptionProps = Omit<
  ComponentProps<typeof Dialog.Description>,
  'className'
>;

export function Modal(props: ModalProps): React.JSX.Element {
  return <Dialog.Root {...props} />;
}

function ModalTrigger(props: ModalTriggerProps): React.JSX.Element {
  return <Dialog.Trigger {...props} className={styles.trigger} />;
}

function ModalContent({
  children,
}: Readonly<{ children: React.ReactNode }>): React.JSX.Element {
  return (
    <Dialog.Portal>
      <Dialog.Backdrop className={styles.backdrop} />
      <Dialog.Viewport className={styles.viewport}>
        <Dialog.Popup className={styles.content}>{children}</Dialog.Popup>
      </Dialog.Viewport>
    </Dialog.Portal>
  );
}

function ModalSection({
  children,
  ...props
}: HTMLAttributes<HTMLDivElement>): React.JSX.Element {
  return (
    <div {...props} className={styles.section}>
      {children}
    </div>
  );
}

function ModalTitle(props: ModalTitleProps): React.JSX.Element {
  return <Dialog.Title {...props} className={styles.title} />;
}

function ModalDescription(props: ModalDescriptionProps): React.JSX.Element {
  return <Dialog.Description {...props} className={styles.description} />;
}

function ModalClose(props: ModalCloseProps): React.JSX.Element {
  return <Dialog.Close {...props} className={styles.close} />;
}

Modal.Trigger = ModalTrigger;
Modal.Content = ModalContent;
Modal.Section = ModalSection;
Modal.Title = ModalTitle;
Modal.Description = ModalDescription;
Modal.Close = ModalClose;
