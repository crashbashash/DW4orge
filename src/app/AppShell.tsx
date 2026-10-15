import { useState } from 'react';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { StatusBar } from '../components/StatusBar';
import { Sidebar } from '../components/Sidebar';
import { SummaryCard } from '../components/SummaryCard';
import { CharacterSection } from '../features/character/CharacterSection';
import { BankSection } from '../features/bank/BankSection';
import { DisksSection } from '../features/disks/DisksSection';
import { EquipmentSection } from '../features/equipment/EquipmentSection';
import { ItemsSection } from '../features/items/ItemsSection';
import { StorySection } from '../features/story/StorySection';
import { CloseGuard } from './CloseGuard';
import { useEditor } from './EditorProvider';
import { TopBar } from './TopBar';
import { useShortcuts } from './useShortcuts';
import { dirty } from './selectors';

export function AppShell() {
  const { state, open, openSample, save, saveAs, undo, redo, clearError } = useEditor();
  const [pendingOpen, setPendingOpen] = useState<'open' | 'sample' | null>(null);
  const requestOpen = (kind: 'open' | 'sample' = 'open') => {
    if (state.status === 'busy') return;
    if (dirty(state)) setPendingOpen(kind);
    else void (kind === 'open' ? open() : openSample());
  };

  useShortcuts({
    open: () => requestOpen(),
    save: () => { if (state.status !== 'busy') void save(); },
    saveAs: () => { if (state.status !== 'busy') void saveAs(); },
    undo,
    redo,
  });

  return (
    <div className="app">
      <TopBar onOpen={() => requestOpen()} onSample={() => requestOpen('sample')} />
      <ConfirmDialog
        isOpen={pendingOpen !== null}
        title="Discard unsaved changes?"
        message="This save has unsaved changes. Opening another save will discard them."
        confirmLabel="Discard"
        onCancel={() => setPendingOpen(null)}
        onConfirm={() => {
          const kind = pendingOpen;
          setPendingOpen(null);
          if (kind) void (kind === 'open' ? open() : openSample());
        }}
      />
      <CloseGuard />
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
              <p>Open a PS2 or GameCube memory card, Xbox save or memory unit, GCI save, or raw save to begin.</p>
              <div className="empty-actions">
                <button type="button" onClick={() => requestOpen()}>
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
