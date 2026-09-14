import { StatusBar } from '../components/StatusBar';
import { Sidebar } from '../components/Sidebar';
import { SummaryCard } from '../components/SummaryCard';
import { CharacterSection } from '../features/character/CharacterSection';
import { BankSection } from '../features/bank/BankSection';
import { DisksSection } from '../features/disks/DisksSection';
import { EquipmentSection } from '../features/equipment/EquipmentSection';
import { ItemsSection } from '../features/items/ItemsSection';
import { StorySection } from '../features/story/StorySection';
import { useEditor } from './EditorProvider';
import { TopBar } from './TopBar';
import { useShortcuts } from './useShortcuts';

export function AppShell() {
  const { state, open, save, saveAs, undo, redo, clearError } = useEditor();

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
          {state.error ? (
            <div className="banner danger" role="alert">
              <span>{state.error}</span>
              <button type="button" onClick={clearError}>
                Dismiss
              </button>
            </div>
          ) : null}
          {state.session ? (
            <>
              <SummaryCard />
              <SectionView />
            </>
          ) : (
            <div className="empty">
              <h2>DW4orge</h2>
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

/** The active section. The remaining sections land in their own tasks. */
function SectionView() {
  const { state } = useEditor();
  if (state.section === 'character') return <CharacterSection />;
  if (state.section === 'items') return <ItemsSection />;
  if (state.section === 'equipment') return <EquipmentSection />;
  if (state.section === 'disks') return <DisksSection />;
  if (state.section === 'story') return <StorySection />;
  if (state.section === 'bank') return <BankSection />;
  return (
    <section className="card">
      <h2>{state.section}</h2>
      <p className="muted">This section is not built yet.</p>
    </section>
  );
}
