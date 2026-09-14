import type { ReactNode } from 'react';
import { Dialog, Heading, Modal as AriaModal, ModalOverlay } from 'react-aria-components';

/** A controlled modal dialog with a title, body and action row. */
export function Modal({
  title,
  isOpen,
  onClose,
  children,
  footer,
}: {
  title: string;
  isOpen: boolean;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <ModalOverlay
      className="modal-overlay"
      isOpen={isOpen}
      isDismissable
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <AriaModal className="modal">
        <Dialog className="modal-dialog">
          {({ close }) => (
            <>
              <Heading slot="title">{title}</Heading>
              <div className="modal-body">{children}</div>
              <div className="modal-actions">
                {footer ?? (
                  <button type="button" onClick={close}>
                    Close
                  </button>
                )}
              </div>
            </>
          )}
        </Dialog>
      </AriaModal>
    </ModalOverlay>
  );
}
