import { useEditor } from '../app/EditorProvider';
import { describeSource } from '../lib/format';
import { errorCount } from '../app/selectors';

export function StatusBar() {
  const { state } = useEditor();
  const errors = errorCount(state);
  const path = state.session?.path ?? null;

  return (
    <footer className="statusbar">
      <span className="status-path" title={path ?? undefined}>
        {path ?? 'untitled'}
        {state.session ? ` (${describeSource(state.session.source)})` : ''}
      </span>
      <span className="status-write">{state.lastWrite?.message ?? ''}</span>
      <span
        className={state.error ? 'status-errors danger' : 'status-errors'}
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {state.error ??
          (errors > 0 ? `${errors} error(s): ${state.validation.errors[0]?.message ?? ''}` : '')}
      </span>
    </footer>
  );
}
