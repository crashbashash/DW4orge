import { Modal } from './Modal';

/** A yes/no confirmation. Used before an action that would discard the draft. */
export function ConfirmDialog({
  title,
  message,
  confirmLabel,
  isOpen,
  onConfirm,
  onCancel,
}: {
  title: string;
  message: string;
  confirmLabel: string;
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Modal
      title={title}
      isOpen={isOpen}
      onClose={onCancel}
      footer={
        <>
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
          <button type="button" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </>
      }
    >
      <p>{message}</p>
    </Modal>
  );
}
