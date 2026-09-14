import { StatusBar } from '../components/StatusBar';
import { Sidebar } from '../components/Sidebar';
import { SummaryCard } from '../components/SummaryCard';
import { useEditor } from './EditorProvider';
import { TopBar } from './TopBar';
import { useShortcuts } from './useShortcuts';

export function AppShell() {
  const { state, open, save, saveAs, undo, redo } = useEditor();

  useShortcuts({
    open: () => void open(),
    save: () => void save(),
    saveAs: () => void saveAs(),
    undo,
    redo,
  });

  return (
    <div className="app">
      <TopBar />
      <div className="body">
        <Sidebar />
        <main className="main">
          {state.session ? (
            <>
              <SummaryCard />
              <SectionView />
            </>
          ) : (
            <div className="empty">
              <h1>DW4orge</h1>
              <p>Open a PS2 memory card or a raw save to begin.</p>
              <div className="empty-actions">
                <button type="button" onClick={() => void open()}>
                  Open save
                </button>
              </div>
            </div>
          )}
        </main>
      </div>
      <StatusBar />
    </div>
  );
}

/** The active section. Filled in by the section tasks; a placeholder until then. */
function SectionView() {
  const { state } = useEditor();
  return (
    <section className="card">
      <h2>{state.section}</h2>
      <p className="muted">This section is not built yet.</p>
    </section>
  );
}
