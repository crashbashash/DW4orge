import { useEditor } from './EditorProvider';
import { dirty } from './selectors';

export function TopBar() {
  const { state, open, openSample, canLoadSample, save, saveAs, setMode, toggleTheme } = useEditor();
  const isDirty = dirty(state);

  return (
    <header className="topbar">
      <span className="brand">DW4orge</span>
      <button type="button" onClick={() => void open()}>
        Open
      </button>
      {canLoadSample ? (
        <button type="button" onClick={() => void openSample()}>
          Load sample save
        </button>
      ) : null}
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
    </header>
  );
}
