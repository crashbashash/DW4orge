import { useEffect, useRef, useState } from 'react';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { useBackend } from '../ipc/context';
import { useEditor } from './EditorProvider';
import { dirty } from './selectors';

/**
 * Intercepts the shell's window-close request and asks before discarding
 * unsaved work.
 *
 * Rendered once, inside the editor and backend providers. A clean draft lets
 * the close through untouched; a dirty one is only closed after the user
 * confirms, matching the Open/New discard prompt.
 */
export function CloseGuard() {
  const backend = useBackend();
  const { state } = useEditor();
  const [askClose, setAskClose] = useState(false);

  // The close handler runs outside React's render cycle, so it reads the
  // latest dirty flag from a ref instead of a captured value.
  const isDirty = dirty(state);
  const latest = useRef(isDirty);
  useEffect(() => {
    latest.current = isDirty;
  });

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    void backend
      .onCloseRequested(() => {
        if (!latest.current) return false;
        setAskClose(true);
        return true;
      })
      .then((off) => {
        if (alive) unlisten = off;
        else off();
      });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [backend]);

  return (
    <ConfirmDialog
      isOpen={askClose}
      title="Discard unsaved changes?"
      message="This save has unsaved changes. Closing the window will discard them."
      confirmLabel="Discard and close"
      onCancel={() => setAskClose(false)}
      onConfirm={() => {
        setAskClose(false);
        // `closeWindow` bypasses the guard, so this cannot ask again.
        void backend.closeWindow();
      }}
    />
  );
}
