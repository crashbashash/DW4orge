import { useState } from 'react';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { NewSaveDialog } from '../features/new/NewSaveDialog';
import { useEditor } from './EditorProvider';
import { dirty } from './selectors';

export function TopBar() {
  const { state, open, openSample, canLoadSample, save, saveAs, setMode, toggleTheme } = useEditor();
  const [confirm, setConfirm] = useState<null | 'open' | 'new'>(null);
  const [newOpen, setNewOpen] = useState(false);
  const isDirty = dirty(state);

  // A dirty draft is only discarded after an explicit confirmation.
  const requestOpen = () => {
    if (isDirty) setConfirm('open');
    else void open();
  };
  const requestNew = () => {
    if (isDirty) setConfirm('new');
    else setNewOpen(true);
  };

  return (
    <header className="topbar">
      <span className="brand">DW4orge</span>
      <button type="button" onClick={requestOpen}>
        Open
      </button>
      {canLoadSample ? (
        <button type="button" onClick={() => void openSample()}>
          Load sample save
        </button>
      ) : null}
      <button type="button" onClick={requestNew} disabled={!state.appInfo}>
        New
      </button>
      <button type="button" onClick={() => void save()} disabled={!state.session}>
        Save
      </button>
      <button type="button" onClick={() => void saveAs()} disabled={!state.draft}>
        Save As
      </button>
      <span
        className={isDirty ? 'dirty' : 'dirty clean'}
        title={isDirty ? 'Unsaved changes' : 'No changes'}
        aria-label={isDirty ? 'Unsaved changes' : 'No changes'}
        role="status"
      />
      <span className="spacer" />
      <button
        type="button"
        onClick={() => setMode(state.mode === 'normal' ? 'advanced' : 'normal')}
      >
        Mode: {state.mode === 'normal' ? 'Normal' : 'Advanced'}
      </button>
      <button type="button" onClick={toggleTheme}>
        Theme: {state.theme === 'dark' ? 'Dark' : 'Light'}
      </button>

      <ConfirmDialog
        isOpen={confirm !== null}
        title="Discard unsaved changes?"
        message="This save has unsaved changes. Opening or creating another save will discard them."
        confirmLabel="Discard"
        onCancel={() => setConfirm(null)}
        onConfirm={() => {
          const what = confirm;
          setConfirm(null);
          if (what === 'open') void open();
          else if (what === 'new') setNewOpen(true);
        }}
      />
      <NewSaveDialog isOpen={newOpen} onClose={() => setNewOpen(false)} />
    </header>
  );
}
